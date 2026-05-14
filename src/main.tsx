import React from "react";
import ReactDOM from "react-dom/client";
import { BarChart3, Bell, CheckCircle2, Clock, Eye, ImageOff, KeyRound, Pause, Play, RefreshCw, Settings, ShieldCheck, Square, X } from "lucide-react";
import "./styles.css";
import { backend, type EvidenceDay, type EvidenceSample, type SessionDetail } from "./tauri";

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
  openai_api_base_url: string;
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

type Tab = "today" | "evidence" | "report" | "settings";
type ClassificationFilter = "all" | Classification;

const defaultSettings: SettingsState = {
  sample_interval_seconds: 20,
  distraction_threshold_seconds: 30,
  screenshot_retention_days: 7,
  ai_enabled: false,
  privacy_mode: "local_first",
  openai_api_key: "",
  openai_api_base_url: "https://api.openai.com/v1",
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

function formatTime(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value.slice(11, 19);
  return date.toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit", second: "2-digit", hour12: false });
}

function formatDateTime(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString("zh-CN", { hour12: false });
}

function App() {
  const [tab, setTab] = React.useState<Tab>("today");
  const [task, setTask] = React.useState("英语阅读");
  const [summary, setSummary] = React.useState<DailySummary | null>(null);
  const [evidence, setEvidence] = React.useState<EvidenceDay | null>(null);
  const [settings, setSettings] = React.useState<SettingsState>(defaultSettings);
  const [message, setMessage] = React.useState("");
  const [isBusy, setIsBusy] = React.useState(false);
  const [countdown, setCountdown] = React.useState<number | null>(null);

  const refresh = React.useCallback(async () => {
    const [nextSummary, nextSettings, nextEvidence] = await Promise.all([backend.getTodaySummary(), backend.getSettings(), backend.getEvidenceDay()]);
    setSummary(nextSummary);
    setSettings({ ...defaultSettings, ...nextSettings });
    setEvidence(nextEvidence);
  }, []);

  React.useEffect(() => {
    refresh().catch((error) => setMessage(String(error)));
    const id = window.setInterval(() => refresh().catch(() => undefined), 1000);
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

  async function startWithCountdown() {
    if (!task.trim() || isBusy || countdown !== null) return;
    setMessage("");
    for (const value of [3, 2, 1]) {
      setCountdown(value);
      await new Promise((resolve) => window.setTimeout(resolve, 700));
    }
    setCountdown(null);
    await runAction(() => backend.startSession(task.trim()), "已开始学习");
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
          <button className={tab === "evidence" ? "active" : ""} onClick={() => setTab("evidence")}>
            <Eye size={18} />
            证据
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
            <h1>{tab === "today" ? "今日自习" : tab === "evidence" ? "证据时间线" : tab === "report" ? "学习日报" : "偏好设置"}</h1>
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
                  <button className="primary" disabled={isBusy || countdown !== null || !task.trim()} onClick={startWithCountdown}>
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

            <RecentEvidence samples={evidence?.samples.slice(0, 3) ?? []} openEvidence={() => setTab("evidence")} />

            {message && <div className="toast">{message}</div>}
          </section>
        )}

        {tab === "evidence" && (
          <EvidenceView
            evidence={evidence}
            refresh={refresh}
            setMessage={setMessage}
            message={message}
          />
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
      {countdown !== null && (
        <div className="countdownOverlay" aria-live="assertive">
          <div>
            <span>准备开始</span>
            <strong>{countdown}</strong>
          </div>
        </div>
      )}
    </main>
  );
}

function RecentEvidence({ samples, openEvidence }: { samples: EvidenceSample[]; openEvidence: () => void }) {
  return (
    <div className="widePanel compactPanel">
      <div className="panelHeader">
        <h2>最近判断</h2>
        <button className="ghostButton" onClick={openEvidence}>
          <Eye size={16} />
          查看全部
        </button>
      </div>
      {samples.length === 0 ? (
        <p className="muted">开始学习后，这里会显示最近的窗口判断。</p>
      ) : (
        <div className="recentList">
          {samples.map((sample) => (
            <div className="recentItem" key={sample.id}>
              <span className={`status mini ${sample.effective_classification}`}>{clsLabel(sample.effective_classification)}</span>
              <div>
                <strong>{sample.app_name || "未知应用"}</strong>
                <span>{sample.window_title || "无窗口标题"}</span>
              </div>
              <time>{formatTime(sample.captured_at)}</time>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function EvidenceView({
  evidence,
  refresh,
  setMessage,
  message,
}: {
  evidence: EvidenceDay | null;
  refresh: () => Promise<void>;
  setMessage: (message: string) => void;
  message: string;
}) {
  const [classificationFilter, setClassificationFilter] = React.useState<ClassificationFilter>("all");
  const [sessionFilter, setSessionFilter] = React.useState("all");
  const [selected, setSelected] = React.useState<EvidenceSample | null>(null);
  const [sessionDetail, setSessionDetail] = React.useState<SessionDetail | null>(null);
  const [isCorrecting, setIsCorrecting] = React.useState(false);

  const samples = evidence?.samples ?? [];
  const filtered = samples.filter((sample) => {
    const classificationMatch = classificationFilter === "all" || sample.effective_classification === classificationFilter;
    const sessionMatch = sessionFilter === "all" || String(sample.session_id) === sessionFilter;
    return classificationMatch && sessionMatch;
  });

  React.useEffect(() => {
    const sessionId = sessionFilter === "all" ? null : Number(sessionFilter);
    if (!sessionId) {
      setSessionDetail(null);
      return;
    }
    backend.getSessionDetail(sessionId).then(setSessionDetail).catch(() => setSessionDetail(null));
  }, [sessionFilter]);

  async function correct(sample: EvidenceSample, classification: Classification) {
    setIsCorrecting(true);
    try {
      const updated = await backend.correctSample(sample.id, classification);
      setSelected(updated);
      await refresh();
      setMessage("已纠正这条记录");
    } catch (error) {
      setMessage(String(error));
    } finally {
      setIsCorrecting(false);
    }
  }

  const stats = sessionDetail?.stats ?? evidence?.stats;

  return (
    <section className="evidencePage">
      <div className="metricGrid evidenceMetrics">
        <Metric label="采样记录" value={`${stats?.total_samples ?? 0}条`} />
        <Metric label="人工纠正" value={`${stats?.corrected_count ?? 0}条`} />
        <Metric label="截图证据" value={`${stats?.screenshot_count ?? 0}张`} />
        <Metric label="当前筛选" value={`${filtered.length}条`} />
      </div>

      <div className="widePanel filtersPanel">
        <label className="field">
          分类
          <select value={classificationFilter} onChange={(event) => setClassificationFilter(event.target.value as ClassificationFilter)}>
            <option value="all">全部</option>
            <option value="focused">专注</option>
            <option value="distracted">分心</option>
            <option value="idle">空闲</option>
            <option value="unknown">观察中</option>
          </select>
        </label>
        <label className="field">
          会话
          <select value={sessionFilter} onChange={(event) => setSessionFilter(event.target.value)}>
            <option value="all">全部会话</option>
            {(evidence?.sessions ?? []).map((session) => (
              <option value={String(session.id)} key={session.id}>
                {session.task} · {formatTime(session.started_at)}
              </option>
            ))}
          </select>
        </label>
      </div>

      <div className="widePanel timelinePanel">
        <div className="panelHeader">
          <h2>{sessionDetail ? sessionDetail.session.task : `${evidence?.date ?? "今天"} 证据`}</h2>
          <span className="muted">{filtered.length} 条记录</span>
        </div>
        {filtered.length === 0 ? (
          <div className="emptyState">今天还没有采样记录。</div>
        ) : (
          <div className="timelineList">
            {filtered.map((sample) => (
              <button className="timelineItem" key={sample.id} onClick={() => setSelected(sample)}>
                <time>{formatTime(sample.captured_at)}</time>
                <span className={`status mini ${sample.effective_classification}`}>{clsLabel(sample.effective_classification)}</span>
                <div className="timelineMain">
                  <strong>{sample.app_name || "未知应用"}</strong>
                  <span>{sample.window_title || "无窗口标题"}</span>
                  <small>{sample.ai_error ? `AI 调用失败：${sample.ai_error}` : `${sample.reason} · ${sample.topic || "未标注内容"}`}</small>
                </div>
                <ScreenshotThumb sample={sample} />
                {sample.manual_classification && (
                  <span className="correctedMark">
                    <CheckCircle2 size={15} />
                    已纠正
                  </span>
                )}
              </button>
            ))}
          </div>
        )}
      </div>

      {selected && (
        <EvidenceDetail
          sample={selected}
          close={() => setSelected(null)}
          correct={correct}
          busy={isCorrecting}
        />
      )}
      {message && <div className="toast">{message}</div>}
    </section>
  );
}

function ScreenshotThumb({ sample }: { sample: EvidenceSample }) {
  const src = useScreenshotSrc(sample);
  if (src) return <img className="thumb" src={src} alt="采样截图" />;
  return (
    <div className="thumb missingThumb">
      <ImageOff size={18} />
      <span>{sample.screenshot_path ? "截图已清理" : "无截图"}</span>
    </div>
  );
}

function EvidenceDetail({
  sample,
  close,
  correct,
  busy,
}: {
  sample: EvidenceSample;
  close: () => void;
  correct: (sample: EvidenceSample, classification: Classification) => Promise<void>;
  busy: boolean;
}) {
  const src = useScreenshotSrc(sample);
  return (
    <div className="modalBackdrop" role="presentation" onClick={close}>
      <section className="detailModal" role="dialog" aria-modal="true" onClick={(event) => event.stopPropagation()}>
        <header className="detailHeader">
          <div>
            <p>{formatDateTime(sample.captured_at)}</p>
            <h2>{sample.app_name || "未知应用"}</h2>
          </div>
          <button className="iconButton" title="关闭" onClick={close}>
            <X size={18} />
          </button>
        </header>

        {src ? (
          <img className="previewImage" src={src} alt="采样截图预览" />
        ) : (
          <div className="previewMissing">
            <ImageOff size={28} />
            <span>{sample.screenshot_path ? "截图已清理" : "这条记录没有截图"}</span>
          </div>
        )}

        <div className="detailGrid">
          <DetailItem label="当前分类" value={clsLabel(sample.effective_classification)} />
          <DetailItem label="原始分类" value={clsLabel(sample.classification)} />
          <DetailItem label="置信度" value={`${Math.round(sample.confidence * 100)}%`} />
          <DetailItem label="来源" value={sample.ai_error ? "AI 失败后本地判断" : sample.manual_classification ? "人工纠正" : "本地/AI 判断"} />
        </div>

        <div className="detailText">
          <strong>窗口标题</strong>
          <p>{sample.window_title || "无窗口标题"}</p>
          <strong>判断原因</strong>
          <p>{sample.reason}</p>
          {sample.ai_error && (
            <>
              <strong>AI 错误</strong>
              <p>{sample.ai_error}</p>
            </>
          )}
          <strong>内容标签</strong>
          <p>{sample.topic || "未标注内容"}</p>
        </div>

        <div className="correctionBar">
          {(["focused", "distracted", "idle", "unknown"] as Classification[]).map((item) => (
            <button
              key={item}
              disabled={busy}
              className={sample.effective_classification === item ? "activeChoice" : ""}
              onClick={() => correct(sample, item)}
            >
              {clsLabel(item)}
            </button>
          ))}
        </div>
      </section>
    </div>
  );
}

function useScreenshotSrc(sample: EvidenceSample) {
  const [src, setSrc] = React.useState<string | null>(null);
  React.useEffect(() => {
    let cancelled = false;
    setSrc(null);
    if (!sample.screenshot_path || !sample.screenshot_exists) return;
    backend
      .getScreenshotDataUrl(sample.screenshot_path)
      .then((value) => {
        if (!cancelled) setSrc(value);
      })
      .catch(() => {
        if (!cancelled) setSrc(null);
      });
    return () => {
      cancelled = true;
    };
  }, [sample.id, sample.screenshot_path, sample.screenshot_exists]);
  return src;
}

function DetailItem({ label, value }: { label: string; value: string }) {
  return (
    <div className="detailItem">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
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
          <span>填入 OpenAI API Key，并选择本地优先或云端增强后才会调用</span>
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
      <label className="field full">
        Responses API 地址
        <input
          value={settings.openai_api_base_url}
          onChange={(event) => setSettings({ ...settings, openai_api_base_url: event.target.value })}
          placeholder="https://api.openai.com/v1"
        />
      </label>
      <label className="field">
        AI 模型
        <input value={settings.openai_model} onChange={(event) => setSettings({ ...settings, openai_model: event.target.value })} />
      </label>

      <div className="aiHelp full">
        <strong>接入 AI 需要四步</strong>
        <span>1. 填 API Key；2. 填兼容 Responses API 的地址，例如 https://api.openai.com/v1 或第三方 /v1 地址；3. 填模型名；4. 打开 AI 分析，并保持隐私模式不是“完全本地”。</span>
      </div>

      <div className="settingsActions full">
        <button className="primary save" disabled={busy} onClick={save}>
          保存设置
        </button>
        <button disabled={busy} onClick={async () => {
          try {
            const result = await backend.testAiSettings(settings);
            window.alert(result.message);
          } catch (error) {
            window.alert(String(error));
          }
        }}>
          测试 AI
        </button>
      </div>
      {message && <div className="toast full">{message}</div>}
    </section>
  );
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
