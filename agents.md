# ASR_IME — AI 编码助手上下文指南

> 本文件用于帮助 AI 编码助手（如 Antigravity、GitHub Copilot、Cursor 等）快速理解本项目的目标、架构和约定，以便给出更准确、更符合风格的代码建议。

---

## 项目概述

**ASR_IME** 是一个 Windows 桌面语音输入工具（输入法辅助）。  
用户按下热键后，程序录制麦克风音频，通过豆包（字节跳动）流式 ASR WebSocket 接口进行实时语音识别，识别完成后可选调用阿里云百炼大模型做轻度润色，最终将文本复制到剪贴板并粘贴到当前输入框。

**语言 / 运行环境：** Python 3.11+，仅支持 Windows  
**框架 / 核心依赖：** PyQt6（GUI）、aiohttp（WebSocket）、sounddevice（录音）、openai SDK（LLM 润色）、pynput（输入钩子）、pycaw / comtypes（音量控制）、pyperclip（剪贴板）

---

## 目录结构

```
ASR_IME/
├── main.py                   # 入口，调用 voice_input.app.main()
├── config.example.toml       # 配置模板（不含密钥）
├── config.toml               # 实际配置（含密钥，已被 .gitignore 忽略）
├── requirements.txt
├── run.ps1                   # 开发运行脚本
├── build_exe.ps1             # PyInstaller 打包脚本
├── voice_input_stats.jsonl   # 本地使用统计（不提交）
├── voice_input.log           # 运行日志（不提交）
├── voice_input/
│   ├── app.py               # ★ 主应用：热键注册、录音控制、会话生命周期
│   ├── asr_client.py        # 豆包 ASR WebSocket 客户端（流式 + 二遍识别）
│   ├── protocol.py          # 豆包二进制协议编解码（Header + Payload）
│   ├── audio_capture.py     # 麦克风录音（sounddevice，生产者-消费者队列）
│   ├── overlay.py           # 悬浮字幕窗（PyQt6，无焦点抢占）
│   ├── text_output.py       # 剪贴板写入 + Win32 模拟粘贴（Ctrl+V）
│   ├── llm_post_edit.py     # 大模型润色（OpenAI 兼容接口，默认阿里云百炼）
│   ├── input_hooks.py       # 低级输入钩子：右 Alt / 鼠标中键（pynput）
│   ├── system_audio.py      # 录音时静音 / 恢复系统音量（pycaw）
│   ├── stats.py             # 使用统计记录与聚合（JSONL 追加）
│   └── config.py            # 配置加载（TOML → dict，带默认值）
├── scripts/
│   ├── scan_secrets.py      # 本地密钥扫描（pre-commit 钩子）
│   └── enable_git_hooks.ps1 # 启用 Git pre-commit 钩子
└── docs/
    └── 豆包流式语音识别参考文档.md
```

---

## 核心模块说明

### `voice_input/app.py` — 主应用

- **`VoiceInputApp`**：单例主类，管理完整的录音会话生命周期
  - 使用 `GlobalHotkeyThread`（Win32 `RegisterHotKey`）和 `InputHookThread`（pynput）双路热键监听
  - 录音/识别通过独立线程(`_run_session`)运行，用 `asyncio.run()` 驱动异步 ASR 和 LLM 润色
  - `UiBridge`（`QObject` + `pyqtSignal`）跨线程安全地驱动 Qt UI 更新
  - 会话结束后：粘贴文本 → 记录统计 → 恢复音量 → 隐藏悬浮窗

- **热键流程**：`hotkey_thread` 触发 `threading.Event` → 50ms 轮询定时器 `_poll_input_hook()` → `toggle_recording()`

### `voice_input/asr_client.py` — ASR 客户端

- **`DoubaoAsrClient`**：通过 `aiohttp` WebSocket 连接豆包 `bigmodel_async` 接口
- **双协程模式**：`sender()` 持续发送 PCM 音频块；`receiver()` 持续接收，跟踪 definite utterance 片段
- **优先输出二遍结果**：`definitive_text`（由 `definite=true` 的 utterance 拼接）优先级高于 `final_text`
- **上下文组装**：热词、最近对话上下文（`recent_context`）、图片 URL 组合为 `corpus.context` JSON 字段

### `voice_input/protocol.py` — 豆包二进制协议

- 字节跳动自定义二进制协议：4 字节 Header（版本、消息类型、序列化类型、压缩类型）+ 4 字节 payload 长度 + payload
- `build_full_request()`：构建初始握手包（JSON payload）
- `build_audio_request()`：构建音频数据包（raw PCM bytes）
- `parse_response()`：解析服务端响应，提取 JSON payload

### `voice_input/overlay.py` — 悬浮字幕窗

- `Qt.WindowType.Tool | Qt.WindowType.WindowStaysOnTopHint | Qt.WindowType.FramelessWindowHint` — 无边框、置顶、不抢焦点
- 支持文字滚动显示（`QTimer` + `QPainter` 逐帧绘制）
- 状态：录音中（蓝色脉冲动画） / 转写中 / 识别完成

### `voice_input/llm_post_edit.py` — LLM 润色

- **`AliyunLlmPostEditor`**：异步调用 OpenAI 兼容接口
- 仅在 `enabled=true` 且文本长度 ≥ `min_chars` 时触发
- 失败时静默回退到原文（不抛出异常，不影响主流程）

### `voice_input/stats.py` — 使用统计

- `StatsStore`：追加写入 JSONL，每条记录包含 `text_length`、`duration_seconds`、`created_at`
- `summarize_recent_hours(n)` / `summarize_recent_days(n)`：按时间窗口聚合，输出 `UsageStats`

---

## 配置结构（`config.toml`）

```toml
hotkey = "ctrl+q"

[auth]
app_key = ""
access_key = ""
resource_id = ""

[audio]
sample_rate = 16000
channels = 1
segment_ms = 200
max_record_seconds = 300
stop_grace_ms = 500
mute_system_volume_while_recording = true

[request]
ws_url = "wss://openspeech.bytedance.com/api/v3/sauc/bigmodel_async"
enable_nonstream = true
enable_itn = true
enable_punc = true
enable_ddc = true
final_result_timeout_seconds = 15

[context]
enable_recent_context = true
recent_context_rounds = 5
hotwords = []

[typing]
paste_delay_ms = 120
paste_method = "ctrl_v"

[llm_post_edit]
enabled = false
min_chars = 40
base_url = "https://dashscope.aliyuncs.com/compatible-mode/v1"
model = "qwen3.5-plus"
enable_thinking = false

[ui]
width = 460
height = 88
margin_bottom = 64
opacity = 0.9

[tray]
show_startup_message = true

[debug]
print_transcript_to_console = true
```

> **重要**：`config.toml` 已在 `.gitignore` 中，永远不要提交真实密钥。

---

## 开发约定

### 代码风格
- **类型注解**：所有函数参数和返回值均应有类型注解（`from __future__ import annotations` 可选但已存在）
- **异常处理**：识别链路（ASR、LLM）中的异常一律 `logging.exception` 记录后回退，不向用户暴露堆栈
- **线程安全**：跨线程 UI 更新必须通过 `UiBridge` 信号，禁止直接从工作线程调用 Qt widget 方法
- **asyncio**：ASR 和 LLM 处理已在独立线程的 `asyncio.run()` 中，不在主线程 event loop 上运行

### 新增功能原则
1. **新增配置项** → 在 `config.example.toml` 同步添加带注释的示例值，并在 `config.py` 设置合理默认值
2. **新增 UI 状态** → 通过 `UiBridge` 新增 `pyqtSignal`，在 `overlay.py` 中对应处理
3. **新增外部 API 调用** → 独立封装为 `voice_input/xxx.py` 模块，保持 `app.py` 职责单一
4. **新增热键触发逻辑** → 统一经过 `toggle_recording()` / `start_recording()` / `stop_recording()` 入口

### 密钥安全
- 提交前可运行 `python scripts/scan_secrets.py` 手动扫描
- 执行 `scripts/enable_git_hooks.ps1` 启用 pre-commit 自动扫描
- 禁止在日志中打印任何 API Key / Access Key

---

## 运行 & 调试

```powershell
# 安装依赖
pip install -r requirements.txt

# 运行（开发）
.\run.ps1

# 关闭控制台转写打印
.\run.ps1 -PrintTranscriptToConsole $false

# 打包为 EXE
.\build_exe.ps1

# 扫描潜在密钥泄漏
python .\scripts\scan_secrets.py
```

日志输出到 `voice_input.log` 及控制台，级别为 `INFO`，日志格式：`%(asctime)s %(levelname)s %(message)s`。

---

## 已知限制 & 注意事项

- **仅支持 Windows**：使用了 Win32 API（`RegisterHotKey`、`PostThreadMessage`、`GetForegroundWindow`）以及 pycaw 音量控制
- **热键冲突**：若 `ctrl+q` 已被其他程序占用，`RegisterHotKey` 会失败并在启动时报错
- **二遍识别依赖**：若豆包服务端未返回任何 `definite=true` utterance，本轮输出为空字符串（`asr_client.py:263`）
- **LLM 润色延迟**：网络延迟叠加模型推理时间，建议仅对长文本启用
- **右 Alt / 鼠标中键**：通过 pynput 低级钩子实现，若安全软件拦截键盘/鼠标钩子则可能失效
- **个人项目声明**：代码全部由 AI 编写，功能并不完善，仅供学习参考，勿用于生产环境
