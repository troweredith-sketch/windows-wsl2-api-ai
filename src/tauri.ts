import { invoke } from "@tauri-apps/api/core";

type Classification = "focused" | "distracted" | "idle" | "unknown";

type FallbackSummary = {
  date: string;
  total_seconds: number;
  focused_seconds: number;
  distracted_seconds: number;
  idle_seconds: number;
  distraction_count: number;
  top_topics: string[];
  evaluation: string;
  last_classification: Classification;
  active_session: null;
};

type FallbackSettings = {
  sample_interval_seconds: number;
  distraction_threshold_seconds: number;
  screenshot_retention_days: number;
  ai_enabled: boolean;
  privacy_mode: "local_first" | "cloud_enhanced" | "local_only";
  openai_api_key: string;
  openai_model: string;
};

const fallbackSummary: FallbackSummary = {
  date: new Date().toISOString().slice(0, 10),
  total_seconds: 0,
  focused_seconds: 0,
  distracted_seconds: 0,
  idle_seconds: 0,
  distraction_count: 0,
  top_topics: [],
  evaluation: "桌面后端未连接。使用 npm run tauri dev 启动后可采集窗口、截图并写入 SQLite。",
  last_classification: "unknown",
  active_session: null,
};

const fallbackSettings: FallbackSettings = {
  sample_interval_seconds: 20,
  distraction_threshold_seconds: 30,
  screenshot_retention_days: 7,
  ai_enabled: false,
  privacy_mode: "local_first",
  openai_api_key: "",
  openai_model: "gpt-4.1-mini",
};

async function call<T>(command: string, args?: Record<string, unknown>, fallback?: T): Promise<T> {
  if (!("__TAURI_INTERNALS__" in window)) {
    if (fallback !== undefined) return fallback;
    throw new Error("请使用 npm run tauri dev 启动桌面端");
  }
  return invoke<T>(command, args);
}

export const backend = {
  getTodaySummary: () => call("get_today_summary", undefined, fallbackSummary),
  getSettings: () => call("get_settings", undefined, fallbackSettings),
  saveSettings: (settings: unknown) => call("save_settings", { settings }),
  startSession: (task: string) => call("start_session", { task }),
  pauseSession: () => call("pause_session"),
  resumeSession: () => call("resume_session"),
  endSession: () => call("end_session"),
};
