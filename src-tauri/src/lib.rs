mod ai;
mod db;
mod models;
mod monitor;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, Result};
use chrono::Local;
use db::Db;
use models::{DailySummary, Settings, StudySession};
use tauri::{AppHandle, Manager, State};

struct RuntimeState {
  session_id: Option<i64>,
  stop: bool,
}

struct AppState {
  db: Arc<Mutex<Db>>,
  runtime: Arc<Mutex<RuntimeState>>,
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
  state.db.lock().map_err(lock_error)?.get_settings().map_err(error_string)
}

#[tauri::command]
fn save_settings(settings: Settings, state: State<'_, AppState>) -> Result<(), String> {
  let db = state.db.lock().map_err(lock_error)?;
  db.save_settings(&settings).map_err(error_string)?;
  db.cleanup_screenshots(settings.screenshot_retention_days).map_err(error_string)?;
  Ok(())
}

#[tauri::command]
fn get_today_summary(state: State<'_, AppState>) -> Result<DailySummary, String> {
  state.db.lock().map_err(lock_error)?.today_summary().map_err(error_string)
}

#[tauri::command]
fn start_session(task: String, app: AppHandle, state: State<'_, AppState>) -> Result<StudySession, String> {
  let task = task.trim();
  if task.is_empty() {
    return Err("学习任务不能为空".to_string());
  }

  {
    let db = state.db.lock().map_err(lock_error)?;
    if db.active_session().map_err(error_string)?.is_some() {
      return Err("已有正在进行的学习会话".to_string());
    }
  }

  let session = state.db.lock().map_err(lock_error)?.create_session(task).map_err(error_string)?;
  {
    let mut runtime = state.runtime.lock().map_err(lock_error)?;
    runtime.session_id = Some(session.id);
    runtime.stop = false;
  }
  spawn_sampler(app, session.id, task.to_string());
  Ok(session)
}

#[tauri::command]
fn pause_session(state: State<'_, AppState>) -> Result<Option<StudySession>, String> {
  state.db.lock().map_err(lock_error)?.set_active_status("paused").map_err(error_string)
}

#[tauri::command]
fn resume_session(state: State<'_, AppState>) -> Result<Option<StudySession>, String> {
  state.db.lock().map_err(lock_error)?.set_active_status("running").map_err(error_string)
}

#[tauri::command]
fn end_session(state: State<'_, AppState>) -> Result<Option<StudySession>, String> {
  {
    let mut runtime = state.runtime.lock().map_err(lock_error)?;
    runtime.stop = true;
    runtime.session_id = None;
  }
  state.db.lock().map_err(lock_error)?.end_active_session().map_err(error_string)
}

fn spawn_sampler(app: AppHandle, session_id: i64, task: String) {
  thread::spawn(move || {
    loop {
      let Some(state) = app.try_state::<AppState>() else {
        return;
      };

      let (should_stop, interval) = {
        let runtime = match state.runtime.lock() {
          Ok(value) => value,
          Err(_) => return,
        };
        let settings = match state.db.lock().ok().and_then(|db| db.get_settings().ok()) {
          Some(settings) => settings,
          None => Settings::default(),
        };
        (
          runtime.stop || runtime.session_id != Some(session_id),
          settings.sample_interval_seconds.max(10),
        )
      };
      if should_stop {
        return;
      }

      if let Err(error) = sample_once(&state, session_id, &task) {
        eprintln!("sample failed: {error:#}");
      }

      thread::sleep(Duration::from_secs(interval));
    }
  });
}

fn sample_once(state: &AppState, session_id: i64, task: &str) -> Result<()> {
  let (settings, status, screenshot_dir) = {
    let db = state.db.lock().map_err(|_| anyhow!("database lock poisoned"))?;
    let session = db.session_by_id(session_id)?.ok_or_else(|| anyhow!("session not found"))?;
    (db.get_settings()?, session.status, db.data_dir.join("screenshots"))
  };

  if status == "paused" || status == "ended" {
    return Ok(());
  }

  let window = monitor::capture_window_info();
  let local = monitor::classify_locally(task, &window);
  let mut screenshot_data_url = None;
  let mut screenshot_path = None;
  if settings.privacy_mode != models::PrivacyMode::LocalOnly {
    if let Ok(screenshot) = monitor::capture_screenshot(&screenshot_dir, session_id) {
      screenshot_path = Some(screenshot.path.to_string_lossy().to_string());
      screenshot_data_url = Some(screenshot.data_url);
    }
  }

  let needs_ai = matches!(local.classification, models::Classification::Unknown) || local.confidence < 0.7 || settings.privacy_mode == models::PrivacyMode::CloudEnhanced;
  let ai_decision = if needs_ai {
    let analyzer = ai::AiAnalyzer::new();
    let data_url = screenshot_data_url.as_deref();
    tauri::async_runtime::block_on(analyzer.analyze(task, &settings, &window, data_url)).ok().flatten()
  } else {
    None
  };
  let decision = ai_decision.unwrap_or(local);
  let sample = models::SampleRecord {
    id: 0,
    session_id,
    captured_at: Local::now().to_rfc3339(),
    app_name: window.app_name,
    window_title: window.window_title,
    classification: decision.classification,
    confidence: decision.confidence,
    reason: decision.reason,
    topic: decision.topic,
    screenshot_path,
  };

  let db = state.db.lock().map_err(|_| anyhow!("database lock poisoned"))?;
  db.insert_sample(&sample)?;
  db.cleanup_screenshots(settings.screenshot_retention_days)?;
  Ok(())
}

pub fn run() {
  tauri::Builder::default()
    .setup(|app| {
      let data_dir = data_dir();
      if let Ok(old_dir) = app.path().app_data_dir() {
        migrate_existing_data(&old_dir, &data_dir)
          .map_err(|error| format!("failed to migrate old app data: {error:#}"))?;
      }
      let db = Db::open(data_dir).map_err(|error| format!("failed to open database: {error:#}"))?;
      app.manage(AppState {
        db: Arc::new(Mutex::new(db)),
        runtime: Arc::new(Mutex::new(RuntimeState {
          session_id: None,
          stop: false,
        })),
      });
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      get_settings,
      save_settings,
      get_today_summary,
      start_session,
      pause_session,
      resume_session,
      end_session
    ])
    .run(tauri::generate_context!())
    .expect("error while running Study Guard");
}

fn lock_error<T>(_: std::sync::PoisonError<T>) -> String {
  "内部状态锁定失败".to_string()
}

fn error_string(error: anyhow::Error) -> String {
  format!("{error:#}")
}

fn data_dir() -> PathBuf {
  std::env::var_os("STUDY_GUARD_DATA_DIR")
    .map(PathBuf::from)
    .unwrap_or_else(|| PathBuf::from(r"D:\StudyGuard\data"))
}

fn migrate_existing_data(old_dir: &Path, new_dir: &Path) -> Result<()> {
  if old_dir == new_dir || !old_dir.exists() || new_dir.join("study_guard.sqlite3").exists() {
    return Ok(());
  }

  copy_dir_all(old_dir, new_dir)
}

fn copy_dir_all(source: &Path, target: &Path) -> Result<()> {
  std::fs::create_dir_all(target)?;
  for entry in std::fs::read_dir(source)? {
    let entry = entry?;
    let source_path = entry.path();
    let target_path = target.join(entry.file_name());
    if source_path.is_dir() {
      copy_dir_all(&source_path, &target_path)?;
    } else if !target_path.exists() {
      std::fs::copy(&source_path, &target_path)?;
    }
  }
  Ok(())
}
