import React from "react";
import ReactDOM from "react-dom/client";
import { Clock, Pause, Play, Square, BarChart3, Settings, ShieldCheck, Bell, KeyRound, RefreshCw } from "lucide-react";
import "./styles.css";
import { backend } from "./tauri";

type Classification = "focused" | "distracted" | "idle" | "unknown";

type StudySession = {
  id: number;
  task: string;
  status: "running" | "paused" | "ended";
  started_at: string;
  ended_at?: string | null;
};

type SettingsState = {
  sample_interval_seconds: number;
  distraction_threshold_seconds: number;
  screenshot_retention_days: number;
  ai_enabled: boolean;
  privacy_mode: "local_first" | "cloud_enhanced" | "local_only";
  openai_api_key: string;
  openai_model: string;
};

type DailySummary = {
  date: string;
  total_seconds: number;
  focused_seconds: number;
  distracted_seconds: number;
  idle_seconds: number;
  distraction_count: number;
  top_topics: string[];
  evaluation: string;
  last_classification: Classification;
  active_session?: StudySession | null;
};

const defaultSettings: SettingsState = {
  sample_interval_seconds: 20,
  distraction_threshold_seconds: 30,
  screenshot_retention_days: 7,
  ai_enabled: false,
  privacy_mode: "local_first",
  openai_api_key: "",
  openai_model: "gpt-4.1-mini",
};

function formatDuration(totalSeconds: number) {
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  if (hours > 0) return `${hours}小时 ${minutes}分钟`;
  if (minutes > 0) return `${minutes}分钟 ${seconds}秒`;
  return `${seconds}秒`;
}

function clsLabel(value: Classification) {
  const labels: Record<Classification, string> = {
    focused: "专注",
    distracted: "分心",
    idle: "空闲",
    unknown: "观察中",
  };
  return labels[value];
}

function App() {
  const [tab, setTab] = React.useState<"today" | "report" | "settings">("today");
  const [task, setTask] = React.useState("英语阅读");
  const [summary, setSummary] = React.useState<DailySummary | null>(null);
  const [settings, setSettings] = React.useState<SettingsState>(defaultSettings);
  const [message, setMessage] = React.useState("");
  const [isBusy, setIsBusy] = React.useState(false);

  const refresh = React.useCallback(async () => {
    const [nextSummary, nextSettings] = await Promise.all([backend.getTodaySummary(), backend.getSettings()]);
    setSummary(nextSummary);
    setSettings({ ...defaultSettings, ...nextSettings });
  }, []);

  React.useEffect(() => {
    refresh().catch((error) => setMessage(String(error)));
    const id = window.setInterval(() => refresh().catch(() => undefined), 5000);
    return () => window.clearInterval(id);
  }, [refresh]);

  async function runAction(action: () => Promise<unknown>, done: string) {
    setIsBusy(true);
    setMessage("");
    try {
      await action();
      await refresh();
      setMessage(done);
    } catch (error) {
      setMessage(String(error));
    } finally {
      setIsBusy(false);
    }
  }

  const active = summary?.active_session;
  const running = active?.status === "running";
  const paused = active?.status === "paused";
  const focusRatio = summary && summary.total_seconds > 0 ? Math.round((summary.focused_seconds / summary.total_seconds) * 100) : 0;

  return (
    <main className="shell">
      <aside className="sidebar">
        <div className="brand">
          <ShieldCheck size={24} />
          <div>
            <strong>Study Guard</strong>
            <span>本地优先自习监督</span>
          </div>
        </div>

        <nav className="nav">
          <button className={tab === "today" ? "active" : ""} onClick={() => setTab("today")}>
            <Clock size={18} />
            今日
          </button>
          <button className={tab === "report" ? "active" : ""} onClick={() => setTab("report")}>
            <BarChart3 size={18} />
            日报
          </button>
          <button className={tab === "settings" ? "active" : ""} onClick={() => setTab("settings")}>
            <Settings size={18} />
            设置
          </button>
        </nav>
      </aside>

      <section className="content">
        <header className="topbar">
          <div>
            <p>{summary?.date ?? "今天"}</p>
            <h1>{tab === "today" ? "今日自习" : tab === "report" ? "学习日报" : "偏好设置"}</h1>
          </div>
          <button className="iconButton" title="刷新" onClick={() => refresh()}>
            <RefreshCw size={18} />
          </button>
        </header>

        {tab === "today" && (
          <section className="workspace">
            <div className="heroPanel">
              <div className="taskRow">
                <label>
                  学习任务
                  <input value={task} onChange={(event) => setTask(event.target.value)} disabled={Boolean(active)} />
                </label>
                <div className={`status ${summary?.last_classification ?? "unknown"}`}>{clsLabel(summary?.last_classification ?? "unknown")}</div>
              </div>

              <div className="timer">{formatDuration(summary?.focused_seconds ?? 0)}</div>
              <div className="subtle">有效专注时间</div>

              <div className="actions">
                {!active && (
                  <button className="primary" disabled={isBusy || !task.trim()} onClick={() => runAction(() => backend.startSession(task.trim()), "已开始学习")}>
                    <Play size={18} />
                    开始
                  </button>
                )}
                {running && (
                  <button disabled={isBusy} onClick={() => runAction(() => backend.pauseSession(), "已暂停")}>
                    <Pause size={18} />
                    暂停
                  </button>
                )}
                {paused && (
                  <button disabled={isBusy} onClick={() => runAction(() => backend.resumeSession(), "已继续")}>
                    <Play size={18} />
                    继续
                  </button>
                )}
                {active && (
                  <button className="danger" disabled={isBusy} onClick={() => runAction(() => backend.endSession(), "已生成本次记录")}>
                    <Square size={18} />
                    结束
                  </button>
                )}
              </div>
            </div>

            <div className="metricGrid">
              <Metric label="今日总时长" value={formatDuration(summary?.total_seconds ?? 0)} />
              <Metric label="专注比例" value={`${focusRatio}%`} />
              <Metric label="分心时长" value={formatDuration(summary?.distracted_seconds ?? 0)} />
              <Metric label="分心次数" value={`${summary?.distraction_count ?? 0}次`} />
            </div>

            {message && <div className="toast">{message}</div>}
          </section>
        )}

        {tab === "report" && (
          <section className="report">
            <div className="metricGrid">
              <Metric label="学习总时长" value={formatDuration(summary?.total_seconds ?? 0)} />
              <Metric label="有效专注" value={formatDuration(summary?.focused_seconds ?? 0)} />
              <Metric label="空闲" value={formatDuration(summary?.idle_seconds ?? 0)} />
              <Metric label="分心" value={`${summary?.distraction_count ?? 0}次`} />
            </div>

            <div className="widePanel">
              <h2>学习内容</h2>
              <div className="tags">
                {(summary?.top_topics.length ? summary.top_topics : ["暂无记录"]).map((item) => (
                  <span key={item}>{item}</span>
                ))}
              </div>
            </div>

            <div className="widePanel">
              <h2>评价总结</h2>
              <p>{summary?.evaluation ?? "今天还没有学习记录。"}</p>
            </div>
          </section>
        )}

        {tab === "settings" && (
          <SettingsView
            settings={settings}
            setSettings={setSettings}
            save={() => runAction(() => backend.saveSettings(settings), "设置已保存")}
            busy={isBusy}
            message={message}
          />
        )}
      </section>
    </main>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="metric">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function SettingsView({
  settings,
  setSettings,
  save,
  busy,
  message,
}: {
  settings: SettingsState;
  setSettings: (settings: SettingsState) => void;
  save: () => void;
  busy: boolean;
  message: string;
}) {
  return (
    <section className="settingsGrid">
      <label className="field">
        采样间隔（秒）
        <input
          type="number"
          min={10}
          max={120}
          value={settings.sample_interval_seconds}
          onChange={(event) => setSettings({ ...settings, sample_interval_seconds: Number(event.target.value) })}
        />
      </label>
      <label className="field">
        分心阈值（秒）
        <input
          type="number"
          min={10}
          max={300}
          value={settings.distraction_threshold_seconds}
          onChange={(event) => setSettings({ ...settings, distraction_threshold_seconds: Number(event.target.value) })}
        />
      </label>
      <label className="field">
        截图保留（天）
        <input
          type="number"
          min={1}
          max={30}
          value={settings.screenshot_retention_days}
          onChange={(event) => setSettings({ ...settings, screenshot_retention_days: Number(event.target.value) })}
        />
      </label>
      <label className="field">
        隐私模式
        <select value={settings.privacy_mode} onChange={(event) => setSettings({ ...settings, privacy_mode: event.target.value as SettingsState["privacy_mode"] })}>
          <option value="local_first">本地优先</option>
          <option value="cloud_enhanced">云端增强</option>
          <option value="local_only">完全本地</option>
        </select>
      </label>

      <div className="switchRow">
        <div>
          <strong>AI 分析</strong>
          <span>仅在非完全本地模式下调用</span>
        </div>
        <button className={settings.ai_enabled ? "toggle on" : "toggle"} onClick={() => setSettings({ ...settings, ai_enabled: !settings.ai_enabled })}>
          <Bell size={16} />
          {settings.ai_enabled ? "开启" : "关闭"}
        </button>
      </div>

      <label className="field full">
        <span className="labelIcon">
          <KeyRound size={16} />
          OpenAI API Key
        </span>
        <input
          type="password"
          value={settings.openai_api_key}
          onChange={(event) => setSettings({ ...settings, openai_api_key: event.target.value })}
          placeholder="sk-..."
        />
      </label>
      <label className="field">
        AI 模型
        <input value={settings.openai_model} onChange={(event) => setSettings({ ...settings, openai_model: event.target.value })} />
      </label>

      <button className="primary save" disabled={busy} onClick={save}>
        保存设置
      </button>
      {message && <div className="toast full">{message}</div>}
    </section>
  );
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
