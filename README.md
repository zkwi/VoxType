# 豆包流式语音输入法

一个尽量简洁的 Windows 个人项目：

- `Ctrl+Q` 开始/停止录音
- 录音期间调用豆包流式识别 `bigmodel_async`
- 开启二遍识别
- 支持热词直传、上下文、图片上下文
- 桌面顶部显示一个不抢焦点的悬浮字幕窗
- 停止后把最终识别结果写入剪贴板，并自动粘贴到当前输入框

## 1. 安装依赖

建议使用你项目说明里的 Python：

```powershell
C:\Users\zkwi\miniconda3\envs\Quantitative-investment\python.exe -m pip install -r requirements.txt
```

## 2. 准备配置

直接修改 [config.json](E:\ollama_proxy\config.json)，填写：

- `auth.app_key`
- `auth.access_key`
- `auth.resource_id`
- `context.hotwords`
- `context.prompt_context`
- `context.image_url`

说明：

- 热词会走 `context` 直传，优先级高于热词表。
- `prompt_context` 需要按“从新到旧”填写，程序会最多保留 20 条。
- `image_url` 只支持 1 张图，是否有效由豆包接口侧校验。
- 当最终二遍文本长度超过 `llm_post_edit.min_chars` 时，会调用阿里云百炼做一次轻度润色。
- 阿里云的调用地址、密钥、模型、阈值和提示词都在 `llm_post_edit` 配置段里可调。
- 超过 5 分钟会自动停止录音。
- 默认会在录音开始时把系统主音量静音，录音结束后恢复原状态。可通过 `audio.mute_system_volume_while_recording` 关闭。

## 3. 运行

```powershell
C:\Users\zkwi\miniconda3\envs\Quantitative-investment\python.exe main.py
```

## 4. 打包 EXE

项目已经补好 `PyInstaller` 打包文件，默认会生成一个可分发目录：

```powershell
.\build_exe.ps1
```

产物位置：

- `dist\voice_input\voice_input.exe`
- `dist\voice_input\config.json`

分发给别人时，至少保留这两个文件在同一目录。
程序启动时会优先读取 `exe` 同目录下的 `config.json`。

## 5. 使用方式

1. 先把光标放到目标输入框。
2. 按 `Ctrl+Q` 开始说话。
3. 再按一次 `Ctrl+Q` 停止。
4. 程序会把最终文本复制到剪贴板，并自动粘贴到当前输入位置。

## 6. 备注

- 全局热键现在走 Windows 原生 `RegisterHotKey`。
- 如果你有多个麦克风，可以在 `audio.input_device` 里指定设备编号。
- 如果不希望自动粘贴，可以修改 [voice_input/text_output.py](E:\ollama_proxy\voice_input\text_output.py)。
- `config.json` 包含真实鉴权信息，分发前建议替换为目标环境自己的配置。
