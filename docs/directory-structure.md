# VoxType 目录结构规范

本文件定义 VoxType 后续新增文件应放在哪里。目标是保持结构简单、清楚、易维护。

---

## 1. 当前顶层目录

```text
VoxType/
├── src/                 # Svelte 前端
├── src-tauri/           # Tauri / Rust 后端
├── docs/                # 文档
├── scripts/             # 本地脚本
├── static/              # 静态资源
├── screenshots/         # README、Wiki 或发布说明使用的截图
├── config.example.toml  # 配置模板
├── package.json
├── svelte.config.js
├── tsconfig.json
├── vite.config.js
└── AGENTS.md            # AI 维护规则
```

---

## 2. 前端目录规范

推荐结构：

```text
src/
├── routes/
│   └── +page.svelte
└── lib/
    ├── components/
    │   ├── app/       # 应用外壳：侧边栏、路由容器
    │   ├── pages/     # 页面级区块，例如 API 配置、热词与提示词
    │   ├── overview/  # 首页卡片
    │   ├── settings/  # 设置项控件
    │   ├── overlay/   # 悬浮字幕
    │   └── common/    # 通用按钮、提示、卡片
    ├── app/           # 前端状态 controller
    ├── i18n/
    ├── utils/
    └── types/
```

### 规则

1. `src/routes/+page.svelte` 是主入口，短期保留。
2. 新增组件放到 `src/lib/components/`。
3. 首页组件放到 `src/lib/components/overview/`；整页级区块放到 `src/lib/components/pages/`，应用外壳放到 `src/lib/components/app/`。
4. 设置页组件放到 `src/lib/components/settings/`。
5. 悬浮字幕组件放到 `src/lib/components/overlay/`。
6. 通用按钮、提示、卡片放到 `src/lib/components/common/`。
7. 三语言文案放到 `src/lib/i18n/`。
8. 格式化函数、状态映射函数放到 `src/lib/utils/`。
9. 共享类型放到 `src/lib/types/`。
10. 前端状态 controller 放到 `src/lib/app/`，命名为 `*Controller.svelte.ts`。`VoxTypeController.svelte.ts` 只作为组合入口，新增业务状态优先拆到独立 controller。

### 禁止

1. 不要在 `+page.svelte` 继续新增大型业务块。
2. 不要在组件里硬编码大量用户文案。
3. 不要为一个只用一次的小逻辑创建过深目录。
4. 不要新增 `src/components`、`src/common` 等与 `src/lib/components` 重复的目录。

---

## 3. Rust / Tauri 目录规范

当前结构：

```text
src-tauri/
├── src/
│   ├── aliyun_asr.rs
│   ├── app_log.rs
│   ├── asr.rs
│   ├── asr_activity.rs
│   ├── asr_provider.rs
│   ├── asr_ws/
│   ├── audio.rs
│   ├── autostart.rs
│   ├── commands/
│   ├── config.rs
│   ├── config_validation.rs
│   ├── error.rs
│   ├── hotkey.rs
│   ├── hotword_generator.rs
│   ├── hotword_history.rs
│   ├── llm_client.rs
│   ├── llm_endpoint.rs
│   ├── llm_post_edit.rs
│   ├── llm_request_adapter.rs
│   ├── main_window.rs
│   ├── overlay.rs
│   ├── protocol.rs
│   ├── screen_context.rs
│   ├── session.rs
│   ├── setup_guide.rs
│   ├── stats.rs
│   ├── system_audio.rs
│   ├── text_output.rs
│   ├── tray.rs
│   ├── update.rs
│   ├── lib.rs
│   └── main.rs
├── capabilities/
├── icons/
├── Cargo.toml
└── tauri.conf.json
```

### 模块职责

1. `session.rs`：录音会话状态。
2. `audio.rs`：麦克风采集。
3. `asr_provider.rs`：选择 ASR 服务、做启动前配置门禁，并转交录音会话参数。
4. `asr.rs`：豆包 ASR 请求组装、上下文和结果解析。
5. `asr_ws/`：豆包 WebSocket ASR 会话、音频发送、最终文本和错误映射。
6. `aliyun_asr.rs`：阿里云 FunASR 会话与事件门禁。
7. `asr_activity.rs`：ASR 有效反馈上报，供无反馈自动停止使用。
8. `protocol.rs`：豆包二进制协议。
9. `screen_context.rs`：屏幕 OCR 上下文采集与延后解析。
10. `llm_post_edit.rs`：LLM 润色主流程。
11. `llm_client.rs`、`llm_endpoint.rs`、`llm_request_adapter.rs`：OpenAI 兼容请求发送、端点归一化、思考开关适配。
12. `text_output.rs`：剪贴板和自动粘贴。
13. `hotkey.rs`：全局热键和输入钩子。
14. `overlay.rs`：悬浮字幕。
15. `tray.rs`：系统托盘。
16. `main_window.rs`：主窗口显示与隐藏。
17. `config.rs`：配置模型、加载、保存、迁移。
18. `config_validation.rs`：保存前的字段校验。
19. `commands/`：Tauri command 层，按配置、会话、诊断、更新分文件。
20. `hotword_generator.rs`、`hotword_history.rs`：自动热词候选生成与本地历史。
21. `app_log.rs`：日志和脱敏。
22. `stats.rs`：使用统计。
23. `update.rs`：更新检查。
24. `system_audio.rs`：系统音量。
25. `autostart.rs`：开机启动。
26. `error.rs`：错误上下文辅助。

### 规则

1. 新增 Rust 模块必须有明确职责。
2. 不要新建 `utils.rs` 这种大杂烩模块。
3. 不要让 `lib.rs` 承担大量业务逻辑。
4. 不要在 `asr_ws/` 中直接处理 UI 复杂逻辑。
5. 不要在 `text_output.rs` 之外写剪贴板逻辑。
6. 不要在 `hotkey.rs` 之外写全局热键或输入钩子逻辑。
7. 不要在 `stats.rs` 中保存识别正文。
8. 不要在 `app_log.rs` 外绕过日志脱敏。

---

## 4. 文档目录规范

```text
docs/
├── README.md
├── architecture.md
├── code-style.md
├── directory-structure.md
├── wiki/
│   ├── Home.md
│   ├── Setup-Guide.md
│   ├── Setup-Guide-English.md
│   ├── Feature-Guide.md
│   ├── Feature-Guide-English.md
│   ├── Troubleshooting.md
│   └── Troubleshooting-English.md
├── audits/
├── plans/
└── 豆包流式语音识别参考文档.md
```

### 规则

1. 面向用户的说明优先放 README。
2. Wiki 页面草稿放 `docs/wiki/`，线上 Wiki 更新时同步检查本地草稿。
3. 面向开发和 AI 的规则放 docs。
4. 发布审计记录放 `docs/audits/`，项目计划和设计草稿放 `docs/plans/`；按 Superpowers 工作流生成的可执行计划放 `docs/superpowers/plans/`。
5. 接口参考文档可以放 docs，但不要混入真实密钥。
6. 大改功能时，如果影响维护规则，应同步更新 docs。

---

## 5. 脚本目录规范

```text
scripts/
├── ai-check.ps1
├── ai-release-check.ps1
├── release-preflight.ps1
├── test-release-preflight.ps1
├── enable_git_hooks.ps1
├── rust-audit.ps1
├── scan-secrets.mjs
├── test-scan-secrets.mjs
├── check-governance.mjs
└── test-governance.mjs
```

### 规则

1. 本地检查脚本放 scripts。
2. 脚本应尽量可读，不要做过度复杂的环境探测。
3. 脚本失败应返回非零退出码。
4. 涉及发布的脚本必须避免打包本地配置、日志、统计和上下文。

---

## 6. 禁止新增的目录

除非有明确理由，不要新增：

```text
src/components/
src/common/
src/helpers/
src-tauri/src/utils.rs
src-tauri/src/common.rs
tmp/
temp/
build-output/
release-assets/
```

---

## 7. 新文件放置决策

新增文件前先判断：

```text
是用户界面组件？ → src/lib/components/
是前端纯函数？ → src/lib/utils/
是前端类型？ → src/lib/types/
是用户文案？ → src/lib/i18n/
是 Rust 桌面能力？ → src-tauri/src/对应模块
是开发规范？ → docs/
是本地脚本？ → scripts/
是用户说明？ → README.md
是 Wiki 草稿？ → docs/wiki/
是发布审计？ → docs/audits/
```
