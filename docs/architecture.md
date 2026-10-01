# VoxType 架构概览

本文说明 VoxType 的主要模块边界和数据流。目标是让维护者快速判断一次改动会影响哪些链路，而不是定义复杂的架构流程。

## 主链路

```mermaid
flowchart TD
  UI["Svelte UI\nroutes/+page.svelte\nsrc/lib/app/*Controller.svelte.ts"]
  Privacy["Privacy & Local Data\nPrivacySection + privacyController"]
  IPC["Tauri invoke / listen\ncommands + events"]
  Commands["Rust commands\nsrc-tauri/src/lib.rs"]
  LocalData["Local data commands\nstatus + clear actions"]
  Session["SessionController\nsrc-tauri/src/session.rs"]
  Audio["AudioCapture\nsrc-tauri/src/audio.rs"]
  OCR["Screen OCR Context\nsrc-tauri/src/screen_context.rs"]
  ASR["ASR provider\nasr_provider.rs + asr_ws/ + aliyun_asr.rs"]
  LLM["optional LLM post edit\nsrc-tauri/src/llm_post_edit.rs"]
  TextOutput["TextOutput\nsrc-tauri/src/text_output.rs"]
  SideEffects["Overlay / Tray / Stats\nRecent Context / Hotword History"]

  UI --> IPC --> Commands --> Session
  UI --> Privacy --> IPC
  Commands --> LocalData --> SideEffects
  Session --> Audio --> ASR
  Session --> OCR --> ASR
  ASR -->|"final text"| LLM
  ASR -->|"LLM disabled or skipped"| TextOutput
  LLM --> TextOutput
  Session --> SideEffects
  TextOutput --> SideEffects
```

主窗口使用 Svelte 维护界面状态，通过 Tauri `invoke` 调用 Rust command，通过 `listen` 接收会话、字幕、统计、托盘和关闭提示事件。Rust 侧由 `SessionController` 统一管理录音会话状态，具体能力拆到音频、ASR、LLM、剪贴板输出、悬浮字幕、托盘、统计和上下文模块。

## 关键设计

### generation 防旧 worker 覆盖新会话

每次开始录音都会递增 `SessionController` 内部的 `generation`。ASR worker 启动时绑定当前 generation，后续状态更新、失败回写和成功结束都必须带着同一个 generation。若用户快速停止并开始下一轮，旧 worker 的迟到结果会被忽略，避免覆盖新会话状态、误显示成功或误恢复旧的处理阶段。

### 剪贴板恢复策略

`TextOutput` 只负责最终文本输出。默认流程是先按配置尝试快照原剪贴板，再写入识别文本，读回校验后发送 `Ctrl+V` 或 `Shift+Insert`，最后在安全延迟后恢复原剪贴板。无法安全快照的格式会跳过；恢复失败只记录 warning，不把已经成功粘贴的主流程改成失败。Win32 资源使用 `ClipboardGuard`、`OwnedGlobalMemory` 和 `LockedMemory` 收敛关闭、释放和解锁边界；`SetClipboardData` 成功后内存所有权转交系统剪贴板。

打开剪贴板必须带上本线程的隐藏消息窗口（`ClipboardOwnerWindow`）。`OpenClipboard(NULL)` 不独占：其他同样传 NULL 的线程可以在持有期间再次打开成功并夺走剪贴板，随后本线程读取会报 `ERROR_CLIPBOARD_NOT_OPEN`。带窗口句柄后冲突表现为可重试的“拒绝访问”，先做几次 5ms 短重试，再走按配置的慢速重试。粘贴延迟从文本写入剪贴板那一刻算起，读回重试不额外叠加。

快照按格式记录哪些没有备份。系统能从已备份格式重新合成的不算丢失：位图族 `CF_BITMAP` / `CF_DIB` / `CF_DIBV5` / `CF_PALETTE` 只要备份了其中的 DIB 就能互相合成；文本族只认一个方向，即已备份 `CF_UNICODETEXT` 时的 `CF_TEXT` / `CF_OEMTEXT` / `CF_LOCALE`，因为从 ANSI 文本反推 Unicode 会丢字符。只有确实拿不回来的格式才产生“部分恢复”提示，且该提示是静默的，只按 info 记录。

### 窗口不可见时不做无用功

VoxType 常驻托盘，一天里绝大部分时间两个窗口都是隐藏的。隐藏的 WebView 仍会照常运行脚本和渲染，所以需要显式约束：

- 悬浮字幕窗的字幕和外观主要靠事件推送，轮询只是兜底。显示期间每 250ms 取一次字幕、每秒核对一次外观；隐藏时降为 2 秒心跳且不读配置。后端显示字幕窗前会先推送外观和字幕事件，字幕页收到后立即恢复快速轮询。`get_overlay_text` 的返回带 `visible` 字段，心跳据此判断快慢。
- 主窗口隐藏到托盘后，前端暂停循环动画并不再应用麦克风电平；`main-window-hidden` / `main-window-shown` 事件和 `get_app_snapshot` 的 `main_window_visible` 字段维护这个状态。事件比快照新：快照请求在路上时收到过事件，就不再采用快照里的可见性。
- 字幕事件会广播给所有窗口，但只有字幕页处理；主窗口不渲染字幕，不跟着做排版测量。
- 开机自启动（注册表启动项带 `--autostart`）且语音识别已配置好时，主窗口保持隐藏；手动打开或尚未配置好时照常显示。主窗口在 `tauri.conf.json` 里默认不可见，由启动流程决定是否显示。两个例外保证始终有入口：从托盘“重启程序”是用户主动操作，新进程虽然继承了 `--autostart` 也照常显示主窗口；托盘图标创建失败时不保持隐藏。

新增周期性工作或循环动画前，先确认它在窗口隐藏时会停下来。

### 统计口径

统计文件只记录时间、耗时、字数和速度等非正文数据，不记录识别文本。节省时间统一按净节省计算：

```text
手打等效时间 - 实际语音时长
```

默认手打速度和语音速度只用于估算展示，最近 24 小时、最近 7 日和按日统计使用同一口径。

### 隐私边界

默认不得把真实密钥、识别正文、热词、prompt、最近上下文正文、屏幕 OCR 正文或 Windows 用户名路径写入日志、诊断报告、截图或文档。统计只保存非正文指标。最近上下文默认关闭；开启后写入独立本地文件，不写回 `config.toml`。自动热词历史默认关闭，只有开启后才保存 VoxType 自己生成的最终语音输入文本。主窗口提供“隐私与本地数据”页面，把保存位置、上传边界和本地清理动作前置给用户；该页只读取计数和开关状态，不展示正文内容。

### 配置和本地文件

- 主配置：`config.toml`。开发时使用项目根目录；安装版使用 `%APPDATA%\VoxType\config.toml`。
- 日志：开发时在项目根目录；安装版在 `%LOCALAPPDATA%\VoxType\logs\voice_input.log`，单文件 2MB 轮转，保留 3 份。每行整行一次写入并带毫秒时间戳，轮转和追加在进程内串行。
- 统计文件：`voice_input_stats.jsonl`。优先使用已存在的文件；还没有时，开发版建在工作目录，安装版建在程序所在目录（开机自启动时工作目录是系统目录，不能往那里写）。
- 最近上下文：`context/recent_context.jsonl`，位于 `config.toml` 同级目录下的 `context/`。
- 自动热词历史：`context/hotword_history.jsonl`，同样位于配置目录下的 `context/`。
- 示例配置：`config.example.toml`，只能放占位值。

配置和日志按“是否处于开发布局”二选一；统计文件优先寻找已存在文件，再按上面的规则新建。
`get_local_data_status` 只汇总最近上下文条数、自动热词历史条数、统计记录数和相关开关；`clear_recent_context`、`clear_hotword_history` 与 `clear_usage_stats` 分别清理本地正文历史和非正文统计。

## ASR / LLM / OCR 数据流

ASR 质量与延迟相关改动必须同时参考 [ASR 质量与延迟守门清单](asr-quality-latency-guardrails.md)。该清单记录 0.1.102 后实测有效的参数组合、不可回退点、测试和手工回归建议。

1. 开始录音时，`SessionController` 加载配置，启动麦克风采集，并按需启动屏幕 OCR。
2. `screen_context.rs` 按配置截取当前显示器或当前前台窗口，在独立线程中执行，早于麦克风启动发起。豆包通道的 OCR 等待与 WebSocket 建连并行：握手不依赖 OCR，只有首包 payload 依赖；阿里云 FunASR 通道仍在建连前解析上下文，保持原有顺序。两条通道都通过 `PendingScreenContext` 解析一次，随后由 ASR 首包和 LLM 润色共用同一份文本。上下文只在本轮请求内使用，失败或超时会跳过，不阻断录音、最终识别和粘贴。首包等待超时后接收端会保留：迟到的结果赶不上 ASR 首包，但润色阶段会取用已经到达的结果，不做任何额外等待。
3. `asr_provider.rs` 按 `asr.provider` 选择豆包 ASR 或阿里云 FunASR，并做当前服务的启动前配置检查。
4. 豆包模式由 `asr.rs` 组装请求、`asr_ws/` 维护流式 WebSocket 会话、音频发送、最终文本选择和错误映射；阿里云模式由 `aliyun_asr.rs` 维护 `run-task`、音频上传、`finish-task` 和 `task-finished` 门禁。热词、最近上下文、场景上下文和 OCR 结果会按服务能力作为上下文发送；OCR 会标注为开始录音时的屏幕 OCR 上下文，不是用户指令或待识别文本。
5. 实时片段只用于悬浮字幕，最终结果进入后处理。豆包必须等待最终包，阿里云必须等待 `task-finished`；缺少最终结果时进入失败态。
6. `llm_post_edit.rs` 只在 LLM 已启用、润色触发长度达到 `min_chars` 且 Base URL、API Key、模型名完整时调用；中文按单字计，英文和数字按连续词片段计。用户词典、场景与偏好上下文、可选最近上下文和屏幕 OCR 会作为参考信息分区追加，并明确不是待润色文本或指令来源，也不能把待润色文本没说的参考信息补进输出。最近上下文进入 LLM 需要 `context.enable_recent_context` 和 `llm_post_edit.use_recent_context` 同时开启，并限制为最近几段中的约 600 字；默认提示词会保持待润色文本的主要语言，不主动翻译中文或外语内容；否则直接使用 ASR 最终文本。
7. `text_output.rs` 输出最终文本。空识别必须进入失败态，不触发 LLM、粘贴或成功统计。
8. 成功输出后，统计刷新、最近上下文和自动热词历史按配置更新；这些文件仍受隐私边界限制。

## 维护建议

- 新增 UI 状态优先放到 `src/lib/app/*Controller.svelte.ts`，让 `VoxTypeController.svelte.ts` 继续作为组合入口。
- 改主链路时先判断是否影响 ASR、LLM、剪贴板、统计、日志脱敏、热键或托盘，并同步 README、Wiki 或本文件中对应说明。
- 改配置字段时同步 Rust 默认值、配置模板、前端设置项、三语言文案和文档。
- 日常小改和发布前检查可以参考 [维护指南](maintenance-guide.md)，避免把 provider、设置页、隐私和发布规则散落在审计记录里。
