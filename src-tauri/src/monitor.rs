use std::io::Cursor;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use base64::Engine;
use chrono::Local;
use image::{DynamicImage, ImageFormat};

use crate::models::{AiDecision, Classification, WindowInfo};

pub fn capture_window_info() -> WindowInfo {
  platform_window_info()
}

pub fn classify_locally(task: &str, window: &WindowInfo) -> AiDecision {
  let text = format!("{} {} {}", task, window.app_name, window.window_title).to_lowercase();
  let title = window.window_title.to_lowercase();
  let task_tokens = task
    .split(|ch: char| ch.is_whitespace() || ch == '-' || ch == '_' || ch == '/')
    .filter(|part| part.chars().count() >= 2)
    .map(|part| part.to_lowercase())
    .collect::<Vec<_>>();
  let task_hints = ["英语", "阅读", "数学", "物理", "化学", "语文", "论文", "课程", "作业", "背单词", "编程", "代码", "复习"];

  let distracted_keywords = [
    "douyin", "抖音", "tiktok", "steam", "game", "游戏", "netflix", "微博", "小红书", "reddit", "twitter", "x.com",
    "instagram", "shorts", "直播", "购物", "taobao", "淘宝",
  ];
  if distracted_keywords.iter().any(|word| text.contains(word)) {
    return AiDecision {
      classification: Classification::Distracted,
      confidence: 0.88,
      reason: "命中娱乐或社交关键词".to_string(),
      topic: "分心内容".to_string(),
    };
  }

  if task_tokens.iter().any(|token| title.contains(token)) || task_hints.iter().any(|hint| task.contains(hint) && text.contains(hint)) {
    return AiDecision {
      classification: Classification::Focused,
      confidence: 0.84,
      reason: "窗口标题与学习任务匹配".to_string(),
      topic: task.to_string(),
    };
  }

  let focused_apps = [
    "code", "cursor", "pycharm", "intellij", "clion", "webstorm", "word", "excel", "powerpoint", "onenote", "notion",
    "obsidian", "acrobat", "pdf", "edge", "chrome", "firefox",
  ];
  let focused_keywords = ["course", "课程", "学习", "lecture", "docs", "文档", "题", "作业", "paper", "论文", "anki", "leetcode"];
  if focused_apps.iter().any(|word| text.contains(word)) || focused_keywords.iter().any(|word| text.contains(word)) {
    return AiDecision {
      classification: Classification::Focused,
      confidence: 0.68,
      reason: "命中常见学习应用或学习关键词".to_string(),
      topic: task.to_string(),
    };
  }

  AiDecision {
    classification: Classification::Unknown,
    confidence: 0.4,
    reason: "本地规则无法稳定判断".to_string(),
    topic: task.to_string(),
  }
}

pub fn capture_screenshot(dir: &Path, session_id: i64) -> Result<CapturedScreenshot> {
  std::fs::create_dir_all(dir)?;
  let screens = screenshots::Screen::all().context("failed to enumerate screens")?;
  let screen = screens.first().context("no screen found")?;
  let image = screen.capture().context("failed to capture screen")?;
  let dynamic = DynamicImage::ImageRgba8(image);
  let resized = dynamic.resize(1280, 720, image::imageops::FilterType::Triangle);
  let mut bytes = Vec::new();
  resized.write_to(&mut Cursor::new(&mut bytes), ImageFormat::Jpeg)?;
  let filename = format!("session-{}-{}.jpg", session_id, Local::now().timestamp_millis());
  let path = dir.join(filename);
  std::fs::write(&path, &bytes)?;
  let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
  Ok(CapturedScreenshot {
    path,
    data_url: format!("data:image/jpeg;base64,{}", encoded),
  })
}

pub struct CapturedScreenshot {
  pub path: PathBuf,
  pub data_url: String,
}

#[cfg(windows)]
fn platform_window_info() -> WindowInfo {
  use windows::core::PWSTR;
  use windows::Win32::Foundation::{CloseHandle, MAX_PATH};
  use windows::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION};
  use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId};

  unsafe {
    let hwnd = GetForegroundWindow();
    if hwnd.0 == std::ptr::null_mut() {
      return WindowInfo {
        app_name: "unknown".to_string(),
        window_title: String::new(),
      };
    }

    let title_len = GetWindowTextLengthW(hwnd);
    let mut title_buf = vec![0_u16; title_len as usize + 1];
    let copied = GetWindowTextW(hwnd, &mut title_buf);
    let window_title = String::from_utf16_lossy(&title_buf[..copied as usize]);

    let mut pid = 0_u32;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    let mut app_name = "unknown".to_string();
    if pid > 0 {
      if let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
        let mut path_buf = vec![0_u16; MAX_PATH as usize];
        let mut size = path_buf.len() as u32;
        if QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, PWSTR(path_buf.as_mut_ptr()), &mut size).is_ok() {
          let path = String::from_utf16_lossy(&path_buf[..size as usize]);
          app_name = std::path::Path::new(&path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_string();
        }
        let _ = CloseHandle(handle);
      }
    }

    WindowInfo { app_name, window_title }
  }
}

#[cfg(not(windows))]
fn platform_window_info() -> WindowInfo {
  WindowInfo {
    app_name: "unsupported-platform".to_string(),
    window_title: "Window capture is only implemented for Windows MVP".to_string(),
  }
}
