use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::Local;
use rusqlite::{params, Connection, OptionalExtension};

use crate::models::{Classification, DailySummary, EvidenceDay, EvidenceSample, EvidenceStats, SampleRecord, SessionDetail, Settings, StudySession};

pub struct Db {
  conn: Connection,
  pub data_dir: PathBuf,
}

impl Db {
  pub fn open(data_dir: PathBuf) -> Result<Self> {
    std::fs::create_dir_all(&data_dir)?;
    std::fs::create_dir_all(data_dir.join("screenshots"))?;
    let conn = Connection::open(data_dir.join("study_guard.sqlite3"))?;
    let db = Self { conn, data_dir };
    db.migrate()?;
    Ok(db)
  }

  fn migrate(&self) -> Result<()> {
    self.conn.execute_batch(
      r#"
      CREATE TABLE IF NOT EXISTS study_sessions (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        task TEXT NOT NULL,
        status TEXT NOT NULL,
        started_at TEXT NOT NULL,
        ended_at TEXT,
        paused_at TEXT,
        paused_seconds INTEGER NOT NULL DEFAULT 0
      );

      CREATE TABLE IF NOT EXISTS samples (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        session_id INTEGER NOT NULL,
        captured_at TEXT NOT NULL,
        app_name TEXT NOT NULL,
        window_title TEXT NOT NULL,
        classification TEXT NOT NULL,
        confidence REAL NOT NULL,
        reason TEXT NOT NULL,
        topic TEXT NOT NULL,
        screenshot_path TEXT,
        manual_classification TEXT,
        corrected_at TEXT,
        ai_error TEXT,
        FOREIGN KEY(session_id) REFERENCES study_sessions(id)
      );

      CREATE TABLE IF NOT EXISTS daily_reports (
        date TEXT PRIMARY KEY,
        evaluation TEXT NOT NULL,
        generated_at TEXT NOT NULL
      );

      CREATE TABLE IF NOT EXISTS settings (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL
      );
      "#,
    )?;
    self.ensure_column("study_sessions", "paused_at", "TEXT")?;
    self.ensure_column("study_sessions", "paused_seconds", "INTEGER NOT NULL DEFAULT 0")?;
    self.ensure_column("samples", "manual_classification", "TEXT")?;
    self.ensure_column("samples", "corrected_at", "TEXT")?;
    self.ensure_column("samples", "ai_error", "TEXT")?;

    if self.get_settings_raw()?.is_none() {
      self.save_settings(&Settings::default())?;
    }
    Ok(())
  }

  fn ensure_column(&self, table: &str, column: &str, definition: &str) -> Result<()> {
    let mut rows = self.conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let columns = rows.query_map([], |row| row.get::<_, String>(1))?;
    for name in columns {
      if name? == column {
        return Ok(());
      }
    }
    self.conn.execute(&format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"), [])?;
    Ok(())
  }

  pub fn create_session(&self, task: &str) -> Result<StudySession> {
    let now = Local::now().to_rfc3339();
    self.conn.execute(
      "INSERT INTO study_sessions (task, status, started_at) VALUES (?1, 'running', ?2)",
      params![task, now],
    )?;
    let id = self.conn.last_insert_rowid();
    Ok(StudySession {
      id,
      task: task.to_string(),
      status: "running".to_string(),
      started_at: now,
      ended_at: None,
      paused_at: None,
      paused_seconds: 0,
    })
  }

  pub fn pause_active_session(&self) -> Result<Option<StudySession>> {
    let session = self.active_session()?;
    if let Some(session) = session {
      if session.status != "running" {
        return self.session_by_id(session.id);
      }
      let now = Local::now().to_rfc3339();
      self.conn.execute(
        "UPDATE study_sessions SET status = 'paused', paused_at = ?1 WHERE id = ?2",
        params![now, session.id],
      )?;
      return self.session_by_id(session.id);
    }
    Ok(None)
  }

  pub fn resume_active_session(&self) -> Result<Option<StudySession>> {
    let session = self.active_session()?;
    if let Some(session) = session {
      if session.status != "paused" {
        return self.session_by_id(session.id);
      }
      let paused_seconds = session.paused_seconds + current_pause_seconds(&session)?;
      self.conn.execute(
        "UPDATE study_sessions SET status = 'running', paused_at = NULL, paused_seconds = ?1 WHERE id = ?2",
        params![paused_seconds, session.id],
      )?;
      return self.session_by_id(session.id);
    }
    Ok(None)
  }

  pub fn end_active_session(&self) -> Result<Option<StudySession>> {
    let session = self.active_session()?;
    if let Some(session) = session {
      let now = Local::now().to_rfc3339();
      let paused_seconds = session.paused_seconds + current_pause_seconds(&session)?;
      self.conn.execute(
        "UPDATE study_sessions SET status = 'ended', ended_at = ?1, paused_at = NULL, paused_seconds = ?2 WHERE id = ?3",
        params![now, paused_seconds, session.id],
      )?;
      return self.session_by_id(session.id);
    }
    Ok(None)
  }

  pub fn active_session(&self) -> Result<Option<StudySession>> {
    self.conn
      .query_row(
        "SELECT id, task, status, started_at, ended_at, paused_at, paused_seconds FROM study_sessions WHERE status IN ('running', 'paused') ORDER BY id DESC LIMIT 1",
        [],
        row_to_session,
      )
      .optional()
      .context("failed to load active session")
  }

  pub fn session_by_id(&self, id: i64) -> Result<Option<StudySession>> {
    self.conn
      .query_row(
        "SELECT id, task, status, started_at, ended_at, paused_at, paused_seconds FROM study_sessions WHERE id = ?1",
        params![id],
        row_to_session,
      )
      .optional()
      .context("failed to load session")
  }

  pub fn insert_sample(&self, sample: &SampleRecord) -> Result<()> {
    self.conn.execute(
      "INSERT INTO samples (session_id, captured_at, app_name, window_title, classification, confidence, reason, topic, screenshot_path, ai_error)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
      params![
        sample.session_id,
        sample.captured_at,
        sample.app_name,
        sample.window_title,
        sample.classification.as_str(),
        sample.confidence,
        sample.reason,
        sample.topic,
        sample.screenshot_path,
        sample.ai_error
      ],
    )?;
    Ok(())
  }

  pub fn get_settings(&self) -> Result<Settings> {
    Ok(self.get_settings_raw()?.unwrap_or_default())
  }

  fn get_settings_raw(&self) -> Result<Option<Settings>> {
    let value: Option<String> = self
      .conn
      .query_row("SELECT value FROM settings WHERE key = 'app'", [], |row| row.get(0))
      .optional()?;
    match value {
      Some(value) => Ok(Some(serde_json::from_str(&value)?)),
      None => Ok(None),
    }
  }

  pub fn save_settings(&self, settings: &Settings) -> Result<()> {
    let value = serde_json::to_string(settings)?;
    self.conn.execute(
      "INSERT INTO settings (key, value) VALUES ('app', ?1)
       ON CONFLICT(key) DO UPDATE SET value = excluded.value",
      params![value],
    )?;
    Ok(())
  }

  pub fn today_summary(&self) -> Result<DailySummary> {
    let settings = self.get_settings()?;
    let date = Local::now().format("%Y-%m-%d").to_string();
    let active_session = self.active_session()?;

    let mut total_seconds = 0_u64;
    let mut rows = self.conn.prepare(
      "SELECT id, task, status, started_at, ended_at, paused_at, paused_seconds FROM study_sessions WHERE substr(started_at, 1, 10) = ?1",
    )?;
    let sessions = rows.query_map(params![date], row_to_session)?;
    for session in sessions {
      let session = session?;
      let start = chrono::DateTime::parse_from_rfc3339(&session.started_at)?;
      let end = match &session.ended_at {
        Some(value) => chrono::DateTime::parse_from_rfc3339(value)?,
        None => Local::now().fixed_offset(),
      };
      let elapsed = (end - start).num_seconds().max(0) as u64;
      let paused = session.paused_seconds + current_pause_seconds(&session)?;
      total_seconds += elapsed.saturating_sub(paused);
    }

    let mut counts: HashMap<String, u64> = HashMap::new();
    let mut topic_counts: HashMap<String, u64> = HashMap::new();
    let mut previous = "unknown".to_string();
    let mut distraction_count = 0_u64;
    let mut last_classification = Classification::Unknown;

    let mut sample_rows = self.conn.prepare(
      "SELECT COALESCE(manual_classification, classification), topic FROM samples WHERE substr(captured_at, 1, 10) = ?1 ORDER BY captured_at ASC",
    )?;
    let samples = sample_rows.query_map(params![date], |row| {
      Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    for sample in samples {
      let (classification, topic) = sample?;
      *counts.entry(classification.clone()).or_default() += 1;
      if !topic.trim().is_empty() {
        *topic_counts.entry(topic).or_default() += 1;
      }
      if classification == "distracted" && previous != "distracted" {
        distraction_count += 1;
      }
      previous = classification.clone();
      last_classification = Classification::from(classification);
    }

    let seconds_for = |key: &str| counts.get(key).copied().unwrap_or_default() * settings.sample_interval_seconds;
    let focused_seconds = seconds_for("focused");
    let distracted_seconds = seconds_for("distracted");
    let idle_seconds = seconds_for("idle");

    let mut top_topics = topic_counts.into_iter().collect::<Vec<_>>();
    top_topics.sort_by(|a, b| b.1.cmp(&a.1));
    let top_topics = top_topics.into_iter().take(5).map(|item| item.0).collect::<Vec<_>>();

    let evaluation = self.build_evaluation(total_seconds, focused_seconds, distracted_seconds, distraction_count, &top_topics);

    Ok(DailySummary {
      date,
      total_seconds,
      focused_seconds,
      distracted_seconds,
      idle_seconds,
      distraction_count,
      top_topics,
      evaluation,
      last_classification,
      active_session,
    })
  }

  pub fn cleanup_screenshots(&self, retention_days: u64) -> Result<()> {
    let dir = self.data_dir.join("screenshots");
    if !dir.exists() {
      return Ok(());
    }
    let cutoff = std::time::SystemTime::now()
      .checked_sub(std::time::Duration::from_secs(retention_days * 24 * 3600))
      .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
    for entry in std::fs::read_dir(dir)? {
      let entry = entry?;
      let metadata = entry.metadata()?;
      if metadata.modified().unwrap_or(std::time::SystemTime::now()) < cutoff {
        let _ = std::fs::remove_file(entry.path());
      }
    }
    Ok(())
  }

  pub fn evidence_day(&self, date: Option<String>) -> Result<EvidenceDay> {
    let date = date.unwrap_or_else(|| Local::now().format("%Y-%m-%d").to_string());
    let mut session_rows = self.conn.prepare(
      "SELECT id, task, status, started_at, ended_at, paused_at, paused_seconds FROM study_sessions WHERE substr(started_at, 1, 10) = ?1 ORDER BY started_at DESC",
    )?;
    let sessions = session_rows
      .query_map(params![date], row_to_session)?
      .collect::<std::result::Result<Vec<_>, _>>()?;
    let samples = self.samples_for_date(&date)?;
    let stats = evidence_stats(&samples);
    Ok(EvidenceDay { date, sessions, samples, stats })
  }

  pub fn session_detail(&self, session_id: i64) -> Result<Option<SessionDetail>> {
    let Some(session) = self.session_by_id(session_id)? else {
      return Ok(None);
    };
    let samples = self.samples_for_session(session_id)?;
    let stats = evidence_stats(&samples);
    Ok(Some(SessionDetail { session, samples, stats }))
  }

  pub fn correct_sample(&self, sample_id: i64, classification: Classification) -> Result<Option<EvidenceSample>> {
    let now = Local::now().to_rfc3339();
    self.conn.execute(
      "UPDATE samples SET manual_classification = ?1, corrected_at = ?2 WHERE id = ?3",
      params![classification.as_str(), now, sample_id],
    )?;
    self.sample_by_id(sample_id)
  }

  fn samples_for_date(&self, date: &str) -> Result<Vec<EvidenceSample>> {
    let mut rows = self.conn.prepare(EVIDENCE_SAMPLE_SELECT_DATE)?;
    let samples = rows
      .query_map(params![date], row_to_evidence_sample)?
      .collect::<std::result::Result<Vec<_>, _>>()
      .map_err(anyhow::Error::from)?;
    Ok(samples)
  }

  fn samples_for_session(&self, session_id: i64) -> Result<Vec<EvidenceSample>> {
    let mut rows = self.conn.prepare(EVIDENCE_SAMPLE_SELECT_SESSION)?;
    let samples = rows
      .query_map(params![session_id], row_to_evidence_sample)?
      .collect::<std::result::Result<Vec<_>, _>>()
      .map_err(anyhow::Error::from)?;
    Ok(samples)
  }

  fn sample_by_id(&self, sample_id: i64) -> Result<Option<EvidenceSample>> {
    self.conn
      .query_row(EVIDENCE_SAMPLE_SELECT_ID, params![sample_id], row_to_evidence_sample)
      .optional()
      .map_err(Into::into)
  }

  fn build_evaluation(&self, total: u64, focused: u64, distracted: u64, distractions: u64, topics: &[String]) -> String {
    if total == 0 {
      return "今天还没有学习记录。".to_string();
    }
    let ratio = if total > 0 { focused * 100 / total } else { 0 };
    let topic_text = if topics.is_empty() { "学习任务".to_string() } else { topics.join("、") };
    if ratio >= 75 {
      format!("今天状态不错，主要投入在{}。有效专注占比约{}%，分心{}次，继续保持这个节奏。", topic_text, ratio, distractions)
    } else if distracted > focused {
      format!("今天分心时间偏多，主要任务是{}。建议下一次缩短单次学习目标，并把容易切走的应用提前关掉。", topic_text)
    } else {
      format!("今天完成了{}的学习记录，专注占比约{}%。节奏已经建立，可以继续优化连续专注时间。", topic_text, ratio)
    }
  }
}

const EVIDENCE_SAMPLE_SELECT_DATE: &str = "
  SELECT id, session_id, captured_at, app_name, window_title, classification,
    COALESCE(manual_classification, classification) AS effective_classification,
    manual_classification, corrected_at, confidence, reason, topic, screenshot_path, ai_error
  FROM samples
  WHERE substr(captured_at, 1, 10) = ?1
  ORDER BY captured_at DESC
";

const EVIDENCE_SAMPLE_SELECT_SESSION: &str = "
  SELECT id, session_id, captured_at, app_name, window_title, classification,
    COALESCE(manual_classification, classification) AS effective_classification,
    manual_classification, corrected_at, confidence, reason, topic, screenshot_path, ai_error
  FROM samples
  WHERE session_id = ?1
  ORDER BY captured_at DESC
";

const EVIDENCE_SAMPLE_SELECT_ID: &str = "
  SELECT id, session_id, captured_at, app_name, window_title, classification,
    COALESCE(manual_classification, classification) AS effective_classification,
    manual_classification, corrected_at, confidence, reason, topic, screenshot_path, ai_error
  FROM samples
  WHERE id = ?1
";

fn evidence_stats(samples: &[EvidenceSample]) -> EvidenceStats {
  let mut stats = EvidenceStats {
    total_samples: samples.len() as u64,
    focused_samples: 0,
    distracted_samples: 0,
    idle_samples: 0,
    unknown_samples: 0,
    corrected_count: 0,
    screenshot_count: 0,
  };
  for sample in samples {
    match sample.effective_classification {
      Classification::Focused => stats.focused_samples += 1,
      Classification::Distracted => stats.distracted_samples += 1,
      Classification::Idle => stats.idle_samples += 1,
      Classification::Unknown => stats.unknown_samples += 1,
    }
    if sample.manual_classification.is_some() {
      stats.corrected_count += 1;
    }
    if sample.screenshot_exists {
      stats.screenshot_count += 1;
    }
  }
  stats
}

fn row_to_session(row: &rusqlite::Row<'_>) -> rusqlite::Result<StudySession> {
  Ok(StudySession {
    id: row.get(0)?,
    task: row.get(1)?,
    status: row.get(2)?,
    started_at: row.get(3)?,
    ended_at: row.get(4)?,
    paused_at: row.get(5)?,
    paused_seconds: row.get(6)?,
  })
}

fn row_to_evidence_sample(row: &rusqlite::Row<'_>) -> rusqlite::Result<EvidenceSample> {
  let screenshot_path: Option<String> = row.get(12)?;
  let screenshot_exists = screenshot_path.as_ref().is_some_and(|path| std::path::Path::new(path).exists());
  let manual_classification: Option<String> = row.get(7)?;
  Ok(EvidenceSample {
    id: row.get(0)?,
    session_id: row.get(1)?,
    captured_at: row.get(2)?,
    app_name: row.get(3)?,
    window_title: row.get(4)?,
    classification: Classification::from_str(row.get::<_, String>(5)?.as_str()),
    effective_classification: Classification::from_str(row.get::<_, String>(6)?.as_str()),
    manual_classification: manual_classification.map(|value| Classification::from_str(&value)),
    corrected_at: row.get(8)?,
    confidence: row.get(9)?,
    reason: row.get(10)?,
    topic: row.get(11)?,
    screenshot_path,
    screenshot_exists,
    ai_error: row.get(13)?,
  })
}

fn current_pause_seconds(session: &StudySession) -> Result<u64> {
  if session.status != "paused" {
    return Ok(0);
  }
  let Some(paused_at) = &session.paused_at else {
    return Ok(0);
  };
  let paused_at = chrono::DateTime::parse_from_rfc3339(paused_at)?;
  Ok((Local::now().fixed_offset() - paused_at).num_seconds().max(0) as u64)
}
