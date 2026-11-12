# VoxType 0.16.0 发布审计

## 版本与范围

0.15.0 已有正式标签和公开 Release。右 Ctrl 是新增的用户可见触发方式，因此本轮源码升至 0.16.0。版本号同步到 npm、Cargo 和 Tauri 元数据。

- 右 Ctrl 默认关闭，旧配置缺少该字段时按关闭处理。开启后，低级键盘钩子拦截右 Ctrl 的按下和松开，在松开时提交一次录音切换。左 Ctrl 和注入事件原样放行。
- 设置保存后立即更新运行中的触发开关。仅开启右 Ctrl 时也可进入就绪状态，首页、准备提示与脱敏诊断均识别该状态。
- 新增手动设备刷新反馈，补齐资源规格和录音静音字段的导航。单按 Tab 离开快捷键录制框，带修饰键的 Tab 可录制，Esc 取消。
- npm 审计脚本支持从 npm 或 Node.js 直接启动。Windows 直接启动时由系统命令解释器执行固定的 npm.cmd 命令。
- 中英文 README 各自使用单语结构，补充 MIT 协议和第三方云服务边界。Wiki 草稿同步右 Ctrl 和系统提示词说明，历史审计统一保留在归档目录。

## 改动文件

- Rust：`config.rs`、`hotkey.rs`、`commands/config_commands.rs`、`commands/diagnostic_commands.rs`，以及 `session.rs` 的入口说明。
- 前端：`types/app.ts`、`app/defaults.ts`、`app/setupController.svelte.ts`、`app/VoxTypeController.svelte.ts`、`HomeSection.svelte`、`OptionsSection.svelte`、`ApiConfigSection.svelte` 和简体、繁体、英文词库。
- 状态与测试：`setupStatus.ts`、`setupStatus.test.ts`、`settingsFields.ts`、`settingsFields.test.ts`。工具脚本为 `scripts/npm-audit.mjs`。
- 配置与文档：`config.example.toml`、两个 npm 版本文件、两个 Cargo 版本文件、`tauri.conf.json`、`CHANGELOG.md`、`AGENTS.md`、中英文 README、文档索引、Wiki 首页及中英文配置和功能指南。

## 已验证

- 本地 `npm run ai:release-check` 通过，包含依赖审计、Clippy 和 Tauri 调试构建。npm 保留 0.15.0 记录的单条开发依赖例外，Rust 审计报告 8 条允许告警。
- 前端 80 项单测、Rust 290 项单测通过，2 项真实剪贴板测试按设计忽略。覆盖右 Ctrl 与右 Alt 独立开关、按下和松开、左 Ctrl 和注入事件放行、旧配置反序列化、仅右 Ctrl 就绪、诊断摘要及校验导航。
- 在 Windows 独立调试实例中验证了旧配置加载、右 Ctrl 开关保存、仅开启右 Ctrl 的就绪状态和首页显示。实测单按 Tab、Esc、Ctrl+Tab 的录制行为以及手动刷新麦克风提示。首页四个状态在 1280px 排成一行、960px 排成两行两列、390px 排成一列。
- npm 审计脚本的 npm 和 Node.js 入口均通过。中英文 README 各 183 行，变更的 Markdown 本地链接和图片路径已核对。

## 手工回归步骤

1. 用旧配置启动，在「选项 → 使用方式 → 备用触发方式」确认右 Ctrl 默认关闭。开启后检查首页状态，并关闭主快捷键确认准备状态仍认可右 Ctrl。
2. 用物理键盘松开右 Ctrl 开始录音，再松开一次结束。开启时，确认其他应用中的右 Ctrl 组合键交由 VoxType 处理。关闭后，确认这些组合键恢复。
3. 保持右 Ctrl 开启，在文本编辑器用左 Ctrl+C 和左 Ctrl+V 操作。用有效 ASR 配置听写，确认应用模拟的自动粘贴不会引发第二轮录音。
4. 在选项页刷新设备，确认显示结果。用带旧错误值的测试配置触发资源规格校验，确认进入 API 高级设置并显示错误。
