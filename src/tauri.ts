import { invoke } from "@tauri-apps/api/core";

type Classification = "focused" | "distracted" | "idle" | "unknown";

type StudySession = {
  id: number;
  task: string;
  status: "running" | "paused" | "ended";
  started_at: string;
  ended_at?: string | null;
};

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

export type EvidenceSample = {
  id: number;
  session_id: number;
  captured_at: string;
  app_name: string;
  window_title: string;
  classification: Classification;
  effective_classification: Classification;
  manual_classification?: Classification | null;
  corrected_at?: string | null;
  confidence: number;
  reason: string;
  topic: string;
  screenshot_path?: string | null;
  screenshot_exists: boolean;
};

export type EvidenceStats = {
  total_samples: number;
  focused_samples: number;
  distracted_samples: number;
  idle_samples: number;
  unknown_samples: number;
  corrected_count: number;
  screenshot_count: number;
};

export type EvidenceDay = {
  date: string;
  sessions: StudySession[];
  samples: EvidenceSample[];
  stats: EvidenceStats;
};

export type SessionDetail = {
  session: StudySession;
  samples: EvidenceSample[];
  stats: EvidenceStats;
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

const fallbackEvidence: EvidenceDay = {
  date: new Date().toISOString().slice(0, 10),
  sessions: [],
  samples: [],
  stats: {
    total_samples: 0,
    focused_samples: 0,
    distracted_samples: 0,
    idle_samples: 0,
    unknown_samples: 0,
    corrected_count: 0,
    screenshot_count: 0,
  },
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
  getEvidenceDay: (date?: string) => call<EvidenceDay>("get_evidence_day", { date }, fallbackEvidence),
  getSessionDetail: (sessionId: number) => call<SessionDetail>("get_session_detail", { sessionId }),
  correctSample: (sampleId: number, classification: Classification) => call<EvidenceSample>("correct_sample", { sampleId, classification }),
  getScreenshotDataUrl: (path: string) => call<string | null>("get_screenshot_data_url", { path }, null),
  getSettings: () => call("get_settings", undefined, fallbackSettings),
  saveSettings: (settings: unknown) => call("save_settings", { settings }),
  startSession: (task: string) => call("start_session", { task }),
  pauseSession: () => call("pause_session"),
  resumeSession: () => call("resume_session"),
  endSession: () => call("end_session"),
};
