# 声写 VoxType — AI 编码助手上下文指南

本文件用于帮助 AI 编码助手快速理解项目目标、架构和约定。

## 项目定位

声写（VoxType）是一个 Windows 桌面语音输入工具。用户在任意输入框中按下全局触发键后，程序采集麦克风音频，通过豆包流式 ASR WebSocket 获取实时转写，结束后可选调用 OpenAI 兼容 LLM 做轻度润色，最终把文本写入剪贴板并粘贴到当前焦点输入框。

当前主实现是根目录 Tauri 客户端：

- Rust：全局热键、右 Alt/鼠标中键输入钩子、音频采集、ASR 会话、剪贴板、托盘、悬浮字幕窗、系统音量、配置和统计。
- Svelte：主窗口 GUI、多语言、配置页、统计页和状态展示。

项目是个人项目，优先级是实用、简洁、易修改。避免过度抽象和不必要依赖。

## 目录结构

```text
VoxType/
├── src/                         # Svelte 前端界面
├── src-tauri/                   # Tauri/Rust 桌面端
│   ├── src/
│   │   ├── app_log.rs           # 本地日志
│   │   ├── asr.rs               # ASR 请求组装、上下文和结果解析
│   │   ├── asr_ws.rs            # 豆包 WebSocket 会话
│   │   ├── audio.rs             # cpal 麦克风采集
│   │   ├── config.rs            # TOML 配置加载
│   │   ├── hotkey.rs            # 全局热键与输入钩子
│   │   ├── llm_post_edit.rs     # LLM 润色
│   │   ├── overlay.rs           # 屏幕居下悬浮字幕窗
│   │   ├── protocol.rs          # 豆包二进制协议
│   │   ├── session.rs           # 录音会话状态机
│   │   ├── stats.rs             # 使用统计
│   │   ├── system_audio.rs      # 系统音量控制
│   │   ├── text_output.rs       # 剪贴板与模拟粘贴
│   │   └── tray.rs              # 系统托盘
│   ├── capabilities/
│   ├── icons/
│   ├── Cargo.toml
│   └── tauri.conf.json
├── static/                      # 静态资源
├── docs/                        # 豆包接口参考文档
├── scripts/
│   ├── enable_git_hooks.ps1
│   └── scan-secrets.mjs
├── config.example.toml          # 配置模板，不含真实密钥
├── package.json
├── svelte.config.js
├── tsconfig.json
└── vite.config.js
```

## 核心链路

1. 触发源：`CTRL+Q`、右 Alt、鼠标中键或托盘/界面命令。
2. `session.rs` 切换录音状态，显示 `overlay.rs` 悬浮字幕。
3. `audio.rs` 采集 PCM 音频块。
4. `asr_ws.rs` 连接豆包 WebSocket，`protocol.rs` 编码/解析二进制消息。
5. 实时转写结果发送给悬浮字幕窗；最终结果进入后处理。
6. `llm_post_edit.rs` 在启用时做轻度润色，失败则回退原文。
7. `text_output.rs` 写剪贴板并粘贴到当前焦点输入框。
8. `stats.rs` 追加本地统计，`tray.rs` 维持托盘常驻。

## 配置

配置文件为根目录 `config.toml`，模板为 `config.example.toml`。真实配置包含密钥，必须保持未提交。

关键配置段：

- `hotkey`
- `[auth]`
- `[audio]`
- `[request]`
- `[context]`
- `[typing]`
- `[llm_post_edit]`
- `[ui]`
- `[tray]`

新增配置项时，同步更新：

1. `config.example.toml`
2. `src-tauri/src/config.rs`
3. 前端配置页和多语言文案
4. README 中的必要说明

## 开发约定

- Rust 侧保持模块职责直接清楚，避免为个人项目引入复杂抽象层。
- UI 修改优先沿用现有 Svelte 结构和蓝白配色。
- 主窗口只展示正式用户信息，不展示调试状态、内部路径或协议细节。
- 实时字幕应显示在屏幕居下悬浮窗，不应依赖主窗口展示。
- 主窗口关闭时隐藏到托盘；托盘菜单“退出”才是真正退出。
- 触发键逻辑统一进入会话状态机，避免多处直接启动/停止录音。
- 跨线程/异步状态更新要清楚释放资源，尤其是低级输入钩子和鼠标事件拦截。
- 配置模板、README、文档和测试数据不得包含真实密钥、个人热词、私有上下文或本地图片 URL。

## 运行与验证

在仓库根目录运行：

```powershell
npm install
npm run tauri dev
```

常用检查：

```powershell
npm run check
npm run build
npm run scan:secrets

Set-Location .\src-tauri
cargo check
cargo test
```

正式构建使用：

```powershell
npx tauri build
```

不要用单独的 `cargo build --release` 代替 Tauri 构建，否则前端资源可能不会被正确打包。

## 密钥安全

- `config.toml`、`*.local.toml`、日志、统计文件、构建产物和依赖目录均应保持忽略。
- 提交前运行 `npm run scan:secrets`。
- 可运行 `.\scripts\enable_git_hooks.ps1` 启用 pre-commit 扫描。
- 禁止在日志、README、示例配置、测试快照或迁移文档中写入真实密钥、个人热词、最近上下文和用户自定义提示。

## 发布流程

当用户要求“提交、编译、发布、合并到 main”时，默认按以下流程执行，除非用户明确要求其他版本号或分支策略。

1. 确认当前分支与远端状态：

```powershell
git status --short --branch
git fetch origin --tags
git tag --list --sort=-v:refname
```

2. 选择新版本号。已有 `v0.1.0` 时，下一个修复版本使用 `0.1.1`。同步修改：

- `package.json`
- `package-lock.json`
- `src-tauri/Cargo.toml`
- `src-tauri/Cargo.lock`
- `src-tauri/tauri.conf.json`

3. 提交前检查，不允许跳过密钥扫描：

```powershell
npm run check
Set-Location .\src-tauri
cargo fmt --check
cargo check
cargo test
Set-Location ..
npm run scan:secrets
```

4. 正式构建。必须使用 Tauri 构建，不要单独使用 `cargo build --release` 作为发布产物：

```powershell
npx tauri build
```

`src-tauri/tauri.conf.json` 的 `bundle.windows.webviewInstallMode` 应保持 `embedBootstrapper`，让 NSIS 安装包在干净 Windows 电脑缺少 WebView2 Runtime 时自动安装运行时。绿色版 ZIP 不负责安装系统运行时，发布说明需提醒用户干净电脑优先使用安装包。

5. 整理发布产物，安装包取 NSIS 产物，绿色版 ZIP 只打包正式 exe、`config.example.toml`、`README.md` 和 `LICENSE`（如存在），禁止包含：

- `config.toml`
- `*.local.toml`
- `voice_input.log`
- `voice_input_stats.jsonl`
- 任意真实密钥、热词、上下文或个人配置

6. 提交并推送 main：

```powershell
git add <需要提交的源码与文档>
git commit -m "Release vX.Y.Z"
git push origin main
git tag vX.Y.Z
git push origin vX.Y.Z
```

如果当前已经在 `main`，不需要额外合并。若在功能分支，先完成检查和提交，再切回 `main` 并使用用户要求的策略合并或覆盖。

7. 创建 GitHub Release 并上传资产。优先使用 `gh`；若没有安装，则使用 GitHub REST API。资产命名：

```text
VoxType-vX.Y.Z-windows-x64-setup.exe
VoxType-vX.Y.Z-windows-x64-portable.zip
```

Release 说明应简洁列出主要变化和配置提醒，不写任何本地路径、密钥或个人配置。
