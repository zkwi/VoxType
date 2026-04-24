# ASR_IME

ASR_IME 是一个 Windows 桌面语音输入工具。把光标放到任意输入框后，按下全局热键、右 Alt 或鼠标中键开始说话，程序会录制麦克风音频，通过豆包流式 ASR WebSocket 识别语音，并将最终文本写入剪贴板后粘贴到当前输入位置。

当前代码已迁移为根目录 Tauri 项目：Rust 负责全局热键、输入钩子、音频采集、ASR 会话、剪贴板、系统托盘、悬浮字幕窗和系统音量；Svelte 负责主窗口 GUI。

> 这是个人项目，目标是实用、轻量、易修改。请勿把真实密钥、个人热词、上下文或本地日志提交到仓库。

## 界面预览

主界面采用蓝白配色和紧凑侧边栏，常用状态、触发方式和最近统计集中在首页。

<img src="screenshots/ScreenShot_2026-04-24_150319_363.png" alt="ASR_IME 主输入界面" width="820">

配置页直接以文本表单展示本地配置项，便于个人项目快速修改和排查。

<img src="screenshots/ScreenShot_2026-04-24_150357_064.png" alt="ASR_IME 配置界面" width="820">

统计页展示最近 24 小时、最近 7 日和按日使用情况，新识别结果写入后会刷新。

<img src="screenshots/ScreenShot_2026-04-24_150402_814.png" alt="ASR_IME 统计历史界面" width="820">

录音过程中会在当前屏幕居下显示悬浮字幕，用于实时查看转写内容。

<img src="screenshots/ScreenShot_2026-04-24_150427_629.png" alt="ASR_IME 实时字幕悬浮窗" width="560">

## 功能

- 全局触发：默认 `CTRL+Q`，同时支持右 Alt 和鼠标中键。
- 麦克风采集：使用 Rust `cpal` 采集 PCM 音频，可选择输入设备。
- 流式识别：对接豆包 `bigmodel_async` WebSocket，支持实时片段和最终结果。
- 悬浮字幕：录音时在屏幕居下显示实时识别文本，不抢焦点。
- 自动输入：最终文本写入剪贴板，并用带扫描码和短间隔的 `Ctrl+V` 或 `Shift+Insert` 粘贴到当前焦点输入框。
- 可选润色：可调用 OpenAI 兼容接口做轻度后处理。
- 系统音量：可配置录音期间临时静音系统音量，结束后恢复。
- 托盘常驻：关闭主窗口时隐藏到托盘，只有托盘菜单“退出”才正式退出。
- 多语言界面：简体中文、繁体中文、英语，默认简体中文。

## 环境

仅面向 Windows 10/11。

需要安装：

- Node.js 和 npm
- Rust 工具链
- WebView2 Runtime，Windows 11 通常已内置

如果 Rust 已安装但当前终端找不到 `cargo`，先执行：

```powershell
$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"
```

## 配置

复制配置模板：

```powershell
Copy-Item .\config.example.toml .\config.toml
```

至少填写豆包 ASR 认证信息：

```toml
[auth]
app_key = ""
access_key = ""
resource_id = "volc.seedasr.sauc.duration"
```

如果启用大模型润色，还需要填写：

```toml
[llm_post_edit]
enabled = true
api_key = ""
base_url = "https://dashscope.aliyuncs.com/compatible-mode/v1"
model = "qwen3.5-plus"
```

`config.toml`、本地日志和统计文件已被 `.gitignore` 忽略。示例配置和文档只保留占位值，不应写入真实密钥、个人热词或自定义上下文。

## 开发运行

在仓库根目录执行：

```powershell
npm install
npm run tauri dev
```

开发服务固定使用：

```text
http://127.0.0.1:18080
```

没有继续使用 Tauri 模板默认的 `1420` 端口，因为部分 Windows 环境会把相邻端口段保留给系统，导致 Vite 报 `listen EACCES`。

## 构建

调试构建：

```powershell
npx tauri build --debug --no-bundle
```

正式构建：

```powershell
npx tauri build
```

正式可执行文件通常位于：

```text
src-tauri\target\release\asr-ime-desktop.exe
```

不要直接用 `cargo build --release` 作为桌面端发布产物；那样不会先构建前端资源，可能导致窗口打开后访问开发地址失败。

## 使用

1. 启动 `ASR_IME Desktop`。
2. 在配置页检查 ASR 密钥、麦克风和粘贴方式。
3. 把光标放到目标输入框。
4. 按 `CTRL+Q`、右 Alt 或鼠标中键开始录音。
5. 录音时查看屏幕居下悬浮字幕。
6. 再按一次触发键停止录音。
7. 程序等待最终识别结果，可选润色，然后自动粘贴到当前焦点输入框。

托盘行为：

- 双击托盘图标：打开主窗口。
- 托盘菜单“打开主窗口”：显示并聚焦主窗口。
- 托盘菜单“打开配置”：用系统默认编辑器打开 `config.toml`。
- 托盘菜单“退出”：停止会话并退出程序。

## 常用命令

```powershell
# 前端类型检查
npm run check

# 前端构建
npm run build

# Rust 检查
Set-Location .\src-tauri
cargo check

# Rust 测试
cargo test

# 本地密钥扫描
Set-Location ..
npm run scan:secrets
```

启用 Git pre-commit 钩子：

```powershell
.\scripts\enable_git_hooks.ps1
```

钩子会调用 `scripts/scan-secrets.mjs` 扫描暂存文件，避免误提交本地配置、密钥、热词和上下文。

## 目录

```text
ASR_IME/
├── src/                         # Svelte 主窗口界面
├── src-tauri/                   # Tauri/Rust 桌面端
│   ├── src/
│   │   ├── audio.rs             # 麦克风采集
│   │   ├── asr.rs               # ASR 请求组装与结果解析
│   │   ├── asr_ws.rs            # 豆包 WebSocket 会话
│   │   ├── config.rs            # TOML 配置加载
│   │   ├── hotkey.rs            # 全局热键与输入钩子
│   │   ├── llm_post_edit.rs     # LLM 后处理
│   │   ├── overlay.rs           # 悬浮字幕窗
│   │   ├── session.rs           # 录音会话状态机
│   │   ├── stats.rs             # 使用统计
│   │   ├── system_audio.rs      # 系统音量控制
│   │   ├── text_output.rs       # 剪贴板与粘贴
│   │   └── tray.rs              # 系统托盘
│   ├── capabilities/
│   ├── icons/
│   └── tauri.conf.json
├── static/                      # 静态资源
├── docs/                        # 接口参考文档
├── scripts/
│   ├── enable_git_hooks.ps1
│   └── scan-secrets.mjs
├── config.example.toml          # 配置模板，不含真实密钥
├── package.json
├── svelte.config.js
├── tsconfig.json
└── vite.config.js
```

## 本地文件

以下文件只用于本机运行，不提交：

- `config.toml`
- `*.local.toml`
- `voice_input.log`
- `voice_input_stats.jsonl`
- `src-tauri/target/`
- `node_modules/`
- `build/`
