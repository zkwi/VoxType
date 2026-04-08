# ASR_IME - 流式语音输入法

Windows 桌面语音输入工具，按下热键即可语音输入到任意应用的输入框。

**核心特性：**

- 热键触发录音（默认 `Ctrl+Q`，也支持右 Alt / 鼠标中键）
- 流式语音识别（豆包 `bigmodel_async`），支持二遍识别
- 支持热词直传、上下文、图片上下文
- 桌面悬浮字幕窗，实时显示识别结果（不抢焦点）
- 识别完成后自动复制到剪贴板并粘贴到当前输入位置
- 可选：调用大模型（阿里云百炼）对长文本做轻度润色

> **声明**：这是一个个人项目，代码全部由 AI 编写，功能并不完善，可能存在 bug。仅供学习参考，请勿用于生产环境。

## 截图

| 正在录音 | 转写中 | 识别完成 |
|:---:|:---:|:---:|
| ![正在录音](screenshots/正在录音.png) | ![转写中](screenshots/转写中.png) | ![识别完成](screenshots/识别完成.png) |

## 快速开始

### 1. 安装依赖

需要 Python 3.11+：

```powershell
pip install -r requirements.txt
```

### 2. 准备配置

复制配置模板并填入你自己的密钥：

```powershell
cp config.example.toml config.toml
```

编辑 `config.toml`，至少填写：

| 字段 | 说明 |
|------|------|
| `auth.app_key` | 豆包语音识别 App Key |
| `auth.access_key` | 豆包语音识别 Access Key |
| `auth.resource_id` | 资源 ID（默认值一般不用改） |

可选配置：

| 字段 | 说明 |
|------|------|
| `context.hotwords` | 热词列表，提升特定词汇识别准确率 |
| `context.prompt_context` | 场景描述，按"从新到旧"排列，最多保留 20 条 |
| `context.image_url` | 图片上下文（仅支持 1 张，是否有效由豆包接口校验） |
| `llm_post_edit.enabled` | 是否启用大模型润色（默认关闭） |
| `llm_post_edit.api_key` | 阿里云百炼 API Key（启用润色时需填写） |
| `debug.print_transcript_to_console` | 是否在 Python 控制台打印最终转写结果，默认 `true` |

程序启动时会额外输出一次使用统计，包含：

- 最近 24 小时的语音输入总时长、输入文字总数、平均每分钟输入字数
- 最近 7 日的同类统计

统计数据保存在项目根目录的 `voice_input_stats.jsonl`，每次成功输出文本后追加一条记录，便于个人回看分析。当前统计从新版本开始累计，不会自动回填旧日志。

> **注意**：`config.toml` 包含你的 API 密钥，已在 `.gitignore` 中，不会被提交到 Git。

### 3. 运行

```powershell
.\run.ps1
```

如果你想临时关闭 Python 控制台里的转写结果打印：

```powershell
.\run.ps1 -PrintTranscriptToConsole $false
```

### 4. 使用方式

1. 把光标放到目标输入框
2. 按 `Ctrl+Q`（或右 Alt / 鼠标中键）开始说话
3. 再按一次停止录音
4. 程序自动将识别结果粘贴到当前输入位置

## 打包为 EXE

```powershell
.\build_exe.ps1 [-PythonExe "你的python路径"]
```

产物在 `dist\voice_input\` 目录，分发时至少保留 `voice_input.exe` 和 `config.toml` 在同一目录。

## 本地安全检查

可以先手动扫描一次仓库里的明显密钥：

```powershell
python .\scripts\scan_secrets.py
```

如果要在每次提交前自动检查，执行：

```powershell
.\scripts\enable_git_hooks.ps1
```

启用后，Git 会在 `pre-commit` 阶段自动运行 `scripts/scan_secrets.py --staged`，只检查本次已暂存、准备提交的文件。像 `config.toml` 这类已被 `.gitignore` 忽略、且未加入暂存区的本地文件，不会拦截提交。

## 配置说明

### 音频设置 (`audio`)

| 字段 | 默认值 | 说明 |
|------|--------|------|
| `sample_rate` | 16000 | 采样率 |
| `channels` | 1 | 声道数 |
| `segment_ms` | 200 | 每段音频时长（毫秒） |
| `max_record_seconds` | 300 | 最长录音时间（秒），超时自动停止 |
| `mute_system_volume_while_recording` | true | 录音时静音系统音量，结束后恢复 |
| `input_device` | null | 麦克风设备编号，null 为系统默认 |

### 识别请求 (`request`)

| 字段 | 默认值 | 说明 |
|------|--------|------|
| `enable_nonstream` | true | 启用二遍识别（更准确） |
| `enable_itn` | true | 逆文本正则化（数字、日期等） |
| `enable_punc` | true | 自动标点 |
| `enable_ddc` | true | 顺滑功能 |
| `final_result_timeout_seconds` | 15 | 停止录音后等待最终结果的超时时间 |

### 大模型润色 (`llm_post_edit`)

当识别文本长度超过 `min_chars` 时，调用大模型做一次轻度润色（修正识别错误、去口头语、整理结构）。

| 字段 | 默认值 | 说明 |
|------|--------|------|
| `enabled` | false | 是否启用 |
| `min_chars` | 40 | 触发润色的最少字符数 |
| `base_url` | (阿里云百炼) | OpenAI 兼容接口地址 |
| `model` | qwen3.5-plus | 模型名称 |
| `system_prompt` | (内置) | 系统提示词，可自定义 |

### 界面设置 (`ui`)

| 字段 | 默认值 | 说明 |
|------|--------|------|
| `width` | 460 | 悬浮窗宽度 |
| `height` | 88 | 悬浮窗高度 |
| `margin_bottom` | 64 | 距屏幕底部距离 |
| `opacity` | 0.9 | 窗口不透明度 |

## 项目结构

```
ASR_IME/
├── main.py                  # 入口
├── config.example.toml      # 配置模板
├── requirements.txt         # Python 依赖
├── build_exe.ps1            # PyInstaller 打包脚本
├── voice_input/
│   ├── app.py               # 主应用：热键注册、录音控制、会话管理
│   ├── asr_client.py        # 豆包 ASR WebSocket 客户端
│   ├── protocol.py          # 豆包二进制协议编解码
│   ├── audio_capture.py     # 麦克风录音
│   ├── stats.py             # 轻量统计记录与聚合
│   ├── overlay.py           # 悬浮字幕窗（PyQt6）
│   ├── text_output.py       # 剪贴板写入 + 模拟粘贴
│   ├── llm_post_edit.py     # 大模型润色（OpenAI 兼容接口）
│   ├── input_hooks.py       # 右 Alt / 鼠标中键的低级输入钩子
│   ├── system_audio.py      # 系统音量控制（录音时静音）
│   └── config.py            # 配置文件加载
├── voice_input.log          # 运行日志
├── voice_input_stats.jsonl  # 语音输入统计数据（本地文件，不提交）
└── 参考文档.md               # 豆包 ASR API 参考
```

## 依赖

- Python 3.11+
- PyQt6 - GUI 框架
- aiohttp - WebSocket 通信
- sounddevice - 麦克风录音
- pycaw / comtypes - Windows 音量控制
- pynput - 输入钩子
- openai - 大模型调用（OpenAI 兼容接口）
- pyperclip - 剪贴板操作

## License

个人项目，代码全部由 AI 编写，仅供学习参考。
