use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
  pub sample_interval_seconds: u64,
  pub distraction_threshold_seconds: u64,
  pub screenshot_retention_days: u64,
  pub ai_enabled: bool,
  pub privacy_mode: PrivacyMode,
  pub openai_api_key: String,
  pub openai_model: String,
}

impl Default for Settings {
  fn default() -> Self {
    Self {
      sample_interval_seconds: 20,
      distraction_threshold_seconds: 30,
      screenshot_retention_days: 7,
      ai_enabled: false,
      privacy_mode: PrivacyMode::LocalFirst,
      openai_api_key: String::new(),
      openai_model: "gpt-4.1-mini".to_string(),
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyMode {
  LocalFirst,
  CloudEnhanced,
  LocalOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Classification {
  Focused,
  Distracted,
  Idle,
  Unknown,
}

impl Classification {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::Focused => "focused",
      Self::Distracted => "distracted",
      Self::Idle => "idle",
      Self::Unknown => "unknown",
    }
  }
}

impl From<String> for Classification {
  fn from(value: String) -> Self {
    match value.as_str() {
      "focused" => Self::Focused,
      "distracted" => Self::Distracted,
      "idle" => Self::Idle,
      _ => Self::Unknown,
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudySession {
  pub id: i64,
  pub task: String,
  pub status: String,
  pub started_at: String,
  pub ended_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SampleRecord {
  pub id: i64,
  pub session_id: i64,
  pub captured_at: String,
  pub app_name: String,
  pub window_title: String,
  pub classification: Classification,
  pub confidence: f32,
  pub reason: String,
  pub topic: String,
  pub screenshot_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailySummary {
  pub date: String,
  pub total_seconds: u64,
  pub focused_seconds: u64,
  pub distracted_seconds: u64,
  pub idle_seconds: u64,
  pub distraction_count: u64,
  pub top_topics: Vec<String>,
  pub evaluation: String,
  pub last_classification: Classification,
  pub active_session: Option<StudySession>,
}

#[derive(Debug, Clone)]
pub struct WindowInfo {
  pub app_name: String,
  pub window_title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiDecision {
  pub classification: Classification,
  pub confidence: f32,
  pub reason: String,
  pub topic: String,
}
