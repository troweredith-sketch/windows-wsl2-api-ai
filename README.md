# Study Guard

Study Guard 是一个本地优先的 Windows 自习监督 MVP。前端使用 React + TypeScript，桌面壳和系统能力使用 Tauri v2，后端使用 Rust，数据保存在本地 SQLite。

## 已实现

- 今日学习、日报、设置三页中文界面。
- 开始、暂停、继续、结束学习会话。
- Windows 前台窗口标题和进程名采集。
- 低频屏幕截图保存，并按保留天数清理。
- 本地 SQLite 数据库表：`study_sessions`、`samples`、`daily_reports`、`settings`。
- 本地规则判断 `focused / distracted / idle / unknown`。
- 设置页可开启 OpenAI API Key，默认模型为 `gpt-4.1-mini`。
- 通过 OpenAI Responses API 分析不确定片段；AI 失败时降级为本地规则。
- 隐私模式：本地优先、云端增强、完全本地。
- 非 Tauri 浏览器环境下提供前端 fallback，方便预览 UI。

## 当前环境状态

当前项目已在本机完成以下修复和验证：

- Tauri 图标缺失问题已修复。
- Rust `image` 版本冲突已修复。
- Windows 句柄判断问题已修复。
- WebView2 Runtime 已安装。
- Visual Studio C++ Build Tools 已安装。
- Rust/Cargo 已安装。
- `npm run build` 已验证通过。
- `cargo check` 已验证通过。
- `npm run tauri dev` 可以启动开发版。

如果新开的终端提示找不到 `cargo` 或 `rustc`，通常是 PATH 尚未刷新；重新打开 PowerShell 后再执行命令。

## 数据位置

新版数据目录固定为：

```text
D:\StudyGuard\data
D:\StudyGuard\data\screenshots
D:\StudyGuard\data\study_guard.sqlite3
```

旧版 Tauri 默认数据目录为：

```text
C:\Users\MR\AppData\Roaming\com.local.studyguard
```

确认新版数据正常后，可以删除旧目录释放空间。

也可以通过环境变量覆盖数据目录：

```powershell
$env:STUDY_GUARD_DATA_DIR="D:\StudyGuard\data"
```

## 运行

安装依赖：

```powershell
npm install
```

只预览前端：

```powershell
npm run dev
```

启动完整桌面端：

```powershell
npm run tauri dev
```

`npm run dev` 只启动前端预览；`npm run tauri dev` 才能调用截图、窗口采集、SQLite 等桌面能力。

## 构建检查

```powershell
npm run build
cd src-tauri
cargo check
```

## 隐私说明

- 默认截图间隔为 20 秒。
- 默认截图保留 7 天。
- AI 分析默认关闭。
- 完全本地模式不会上传截图。
- 本地优先模式只在本地规则不确定时尝试 AI 分析。
- 云端增强模式会更积极地调用 AI 分析。
