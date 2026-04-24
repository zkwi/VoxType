<script lang="ts">
  import { onMount } from "svelte";
  import { browser } from "$app/environment";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import {
    Activity,
    Gauge,
    Keyboard,
    Maximize2,
    Mic,
    Minus,
    Save,
    Settings,
    ShieldCheck,
    X as XIcon,
  } from "lucide-svelte";

  type Section = "Overview" | "Settings" | "History";
  type Language = "zh-CN" | "zh-TW" | "en";

  type AppSnapshot = {
    hotkey: string;
  };

  type UsageStats = {
    session_count: number;
    total_seconds: number;
    total_chars: number;
    total_minutes_int: number;
    avg_chars_per_minute: number;
  };

  type DailyUsageStats = {
    day: string;
    stats: UsageStats;
  };

  type HistoryEvent = {
    created_at: string;
    duration_seconds: number;
    text_chars: number;
  };

  type StatsSnapshot = {
    path: string;
    recent_24h: UsageStats;
    recent_7d: UsageStats;
    by_day: DailyUsageStats[];
    history: HistoryEvent[];
  };

  type LoadedConfig = {
    path: string;
    exists: boolean;
    data: AppConfig;
  };

  type SessionState = {
    recording: boolean;
    message: string;
  };

  type AsrFinalText = {
    text: string;
    error: string | null;
  };

  type AsrPartialText = { text: string };
  type OverlayText = { text: string };
  type AudioDeviceInfo = { index: number; name: string };
  type OverlayMode = "single" | "double";

  type TextContext = { text: string };

  type AppConfig = {
    hotkey: string;
    auth: { app_key: string; access_key: string; resource_id: string };
    audio: {
      sample_rate: number;
      channels: number;
      segment_ms: number;
      max_record_seconds: number;
      stop_grace_ms: number;
      mute_system_volume_while_recording: boolean;
      input_device: number | null;
    };
    request: {
      ws_url: string;
      model_name: string;
      enable_nonstream: boolean;
      enable_itn: boolean;
      enable_punc: boolean;
      enable_ddc: boolean;
      show_utterances: boolean;
      result_type: string;
      enable_accelerate_text: boolean | null;
      accelerate_score: number | null;
      end_window_size: number | null;
      force_to_speech_time: number | null;
      final_result_timeout_seconds: number;
    };
    context: {
      enable_recent_context: boolean;
      recent_context_rounds: number;
      image_url: string | null;
      hotwords: string[];
      prompt_context: TextContext[];
      recent_context: TextContext[];
    };
    typing: { paste_delay_ms: number; paste_method: string };
    llm_post_edit: {
      enabled: boolean;
      min_chars: number;
      base_url: string;
      api_key: string;
      model: string;
      timeout_seconds: number;
      enable_thinking: boolean;
      system_prompt: string;
      user_prompt_template: string;
    };
    ui: {
      width: number;
      height: number;
      margin_bottom: number;
      opacity: number;
      scroll_interval_ms: number;
    };
    tray: { show_startup_message: boolean; startup_message_timeout_ms: number };
    debug: { print_transcript_to_console: boolean };
  };

  const fallbackConfig: AppConfig = {
    hotkey: "ctrl+q",
    auth: { app_key: "", access_key: "", resource_id: "volc.seedasr.sauc.duration" },
    audio: {
      sample_rate: 16000,
      channels: 1,
      segment_ms: 200,
      max_record_seconds: 300,
      stop_grace_ms: 500,
      mute_system_volume_while_recording: true,
      input_device: null,
    },
    request: {
      ws_url: "wss://openspeech.bytedance.com/api/v3/sauc/bigmodel_async",
      model_name: "bigmodel",
      enable_nonstream: true,
      enable_itn: true,
      enable_punc: true,
      enable_ddc: true,
      show_utterances: true,
      result_type: "full",
      enable_accelerate_text: false,
      accelerate_score: 0,
      end_window_size: 1200,
      force_to_speech_time: null,
      final_result_timeout_seconds: 15,
    },
    context: {
      enable_recent_context: true,
      recent_context_rounds: 5,
      image_url: null,
      hotwords: [],
      prompt_context: [],
      recent_context: [],
    },
    typing: { paste_delay_ms: 120, paste_method: "ctrl_v" },
    llm_post_edit: {
      enabled: false,
      min_chars: 40,
      base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1",
      api_key: "",
      model: "qwen3.5-plus",
      timeout_seconds: 30,
      enable_thinking: false,
      system_prompt: "",
      user_prompt_template: "{text}",
    },
    ui: { width: 350, height: 64, margin_bottom: 52, opacity: 0.9, scroll_interval_ms: 1200 },
    tray: { show_startup_message: true, startup_message_timeout_ms: 6000 },
    debug: { print_transcript_to_console: true },
  };

  const fallbackSnapshot: AppSnapshot = {
    hotkey: "ctrl+q",
  };

  const emptyStats: StatsSnapshot = {
    path: "voice_input_stats.jsonl",
    recent_24h: emptyUsage(),
    recent_7d: emptyUsage(),
    by_day: [],
    history: [],
  };
  const defaultOverlayText = "正在录音...";
  const overlayLineHeight = 1.18;
  const chineseTypingCharsPerMinute = 50;

  const navItems: { id: Section; icon: typeof Gauge }[] = [
    { id: "Overview", icon: Gauge },
    { id: "Settings", icon: Settings },
    { id: "History", icon: Activity },
  ];

  const copy = {
    "zh-CN": {
      appTitle: "声写",
      language: "语言",
      navOverview: "输入",
      navSettings: "配置",
      navHistory: "统计历史",
      topEyebrow: "VoxType",
      recordingPreview: "录音中",
      idle: "空闲",
      listeningPreview: "正在监听麦克风，实时字幕显示在屏幕下方。",
      pressHotkey: "按 {hotkey}、右 Alt 或鼠标中键，也可从托盘启动，向任意输入框语音输入。",
      setupRequired: "需要先完成配置",
      setupMissingFile: "未找到配置文件。请在配置页填写认证信息并保存，或打开配置文件手动编辑。",
      setupMissingAuth: "ASR 认证信息未填写。请在配置页填写 App Key 和 Access Key 后保存。",
      setupCta: "去配置",
      setupGuideCta: "查看配置指南",
      desktopControl: "启动方式",
      hotkey: "热键",
      recent24h: "最近 24 小时",
      sessions: "会话数",
      recent7d: "最近 7 日",
      chars: "字",
      coreSurfaces: "核心能力",
      coreDescription: "常用语音输入能力已经整合在桌面端。",
      asrProvider: "ASR 服务",
      asrProviderValue: "豆包 bigmodel_async",
      postEdit: "文本润色",
      postEditValue: "OpenAI 兼容接口",
      output: "输出方式",
      outputValue: "剪贴板 + 模拟粘贴",
      trigger: "触发方式",
      triggerValue: "右 Alt / 鼠标中键 / 托盘",
      configuration: "配置文件",
      resourceId: "资源 ID",
      appKey: "App Key",
      accessKey: "Access Key",
      audio: "音频",
      audioDescription: "原生录音器使用的 PCM 流参数。",
      sampleRate: "采样率",
      channels: "声道数",
      segmentMs: "分片毫秒",
      maxSeconds: "最长秒数",
      stopGraceMs: "尾音保留毫秒",
      inputDevice: "输入设备",
      defaultInputDevice: "默认输入设备",
      noAudioDevices: "未发现输入设备",
      systemPrompt: "System Prompt",
      userPromptTemplate: "User Prompt 模板",
      interface: "界面",
      interfaceDescription: "主窗口与悬浮字幕窗口参数。",
      width: "宽度",
      height: "高度",
      marginBottom: "底部边距",
      opacity: "透明度",
      scrollInterval: "滚动间隔毫秒",
      tray: "托盘",
      trayDescription: "托盘常驻与启动提示。",
      showStartupMessage: "启动时显示提示",
      startupTimeout: "提示停留毫秒",
      muteSystemAudio: "录音时静音系统音频",
      asrRequest: "ASR 请求",
      asrDescription: "豆包流式识别请求选项。",
      websocketUrl: "WebSocket 地址",
      model: "模型",
      finalTimeout: "最终结果超时",
      secondPass: "二遍识别",
      itn: "ITN",
      punctuation: "自动标点",
      ddc: "顺滑",
      context: "上下文",
      contextDescription: "热词、场景提示和图片上下文。",
      hotwords: "热词，每行一个",
      promptContext: "场景上下文，每行一个",
      imageUrl: "图片 URL",
      useRecentContext: "使用最近上下文",
      typing: "输入输出",
      typingDescription: "最终文本如何进入目标输入框。",
      pasteDelayMs: "粘贴延迟毫秒",
      pasteMethod: "粘贴方式",
      clipboardOnly: "仅剪贴板",
      llmPostEdit: "大模型润色",
      llmDescription: "OpenAI 兼容接口的润色回退。",
      enablePolishing: "启用润色",
      minChars: "最少字符数",
      timeout: "超时",
      saveConfig: "保存配置",
      saving: "保存中",
      reload: "重新加载",
      recentUsage: "近期使用",
      chars24h: "24 小时字数",
      chars7d: "7 日字数",
      avgCpm: "平均字/分钟",
      weeklySavedHours: "每周节省",
      weeklySavedHoursHint: "按中文输入约 50 字/分钟估算。",
      byDay: "按日统计",
      lastSevenDays: "最近七个自然日。",
      recentRecords: "最近记录",
      historyDescription: "最近保存的输入记录。",
      noUsageRecords: "暂无使用记录。",
      bridgeLoading: "正在准备...",
      browserPreview: "请在桌面应用中使用。",
      bridgeConnected: "准备就绪。",
      configSaved: "保存成功。",
      previewRecording: "录音中。",
      previewStopped: "等待快捷键。",
      startupToastTitle: "声写已启动",
      startupToastHint: "{hotkey} / 右 Alt / 鼠标中键",
    },
    "zh-TW": {
      appTitle: "聲寫",
      language: "語言",
      navOverview: "輸入",
      navSettings: "配置",
      navHistory: "統計歷史",
      topEyebrow: "VoxType",
      recordingPreview: "錄音中",
      idle: "閒置",
      listeningPreview: "正在監聽麥克風，即時字幕顯示在螢幕下方。",
      pressHotkey: "按 {hotkey}、右 Alt 或滑鼠中鍵，也可從系統匣啟動，向任意輸入框語音輸入。",
      setupRequired: "需要先完成配置",
      setupMissingFile: "未找到配置檔案。請在配置頁填寫認證資訊並儲存，或打開配置檔案手動編輯。",
      setupMissingAuth: "ASR 認證資訊未填寫。請在配置頁填寫 App Key 和 Access Key 後儲存。",
      setupCta: "去配置",
      setupGuideCta: "查看配置指南",
      desktopControl: "啟動方式",
      hotkey: "快捷鍵",
      recent24h: "最近 24 小時",
      sessions: "會話數",
      recent7d: "最近 7 日",
      chars: "字",
      coreSurfaces: "核心能力",
      coreDescription: "常用語音輸入能力已整合在桌面端。",
      asrProvider: "ASR 服務",
      asrProviderValue: "豆包 bigmodel_async",
      postEdit: "文字潤飾",
      postEditValue: "OpenAI 相容介面",
      output: "輸出方式",
      outputValue: "剪貼簿 + 模擬貼上",
      trigger: "觸發方式",
      triggerValue: "右 Alt / 滑鼠中鍵 / 系統匣",
      configuration: "配置檔案",
      resourceId: "資源 ID",
      appKey: "App Key",
      accessKey: "Access Key",
      audio: "音訊",
      audioDescription: "原生錄音器使用的 PCM 流參數。",
      sampleRate: "取樣率",
      channels: "聲道數",
      segmentMs: "分片毫秒",
      maxSeconds: "最長秒數",
      stopGraceMs: "尾音保留毫秒",
      inputDevice: "輸入裝置",
      defaultInputDevice: "預設輸入裝置",
      noAudioDevices: "未發現輸入裝置",
      systemPrompt: "System Prompt",
      userPromptTemplate: "User Prompt 模板",
      interface: "介面",
      interfaceDescription: "主視窗與懸浮字幕視窗參數。",
      width: "寬度",
      height: "高度",
      marginBottom: "底部邊距",
      opacity: "透明度",
      scrollInterval: "滾動間隔毫秒",
      tray: "系統匣",
      trayDescription: "系統匣常駐與啟動提示。",
      showStartupMessage: "啟動時顯示提示",
      startupTimeout: "提示停留毫秒",
      muteSystemAudio: "錄音時靜音系統音訊",
      asrRequest: "ASR 請求",
      asrDescription: "豆包串流識別請求選項。",
      websocketUrl: "WebSocket 位址",
      model: "模型",
      finalTimeout: "最終結果逾時",
      secondPass: "二遍識別",
      itn: "ITN",
      punctuation: "自動標點",
      ddc: "順滑",
      context: "上下文",
      contextDescription: "熱詞、場景提示和圖片上下文。",
      hotwords: "熱詞，每行一個",
      promptContext: "場景上下文，每行一個",
      imageUrl: "圖片 URL",
      useRecentContext: "使用最近上下文",
      typing: "輸入輸出",
      typingDescription: "最終文字如何進入目標輸入框。",
      pasteDelayMs: "貼上延遲毫秒",
      pasteMethod: "貼上方式",
      clipboardOnly: "僅剪貼簿",
      llmPostEdit: "大模型潤飾",
      llmDescription: "OpenAI 相容介面的潤飾回退。",
      enablePolishing: "啟用潤飾",
      minChars: "最少字元數",
      timeout: "逾時",
      saveConfig: "儲存配置",
      saving: "儲存中",
      reload: "重新載入",
      recentUsage: "近期使用",
      chars24h: "24 小時字數",
      chars7d: "7 日字數",
      avgCpm: "平均字/分鐘",
      weeklySavedHours: "每週節省",
      weeklySavedHoursHint: "按中文輸入約 50 字/分鐘估算。",
      byDay: "按日統計",
      lastSevenDays: "最近七個自然日。",
      recentRecords: "最近記錄",
      historyDescription: "最近保存的輸入記錄。",
      noUsageRecords: "暫無使用記錄。",
      bridgeLoading: "正在準備...",
      browserPreview: "請在桌面應用中使用。",
      bridgeConnected: "準備就緒。",
      configSaved: "儲存成功。",
      previewRecording: "錄音中。",
      previewStopped: "等待快捷鍵。",
      startupToastTitle: "聲寫已啟動",
      startupToastHint: "{hotkey} / 右 Alt / 滑鼠中鍵",
    },
    en: {
      appTitle: "VoxType",
      language: "Language",
      navOverview: "Input",
      navSettings: "Settings",
      navHistory: "Stats",
      topEyebrow: "VoxType",
      recordingPreview: "Recording",
      idle: "Idle",
      listeningPreview: "Listening to the microphone. Live captions appear near the bottom of the screen.",
      pressHotkey: "Press {hotkey}, Right Alt, or the middle mouse button, or start from the tray.",
      setupRequired: "Setup required",
      setupMissingFile: "No config file found. Fill credentials on the Settings page and save, or open the config file manually.",
      setupMissingAuth: "ASR credentials are missing. Fill App Key and Access Key on the Settings page, then save.",
      setupCta: "Open Settings",
      setupGuideCta: "Setup Guide",
      desktopControl: "Start methods",
      hotkey: "Hotkey",
      recent24h: "Recent 24h",
      sessions: "Sessions",
      recent7d: "Recent 7d",
      chars: "chars",
      coreSurfaces: "Core surfaces",
      coreDescription: "Core voice input capabilities are available in the desktop app.",
      asrProvider: "ASR provider",
      asrProviderValue: "Doubao bigmodel_async",
      postEdit: "Post edit",
      postEditValue: "OpenAI-compatible endpoint",
      output: "Output",
      outputValue: "Clipboard + simulated paste",
      trigger: "Trigger",
      triggerValue: "Right Alt / middle mouse / tray",
      configuration: "Configuration",
      resourceId: "Resource ID",
      appKey: "App Key",
      accessKey: "Access Key",
      audio: "Audio",
      audioDescription: "PCM stream settings for the native recorder.",
      sampleRate: "Sample rate",
      channels: "Channels",
      segmentMs: "Segment ms",
      maxSeconds: "Max seconds",
      stopGraceMs: "Stop grace ms",
      inputDevice: "Input device",
      defaultInputDevice: "Default input device",
      noAudioDevices: "No input devices found",
      systemPrompt: "System Prompt",
      userPromptTemplate: "User prompt template",
      interface: "Interface",
      interfaceDescription: "Main window and floating caption settings.",
      width: "Width",
      height: "Height",
      marginBottom: "Bottom margin",
      opacity: "Opacity",
      scrollInterval: "Scroll interval ms",
      tray: "Tray",
      trayDescription: "Tray residency and startup hint.",
      showStartupMessage: "Show startup hint",
      startupTimeout: "Hint timeout ms",
      muteSystemAudio: "Mute system audio while recording",
      asrRequest: "ASR Request",
      asrDescription: "Doubao streaming recognition options.",
      websocketUrl: "WebSocket URL",
      model: "Model",
      finalTimeout: "Final timeout",
      secondPass: "Second pass",
      itn: "ITN",
      punctuation: "Punctuation",
      ddc: "DDC",
      context: "Context",
      contextDescription: "Hotwords, scene hints, and image context.",
      hotwords: "Hotwords, one per line",
      promptContext: "Prompt context, one per line",
      imageUrl: "Image URL",
      useRecentContext: "Use recent context",
      typing: "Typing",
      typingDescription: "How final text reaches the target input box.",
      pasteDelayMs: "Paste delay ms",
      pasteMethod: "Paste method",
      clipboardOnly: "Clipboard only",
      llmPostEdit: "LLM Post Edit",
      llmDescription: "OpenAI-compatible polishing fallback.",
      enablePolishing: "Enable polishing",
      minChars: "Min chars",
      timeout: "Timeout",
      saveConfig: "Save config",
      saving: "Saving",
      reload: "Reload",
      recentUsage: "Recent usage",
      chars24h: "24h chars",
      chars7d: "7d chars",
      avgCpm: "Avg cpm",
      weeklySavedHours: "Weekly saved",
      weeklySavedHoursHint: "Estimated at 50 Chinese chars/min.",
      byDay: "By day",
      lastSevenDays: "Last seven calendar days.",
      recentRecords: "Recent records",
      historyDescription: "Recently saved dictation records.",
      noUsageRecords: "No usage records yet.",
      bridgeLoading: "Preparing...",
      browserPreview: "Open the desktop app to use this feature.",
      bridgeConnected: "Ready.",
      configSaved: "Saved.",
      previewRecording: "Recording.",
      previewStopped: "Waiting for shortcut.",
      startupToastTitle: "VoxType is running",
      startupToastHint: "{hotkey} / Right Alt / middle mouse",
    },
  };

  type CopyKey = keyof typeof copy["zh-CN"];

  const navLabelKeys: Record<Section, CopyKey> = {
    Overview: "navOverview",
    Settings: "navSettings",
    History: "navHistory",
  };

  let measureCanvas: HTMLCanvasElement | undefined;
  let snapshot = $state<AppSnapshot>(fallbackSnapshot);
  let config = $state<AppConfig>(structuredClone(fallbackConfig));
  let stats = $state<StatsSnapshot>(emptyStats);
  let recording = $state(false);
  let language = $state<Language>("zh-CN");
  let statusMessage = $state(copy["zh-CN"].bridgeLoading);
  let selectedSection = $state<Section>("Overview");
  let saving = $state(false);
  let configExists = $state(true);
  const initialParams = browser ? new URLSearchParams(window.location.search) : new URLSearchParams();
  let audioDevices = $state<AudioDeviceInfo[]>([]);
  let isOverlay = $state(initialParams.has("overlay"));
  let isToast = $state(initialParams.has("toast"));
  let toastHotkey = $state(initialParams.get("hotkey") || "Ctrl+Q");
  let overlayText = $state(defaultOverlayText);
  let overlayMode = $state<OverlayMode>("single");
  let overlayFontSize = $state(20);
  let overlayLineLimit = $state(1);
  let overlayDisplayLines = $state<string[]>([defaultOverlayText]);
  let overlayTextElement = $state<HTMLDivElement | null>(null);
  let overlayAllLines: string[] = [];
  let overlayScrollOffset = 0;
  let overlayTailHoldSteps = 0;
  let overlayScrollTimer: number | undefined;
  let overlayPollPending = false;
  let overlaySmallLayoutLocked = false;

  onMount(() => {
    const params = new URLSearchParams(window.location.search);
    isOverlay = params.has("overlay");
    isToast = params.has("toast");
    toastHotkey = params.get("hotkey") || toastHotkey;
    const savedLanguage = localStorage.getItem("voxtype-language");
    if (savedLanguage === "zh-CN" || savedLanguage === "zh-TW" || savedLanguage === "en") {
      language = savedLanguage;
      statusMessage = t("bridgeLoading");
    }
    void loadAll();
    void hydrateSession();
    let overlayPoll: number | undefined;
    if (isOverlay) {
      applyOverlayText(overlayText, true);
      window.addEventListener("resize", refreshOverlayLayout);
      overlayPoll = window.setInterval(() => {
        void refreshOverlayText();
      }, 80);
    }
    const unlistenSession = listen<SessionState>("session-state-changed", (event) => {
      applySessionState(event.payload);
    });
    const unlistenAsr = listen<AsrFinalText>("asr-final-text", (event) => {
      if (event.payload.error) {
        statusMessage = event.payload.error;
        if (isConfigError(event.payload.error)) selectedSection = "Settings";
        return;
      }
      if (isOverlay && event.payload.text.trim()) {
        applyOverlayText(event.payload.text);
      }
      statusMessage = t("previewStopped");
    });
    const unlistenPartial = listen<AsrPartialText>("asr-partial-text", (event) => {
      if (event.payload.text.trim()) {
        if (isOverlay) {
          applyOverlayText(event.payload.text);
        }
      }
    });
    const unlistenOverlay = listen<OverlayText>("overlay-text", (event) => {
      applyOverlayText(event.payload.text || defaultOverlayText);
    });
    const unlistenStats = listen<StatsSnapshot>("usage-stats-updated", (event) => {
      if (!isOverlay && !isToast) stats = event.payload;
    });
    return () => {
      if (overlayPoll !== undefined) window.clearInterval(overlayPoll);
      stopOverlayScroll();
      window.removeEventListener("resize", refreshOverlayLayout);
      void Promise.all([unlistenSession, unlistenAsr, unlistenPartial, unlistenOverlay, unlistenStats]).then((disposers) => {
        for (const dispose of disposers) dispose();
      });
    };
  });

  function t(key: CopyKey, values: Record<string, string> = {}) {
    let value = copy[language][key];
    for (const [name, replacement] of Object.entries(values)) {
      value = value.replace(`{${name}}`, replacement);
    }
    return value;
  }

  function setLanguage(value: string) {
    if (value !== "zh-CN" && value !== "zh-TW" && value !== "en") return;
    language = value;
    localStorage.setItem("voxtype-language", value);
    if (
      statusMessage === copy["zh-CN"].bridgeLoading ||
      statusMessage === copy["zh-TW"].bridgeLoading ||
      statusMessage === copy.en.bridgeLoading
    ) {
      statusMessage = t("bridgeLoading");
    }
  }

  function emptyUsage(): UsageStats {
    return {
      session_count: 0,
      total_seconds: 0,
      total_chars: 0,
      total_minutes_int: 0,
      avg_chars_per_minute: 0,
    };
  }

  async function safeInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T | null> {
    try {
      return await invoke<T>(command, args);
    } catch (error) {
      statusMessage = typeof error === "string" ? error : t("browserPreview");
      console.warn(error);
      return null;
    }
  }

  async function refreshOverlayText() {
    if (overlayPollPending) return;
    overlayPollPending = true;
    try {
      const result = await safeInvoke<OverlayText>("get_overlay_text");
      const text = result?.text ?? "";
      if (text.trim()) applyOverlayText(text);
    } finally {
      overlayPollPending = false;
    }
  }

  function refreshOverlayLayout() {
    if (isOverlay) applyOverlayText(overlayText, true);
  }

  function applyOverlayText(rawText: string, force = false) {
    const normalized = normalizeOverlayText(rawText);
    if (!force && normalized === overlayText) return;
    overlayText = normalized;

    if (normalized === defaultOverlayText) {
      overlaySmallLayoutLocked = false;
    }

    const { mode, fontSize, lineLimit } = resolveOverlayLayout(normalized, overlaySmallLayoutLocked);
    if (mode === "double" && normalized !== defaultOverlayText) {
      overlaySmallLayoutLocked = true;
    }
    overlayMode = mode;
    overlayFontSize = fontSize;
    overlayLineLimit = lineLimit;
    overlayAllLines = wrapOverlayText(normalized, fontSize);
    const visibleCount = overlayVisibleLineCount();
    overlayScrollOffset = Math.max(0, overlayAllLines.length - visibleCount);
    overlayTailHoldSteps = overlayAllLines.length > visibleCount ? 2 : 1;
    refreshVisibleOverlayLines();
  }

  function normalizeOverlayText(text: string) {
    const raw = String(text || "").replace(/\r\n/g, "\n").replace(/\r/g, "\n").trim();
    if (!raw) return defaultOverlayText;

    const lines: string[] = [];
    let blankPending = false;
    for (const line of raw.split("\n")) {
      const cleaned = line.trim();
      if (!cleaned) {
        if (!blankPending && lines.length > 0) lines.push("");
        blankPending = true;
        continue;
      }
      lines.push(cleaned);
      blankPending = false;
    }
    return lines.join("\n") || defaultOverlayText;
  }

  function resolveOverlayLayout(
    text: string,
    forceSmall: boolean,
  ): { mode: OverlayMode; fontSize: number; lineLimit: number } {
    const compactLength = Array.from(text.replace(/\s/g, "")).length;
    const singleFont = fontForVisibleLines(1, 20, 18);
    const doubleFont = fontForVisibleLines(2, 16, 14);

    if (!forceSmall && wrapOverlayText(text, singleFont).length <= 1 && compactLength <= 18) {
      return { mode: "single", fontSize: singleFont, lineLimit: 1 };
    }
    return { mode: "double", fontSize: doubleFont, lineLimit: 2 };
  }

  function fontForVisibleLines(lines: number, preferred: number, min: number) {
    const availableHeight = overlayAvailableTextHeight();
    const fitted = Math.floor((availableHeight - 2) / (lines * overlayLineHeight));
    return Math.max(min, Math.min(preferred, fitted));
  }

  function wrapOverlayText(text: string, fontSize: number) {
    const maxWidth = Math.max(80, overlayTextElement?.clientWidth ?? window.innerWidth - 88);
    const lines: string[] = [];

    for (const paragraph of text.split("\n")) {
      if (!paragraph) {
        lines.push("");
        continue;
      }
      let current = "";
      for (const char of Array.from(paragraph)) {
        const candidate = current + char;
        if (current && measureOverlayText(candidate, fontSize) > maxWidth) {
          lines.push(current);
          current = char;
        } else {
          current = candidate;
        }
      }
      if (current) lines.push(current);
    }
    return lines.length ? lines : [text];
  }

  function measureOverlayText(text: string, fontSize: number) {
    measureCanvas ??= document.createElement("canvas");
    const context = measureCanvas.getContext("2d");
    if (!context) return Array.from(text).length * fontSize;
    context.font = `400 ${fontSize}px "Segoe UI", "Microsoft YaHei", sans-serif`;
    return context.measureText(text).width;
  }

  function overlayVisibleLineCount() {
    return Math.max(1, Math.min(2, overlayLineLimit));
  }

  function overlayAvailableTextHeight() {
    return Math.max(1, window.innerHeight - 24);
  }

  function refreshVisibleOverlayLines() {
    const visibleCount = overlayVisibleLineCount();
    if (overlayAllLines.length <= visibleCount) {
      stopOverlayScroll();
      overlayDisplayLines = overlayAllLines;
      return;
    }

    const end = overlayScrollOffset + visibleCount;
    overlayDisplayLines = overlayAllLines.slice(overlayScrollOffset, end);
    startOverlayScroll();
  }

  function startOverlayScroll() {
    if (overlayScrollTimer !== undefined) return;
    const intervalMs = Math.max(300, config.ui.scroll_interval_ms || 1200);
    overlayScrollTimer = window.setInterval(advanceOverlayScroll, intervalMs);
  }

  function stopOverlayScroll() {
    if (overlayScrollTimer !== undefined) {
      window.clearInterval(overlayScrollTimer);
      overlayScrollTimer = undefined;
    }
  }

  function advanceOverlayScroll() {
    const visibleCount = overlayVisibleLineCount();
    if (overlayAllLines.length <= visibleCount) {
      stopOverlayScroll();
      return;
    }
    if (overlayTailHoldSteps > 0) {
      overlayTailHoldSteps -= 1;
      return;
    }
    if (overlayScrollOffset <= 0) {
      stopOverlayScroll();
      return;
    }

    overlayScrollOffset -= 1;
    overlayDisplayLines = overlayAllLines.slice(overlayScrollOffset, overlayScrollOffset + visibleCount);
  }

  async function loadAll() {
    const [snapshotResult, configResult, statsResult, devicesResult] = await Promise.all([
      safeInvoke<AppSnapshot>("get_app_snapshot"),
      safeInvoke<LoadedConfig>("load_app_config"),
      safeInvoke<StatsSnapshot>("get_usage_stats"),
      safeInvoke<AudioDeviceInfo[]>("list_audio_input_devices"),
    ]);
    if (snapshotResult) snapshot = snapshotResult;
    if (configResult) {
      config = configResult.data;
      configExists = configResult.exists;
      const setupMessage = configSetupMessage(configResult);
      if (setupMessage) {
        statusMessage = setupMessage;
        selectedSection = "Settings";
      }
    }
    if (statsResult) stats = statsResult;
    if (devicesResult) audioDevices = devicesResult;
    if ((snapshotResult || configResult || statsResult) && !configSetupMessage(configResult)) {
      statusMessage = t("bridgeConnected");
    }
  }

  async function hydrateSession() {
    const result = await safeInvoke<SessionState>("get_session_state");
    if (result) applySessionState(result);
  }

  function applySessionState(state: SessionState) {
    recording = state.recording;
    if (isConfigError(state.message)) {
      statusMessage = state.message;
      selectedSection = "Settings";
      return;
    }
    statusMessage = state.recording ? t("previewRecording") : t("previewStopped");
  }

  async function saveConfig() {
    saving = true;
    const result = await safeInvoke<LoadedConfig>("save_app_config", { config });
    if (result) {
      config = result.data;
      configExists = result.exists;
      statusMessage = t("configSaved");
      await loadAll();
    }
    saving = false;
  }

  function formatHotkey(value: string) {
    return value
      .split("+")
      .map((part) => part.trim().toUpperCase())
      .filter(Boolean)
      .join("+");
  }

  function setHotkey(value: string) {
    config.hotkey = formatHotkey(value);
  }

  async function minimizeWindow() {
    try {
      await getCurrentWindow().minimize();
    } catch (error) {
      console.warn(error);
    }
  }

  async function toggleMaximizeWindow() {
    try {
      await getCurrentWindow().toggleMaximize();
    } catch (error) {
      console.warn(error);
    }
  }

  async function closeWindow() {
    try {
      await getCurrentWindow().close();
    } catch (error) {
      console.warn(error);
    }
  }

  function updateHotwords(value: string) {
    config.context.hotwords = value
      .split("\n")
      .map((item) => item.trim())
      .filter(Boolean);
  }

  function updatePromptContext(value: string) {
    config.context.prompt_context = value
      .split("\n")
      .map((text) => text.trim())
      .filter(Boolean)
      .map((text) => ({ text }));
  }

  function setOptionalImageUrl(value: string) {
    const trimmed = value.trim();
    config.context.image_url = trimmed ? trimmed : null;
  }

  function setInputDevice(value: string | number | null) {
    if (value === null || value === "") {
      config.audio.input_device = null;
      return;
    }
    config.audio.input_device = Number(value);
  }

  function formatSeconds(seconds: number) {
    if (seconds < 60) return `${seconds.toFixed(1)}s`;
    return `${Math.floor(seconds / 60)}m ${Math.round(seconds % 60)}s`;
  }

  function weeklySavedHours() {
    const typingHours = stats.recent_7d.total_chars / chineseTypingCharsPerMinute / 60;
    const recordingHours = stats.recent_7d.total_seconds / 3600;
    return Math.max(0, typingHours - recordingHours);
  }

  function formatHours(hours: number) {
    if (hours < 0.05) return "0 h";
    return `${hours.toFixed(1)} h`;
  }

  function hasAuth(configValue = config) {
    return Boolean(configValue.auth.app_key.trim() && configValue.auth.access_key.trim());
  }

  function configSetupMessage(loaded: LoadedConfig | null) {
    if (!loaded) return "";
    if (!loaded.exists) return t("setupMissingFile");
    if (!hasAuth(loaded.data)) return t("setupMissingAuth");
    return "";
  }

  function isConfigError(message: string) {
    return (
      message.includes("ASR 未配置") ||
      message.includes("config.toml") ||
      message.includes("app_key") ||
      message.includes("access_key")
    );
  }

  function openSettings() {
    selectedSection = "Settings";
  }

  async function openSetupGuide() {
    await safeInvoke<void>("open_setup_guide");
  }
</script>

<svelte:head>
  <title>VoxType</title>
</svelte:head>

{#if isOverlay}
  <main class="overlay-root">
    <div class="overlay-caption">
      <span class="overlay-dot"></span>
      <div
        class:single={overlayMode === "single"}
        class:double={overlayMode === "double"}
        class="overlay-caption-text"
        style={`font-size: ${overlayFontSize}px`}
        bind:this={overlayTextElement}
      >
        {#each overlayDisplayLines as line}
          <span>{line || "\u00a0"}</span>
        {/each}
      </div>
    </div>
  </main>
{:else if isToast}
  <main class="toast-root">
    <section class="toast-card">
      <div class="toast-icon"><Mic size={18} /></div>
      <div class="toast-copy">
        <strong>{t("startupToastTitle")}</strong>
        <span>{t("startupToastHint").replace("{hotkey}", formatHotkey(toastHotkey))}</span>
      </div>
    </section>
  </main>
{:else}
<div class="app-frame">
<header class="window-titlebar">
  <div class="window-title" data-tauri-drag-region>
    <span class="window-title-mark"><Mic size={12} strokeWidth={2.6} /></span>
    <span data-tauri-drag-region>VoxType</span>
  </div>
  <div class="window-controls">
    <button aria-label="最小化" title="最小化" onclick={minimizeWindow}><Minus size={13} /></button>
    <button aria-label="最大化或还原" title="最大化或还原" onclick={toggleMaximizeWindow}><Maximize2 size={12} /></button>
    <button class="close" aria-label="关闭" title="关闭" onclick={closeWindow}><XIcon size={14} /></button>
  </div>
</header>
<main class="shell">
  <aside class="sidebar">
    <div class="brand">
      <div class="brand-mark"><Mic size={22} /></div>
      <div>
        <p class="eyebrow">VoxType</p>
        <h1>{t("appTitle")}</h1>
      </div>
    </div>

    <nav aria-label="Main sections">
      {#each navItems as item}
        {@const Icon = item.icon}
        <button class:active={selectedSection === item.id} onclick={() => (selectedSection = item.id)}>
          <Icon size={17} />
          <span>{t(navLabelKeys[item.id])}</span>
        </button>
      {/each}
    </nav>

    <label class="language-control">
      <span>{t("language")}</span>
      <select value={language} onchange={(event) => setLanguage(event.currentTarget.value)}>
        <option value="zh-CN">简体中文</option>
        <option value="zh-TW">繁體中文</option>
        <option value="en">English</option>
      </select>
    </label>

    <section class="bridge-card">
      <div class="bridge-top">
        <span class="pulse" class:recording></span>
        <span>{recording ? t("recordingPreview") : t("idle")}</span>
      </div>
      <p>{statusMessage}</p>
    </section>
  </aside>

  <section class="content">
    <header class="topbar">
      <div>
        <p class="eyebrow">{t("topEyebrow")}</p>
        <h2>{selectedSection === "Overview" ? t("navOverview") : t(navLabelKeys[selectedSection])}</h2>
      </div>
    </header>

    {#if selectedSection === "Overview"}
      <div class="hero-grid">
        <section class="live-panel">
          {#if !configExists || !hasAuth()}
            <div class="setup-alert">
              <div>
                <strong>{t("setupRequired")}</strong>
                <p>{!configExists ? t("setupMissingFile") : t("setupMissingAuth")}</p>
              </div>
              <div class="setup-actions">
                <button onclick={openSettings}>{t("setupCta")}</button>
                <button class="secondary" onclick={openSetupGuide}>{t("setupGuideCta")}</button>
              </div>
            </div>
          {/if}
          <div class="caption-window">
            <div class="caption-meta">
              <span class="dot" class:recording></span>
              <span>{recording ? t("recordingPreview") : t("idle")}</span>
            </div>
            <p>
              {recording
                ? t("listeningPreview")
                : t("pressHotkey", { hotkey: formatHotkey(snapshot.hotkey) })}
            </p>
          </div>
        </section>

        <section class="control-panel">
          <div class="panel-title">
            <Keyboard size={18} />
            <span>{t("desktopControl")}</span>
          </div>
          <dl class="control-grid">
            <div class="control-inline">
              <div>
                <dt>{t("hotkey")}</dt>
                <dd>{formatHotkey(snapshot.hotkey)}</dd>
              </div>
              <div>
                <dt>{t("trigger")}</dt>
                <dd>{t("triggerValue")}</dd>
              </div>
            </div>
          </dl>
        </section>
      </div>

      <section class="stats-row" aria-label="Usage summary">
        <article class="stat green"><span>{t("recent24h")}</span><strong>{stats.recent_24h.total_chars} {t("chars")}</strong></article>
        <article class="stat amber"><span>{t("recent7d")}</span><strong>{stats.recent_7d.total_chars} {t("chars")}</strong></article>
        <article class="stat blue"><span>{t("avgCpm")}</span><strong>{stats.recent_7d.avg_chars_per_minute.toFixed(1)}</strong></article>
        <article class="stat blue">
          <span>{t("weeklySavedHours")}</span>
          <strong>{formatHours(weeklySavedHours())}</strong>
          <small>{t("weeklySavedHoursHint")}</small>
        </article>
      </section>
    {:else if selectedSection === "Settings"}
      <section class="page-grid">
        <div class="form-panel wide">
          <div class="section-heading">
            <h3>{t("configuration")}</h3>
            {#if !configExists || !hasAuth()}
              <p class="setup-note">{!configExists ? t("setupMissingFile") : t("setupMissingAuth")}</p>
              <button class="link-button" onclick={openSetupGuide}>{t("setupGuideCta")}</button>
            {/if}
          </div>
          <div class="form-grid">
            <label><span>{t("hotkey")}</span><input value={formatHotkey(config.hotkey)} oninput={(event) => setHotkey(event.currentTarget.value)} /></label>
            <label><span>{t("resourceId")}</span><input bind:value={config.auth.resource_id} /></label>
            <label><span>{t("appKey")}</span><input bind:value={config.auth.app_key} /></label>
            <label><span>{t("accessKey")}</span><input bind:value={config.auth.access_key} /></label>
          </div>
        </div>

        <div class="form-panel">
          <div class="section-heading"><h3>{t("audio")}</h3><p>{t("audioDescription")}</p></div>
          <div class="form-grid compact">
            <label><span>{t("sampleRate")}</span><input type="number" bind:value={config.audio.sample_rate} /></label>
            <label><span>{t("channels")}</span><input type="number" bind:value={config.audio.channels} /></label>
            <label><span>{t("segmentMs")}</span><input type="number" bind:value={config.audio.segment_ms} /></label>
            <label><span>{t("maxSeconds")}</span><input type="number" bind:value={config.audio.max_record_seconds} /></label>
            <label><span>{t("stopGraceMs")}</span><input type="number" bind:value={config.audio.stop_grace_ms} /></label>
            <label>
              <span>{t("inputDevice")}</span>
              <select value={config.audio.input_device ?? ""} onchange={(event) => setInputDevice(event.currentTarget.value)}>
                <option value="">{t("defaultInputDevice")}</option>
                {#if audioDevices.length === 0}
                  <option value="" disabled>{t("noAudioDevices")}</option>
                {/if}
                {#each audioDevices as device}
                  <option value={device.index}>{device.index}: {device.name}</option>
                {/each}
              </select>
            </label>
          </div>
          <label class="check"><input type="checkbox" bind:checked={config.audio.mute_system_volume_while_recording} />{t("muteSystemAudio")}</label>
        </div>

        <div class="form-panel">
          <div class="section-heading"><h3>{t("asrRequest")}</h3><p>{t("asrDescription")}</p></div>
          <label><span>{t("websocketUrl")}</span><input bind:value={config.request.ws_url} /></label>
          <div class="form-grid compact">
            <label><span>{t("model")}</span><input bind:value={config.request.model_name} /></label>
            <label><span>{t("finalTimeout")}</span><input type="number" bind:value={config.request.final_result_timeout_seconds} /></label>
          </div>
          <div class="toggle-grid">
            <label class="check"><input type="checkbox" bind:checked={config.request.enable_nonstream} />{t("secondPass")}</label>
            <label class="check"><input type="checkbox" bind:checked={config.request.enable_itn} />{t("itn")}</label>
            <label class="check"><input type="checkbox" bind:checked={config.request.enable_punc} />{t("punctuation")}</label>
            <label class="check"><input type="checkbox" bind:checked={config.request.enable_ddc} />{t("ddc")}</label>
          </div>
        </div>

        <div class="form-panel">
          <div class="section-heading"><h3>{t("context")}</h3><p>{t("contextDescription")}</p></div>
          <label><span>{t("hotwords")}</span><textarea value={config.context.hotwords.join("\n")} oninput={(event) => updateHotwords(event.currentTarget.value)}></textarea></label>
          <label><span>{t("promptContext")}</span><textarea value={config.context.prompt_context.map((item) => item.text).join("\n")} oninput={(event) => updatePromptContext(event.currentTarget.value)}></textarea></label>
          <label><span>{t("imageUrl")}</span><input value={config.context.image_url ?? ""} oninput={(event) => setOptionalImageUrl(event.currentTarget.value)} /></label>
          <label class="check"><input type="checkbox" bind:checked={config.context.enable_recent_context} />{t("useRecentContext")}</label>
        </div>

        <div class="form-panel">
          <div class="section-heading"><h3>{t("typing")}</h3><p>{t("typingDescription")}</p></div>
          <div class="form-grid compact">
            <label><span>{t("pasteDelayMs")}</span><input type="number" bind:value={config.typing.paste_delay_ms} /></label>
            <label><span>{t("pasteMethod")}</span><select bind:value={config.typing.paste_method}><option value="ctrl_v">Ctrl+V</option><option value="shift_insert">Shift+Insert</option><option value="clipboard_only">{t("clipboardOnly")}</option></select></label>
          </div>
        </div>

        <div class="form-panel">
          <div class="section-heading"><h3>{t("llmPostEdit")}</h3><p>{t("llmDescription")}</p></div>
          <label class="check"><input type="checkbox" bind:checked={config.llm_post_edit.enabled} />{t("enablePolishing")}</label>
          <div class="form-grid compact">
            <label><span>{t("minChars")}</span><input type="number" bind:value={config.llm_post_edit.min_chars} /></label>
            <label><span>{t("timeout")}</span><input type="number" bind:value={config.llm_post_edit.timeout_seconds} /></label>
          </div>
          <label><span>Base URL</span><input bind:value={config.llm_post_edit.base_url} /></label>
          <label><span>{t("model")}</span><input bind:value={config.llm_post_edit.model} /></label>
          <label><span>API Key</span><input bind:value={config.llm_post_edit.api_key} /></label>
          <label><span>{t("systemPrompt")}</span><textarea bind:value={config.llm_post_edit.system_prompt}></textarea></label>
          <label><span>{t("userPromptTemplate")}</span><textarea bind:value={config.llm_post_edit.user_prompt_template}></textarea></label>
        </div>

        <div class="form-panel">
          <div class="section-heading"><h3>{t("interface")}</h3><p>{t("interfaceDescription")}</p></div>
          <div class="form-grid compact">
            <label><span>{t("width")}</span><input type="number" bind:value={config.ui.width} /></label>
            <label><span>{t("height")}</span><input type="number" bind:value={config.ui.height} /></label>
            <label><span>{t("marginBottom")}</span><input type="number" bind:value={config.ui.margin_bottom} /></label>
            <label><span>{t("opacity")}</span><input type="number" step="0.05" bind:value={config.ui.opacity} /></label>
            <label><span>{t("scrollInterval")}</span><input type="number" bind:value={config.ui.scroll_interval_ms} /></label>
          </div>
        </div>

        <div class="form-panel">
          <div class="section-heading"><h3>{t("tray")}</h3><p>{t("trayDescription")}</p></div>
          <label class="check"><input type="checkbox" bind:checked={config.tray.show_startup_message} />{t("showStartupMessage")}</label>
          <label><span>{t("startupTimeout")}</span><input type="number" bind:value={config.tray.startup_message_timeout_ms} /></label>
        </div>

        <div class="form-actions">
          <button class="primary" onclick={saveConfig} disabled={saving}><Save size={16} />{saving ? t("saving") : t("saveConfig")}</button>
          <button onclick={loadAll}><ShieldCheck size={16} />{t("reload")}</button>
        </div>
      </section>
    {:else if selectedSection === "History"}
      <section class="page-grid">
        <div class="form-panel">
          <div class="section-heading"><h3>{t("recentUsage")}</h3><p>{t("lastSevenDays")}</p></div>
          <div class="stats-row nested">
            <article class="stat green"><span>{t("chars24h")}</span><strong>{stats.recent_24h.total_chars}</strong></article>
            <article class="stat blue"><span>{t("chars7d")}</span><strong>{stats.recent_7d.total_chars}</strong></article>
            <article class="stat amber"><span>{t("avgCpm")}</span><strong>{stats.recent_7d.avg_chars_per_minute.toFixed(1)}</strong></article>
            <article class="stat blue saved-time">
              <span>{t("weeklySavedHours")}</span>
              <strong>{formatHours(weeklySavedHours())}</strong>
              <small>{t("weeklySavedHoursHint")}</small>
            </article>
          </div>
        </div>

        <div class="form-panel">
          <div class="section-heading"><h3>{t("byDay")}</h3><p>{t("lastSevenDays")}</p></div>
          <div class="day-list">
            {#each stats.by_day as day}
              <div><span>{day.day}</span><strong>{day.stats.total_chars} {t("chars")}</strong></div>
            {/each}
          </div>
        </div>

        <div class="form-panel wide">
          <div class="section-heading"><h3>{t("recentRecords")}</h3><p>{t("historyDescription")}</p></div>
          <div class="history-list">
            {#if stats.history.length === 0}
              <p class="empty">{t("noUsageRecords")}</p>
            {:else}
              {#each stats.history as event}
                <article>
                  <span>{event.created_at}</span>
                  <strong>{event.text_chars} {t("chars")}</strong>
                  <em>{formatSeconds(event.duration_seconds)}</em>
                </article>
              {/each}
            {/if}
          </div>
        </div>
      </section>
    {/if}
  </section>
</main>
</div>
{/if}

<style>
  :global(:root) {
    --blue-50: #eef8ff;
    --blue-100: #d9efff;
    --blue-200: #c7e6fb;
    --blue-500: #2f8cff;
    --blue-600: #176ee6;
    --blue-700: #1158bb;
    --ink: #202933;
    --muted: #7a8794;
    --line: #e7edf3;
    --panel: #ffffff;
    --canvas: #f6f8fb;
  }

  :global(*) { box-sizing: border-box; }
  :global(html), :global(body) {
    margin: 0;
    min-width: 320px;
    width: 100%;
    height: 100%;
    overflow: hidden;
    color: var(--ink);
    background: #e9eef5;
    font-family: "Segoe UI", "Microsoft YaHei", sans-serif;
    font-size: 16px;
    letter-spacing: 0;
    text-rendering: optimizeLegibility;
    -webkit-font-smoothing: antialiased;
  }
  :global(html:has(.overlay-root)), :global(body:has(.overlay-root)) {
    min-width: 0;
    width: 100vw;
    height: 100vh;
    overflow: hidden !important;
    background: #176ee6;
  }
  :global(html:has(.toast-root)), :global(body:has(.toast-root)) {
    min-width: 0;
    width: 100vw;
    height: 100vh;
    overflow: hidden !important;
    background: transparent;
    font-size: 14px;
  }
  :global(body:has(.overlay-root)::-webkit-scrollbar),
  :global(body:has(.overlay-root) *::-webkit-scrollbar),
  :global(body:has(.toast-root)::-webkit-scrollbar),
  :global(body:has(.toast-root) *::-webkit-scrollbar) {
    width: 0;
    height: 0;
    display: none;
  }
  .overlay-root {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100vw;
    height: 100vh;
    padding: 0;
    overflow: hidden;
    background: #176ee6;
  }
  .overlay-caption {
    display: flex;
    align-items: center;
    justify-content: flex-start;
    gap: 9px;
    width: 100%;
    height: 100%;
    min-width: 0;
    padding: 8px 14px;
    overflow: hidden;
    color: #ffffff;
    background: #176ee6;
    border: 0;
    border-radius: 0;
    box-shadow: none;
    text-align: left;
  }
  .overlay-dot {
    flex: 0 0 auto;
    width: 8px;
    height: 8px;
    border-radius: 999px;
    background: #dff0ff;
    box-shadow: 0 0 0 5px rgba(255, 255, 255, 0.18);
  }
  .overlay-caption-text {
    display: grid;
    align-content: start;
    flex: 1 1 auto;
    min-width: 0;
    height: 100%;
    max-height: 100%;
    overflow: hidden;
    color: #ffffff !important;
    font-weight: 400;
    line-height: 1.18;
    text-shadow: 0 1px 1px rgba(12, 57, 120, 0.55);
    white-space: normal;
    overflow-wrap: normal;
  }
  .overlay-caption-text.single {
    align-content: center;
    text-align: center;
  }
  .overlay-caption-text.double {
    align-content: start;
    text-align: left;
  }
  .overlay-caption-text span {
    display: block;
    min-width: 0;
    overflow: hidden;
    text-overflow: clip;
    white-space: pre-wrap;
  }
  .toast-root {
    display: grid;
    width: 100vw;
    height: 100vh;
    place-items: center;
    min-width: 0;
    padding: 6px;
    overflow: hidden;
    background: transparent;
  }
  .toast-card {
    display: grid;
    grid-template-columns: 30px minmax(0, 1fr);
    align-items: center;
    gap: 10px;
    width: 100%;
    height: 100%;
    min-width: 0;
    padding: 9px 12px;
    overflow: hidden;
    color: #17222e;
    background: rgba(255, 255, 255, 0.98);
    border: 1px solid rgba(47, 140, 255, 0.18);
    border-radius: 12px;
    box-shadow: 0 8px 20px rgba(34, 61, 88, 0.16);
  }
  .toast-icon {
    display: grid;
    width: 30px;
    height: 30px;
    place-items: center;
    color: #ffffff;
    background: var(--blue-500);
    border-radius: 50%;
    box-shadow: 0 0 0 4px rgba(47, 140, 255, 0.12);
  }
  .toast-copy {
    display: grid;
    gap: 3px;
    min-width: 0;
    overflow: hidden;
  }
  .toast-card strong {
    display: block;
    min-width: 0;
    overflow: hidden;
    color: #152231;
    font-size: 0.88rem;
    font-weight: 700;
    line-height: 1.18;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .toast-card span {
    display: block;
    min-width: 0;
    overflow: hidden;
    color: #66788a;
    font-size: 0.72rem;
    line-height: 1.24;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  @media (prefers-reduced-motion: no-preference) {
    .toast-card { animation: toast-enter 180ms ease-out; }
    @keyframes toast-enter {
      from { opacity: 0; transform: translateY(6px) scale(0.98); }
      to { opacity: 1; transform: translateY(0) scale(1); }
    }
  }
  :global(button), :global(input), :global(textarea), :global(select) { font: inherit; }
  .app-frame {
    display: grid;
    grid-template-rows: 36px minmax(0, 1fr);
    width: 100vw;
    height: 100vh;
    overflow: hidden;
    background: var(--canvas);
  }
  .window-titlebar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-width: 0;
    padding: 0 8px 0 14px;
    color: #40566b;
    background: linear-gradient(180deg, #f9fbfd 0%, #edf5fc 100%);
    border-bottom: 1px solid #dbe5ee;
    user-select: none;
  }
  .window-title {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1 1 auto;
    min-width: 0;
    height: 100%;
    color: #455f78;
    font-size: 0.78rem;
    font-weight: 800;
    letter-spacing: 0.02em;
  }
  .window-title-mark {
    display: grid;
    width: 20px;
    height: 20px;
    place-items: center;
    color: #ffffff;
    background: var(--blue-500);
    border: 2px solid #ffffff;
    border-radius: 50%;
    box-shadow: 0 0 0 4px rgba(47, 140, 255, 0.14);
  }
  .window-controls {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .window-controls button {
    display: grid;
    width: 34px;
    height: 26px;
    place-items: center;
    color: #4d5e70;
    background: transparent;
    border-radius: 7px;
  }
  .window-controls button:hover { background: rgba(47, 140, 255, 0.12); color: #17436d; }
  .window-controls button.close:hover { background: #e85d67; color: #ffffff; }
  .shell {
    display: grid;
    grid-template-columns: 220px minmax(0, 1fr);
    width: 100%;
    height: 100%;
    min-height: 0;
    margin: 0;
    overflow: hidden;
    background: var(--canvas);
    border: 0;
    border-radius: 0;
    box-shadow: none;
  }
  .sidebar {
    display: flex;
    flex-direction: column;
    gap: 14px;
    min-width: 0;
    padding: 20px 10px 14px;
    color: var(--ink);
    background: linear-gradient(180deg, #d8f1ff 0%, #e9f7ff 100%);
    overflow: hidden;
  }
  .brand { display: flex; align-items: center; gap: 12px; min-width: 0; padding: 0 10px; }
  .brand-mark {
    display: grid;
    width: 38px;
    height: 38px;
    place-items: center;
    color: #ffffff;
    background: var(--blue-500);
    border: 3px solid #ffffff;
    border-radius: 50%;
    box-shadow: 0 2px 8px rgba(47, 140, 255, 0.28);
  }
  .eyebrow { margin: 0 0 4px; color: #5e7892; font-size: 0.72rem; font-weight: 700; text-transform: uppercase; }
  h1, h2, h3, p { margin-top: 0; }
  h1 { margin-bottom: 0; font-size: 1.12rem; font-weight: 800; white-space: nowrap; }
  nav { display: grid; gap: 6px; padding: 0; }
  button { border: 0; cursor: pointer; }
  nav button {
    display: flex;
    align-items: center;
    gap: 14px;
    width: calc(100% - 14px);
    min-height: 42px;
    margin: 0 7px;
    padding: 0 14px;
    color: #26323e;
    background: transparent;
    border-radius: 12px;
    text-align: left;
    font-size: 0.98rem;
    font-weight: 500;
  }
  nav button:hover { background: rgba(255, 255, 255, 0.58); }
  nav button.active {
    color: #ffffff;
    background: var(--blue-500);
    box-shadow: 0 10px 22px rgba(47, 140, 255, 0.24);
  }
  .language-control {
    display: grid;
    gap: 7px;
    margin: 2px 7px 0;
  }
  .language-control span {
    color: #5e7892;
    font-size: 0.72rem;
    font-weight: 700;
    text-transform: uppercase;
  }
  .language-control select {
    min-height: 34px;
    padding: 6px 8px;
    color: #26323e;
    background: rgba(255, 255, 255, 0.72);
    border: 1px solid rgba(47, 140, 255, 0.16);
    border-radius: 8px;
  }
  .bridge-card {
    margin: auto 7px 0;
    padding: 11px;
    background: rgba(255, 255, 255, 0.72);
    border: 1px solid rgba(47, 140, 255, 0.14);
    border-radius: 12px;
  }
  .bridge-top { display: flex; align-items: center; gap: 9px; margin-bottom: 8px; font-weight: 700; }
  .bridge-card p { margin-bottom: 0; color: #607487; font-size: 0.8rem; line-height: 1.4; }
  .pulse, .dot { width: 9px; height: 9px; border-radius: 50%; background: #a8b8c6; }
  .pulse.recording, .dot.recording { background: var(--blue-500); box-shadow: 0 0 0 7px rgba(47, 140, 255, 0.16); }
  .content { min-width: 0; padding: 18px 24px 18px; overflow-y: auto; }
  .topbar { display: flex; align-items: center; justify-content: space-between; gap: 14px; margin-bottom: 12px; }
  h2 { max-width: 760px; margin-bottom: 0; color: #111820; font-size: 1.36rem; line-height: 1.2; font-weight: 500; }
  .form-actions { display: flex; gap: 10px; }
  .form-actions button { display: inline-flex; align-items: center; justify-content: center; gap: 8px; min-height: 40px; padding: 0 14px; color: #ffffff; background: var(--blue-500); border-radius: 8px; }
  .form-actions button:not(.primary) { color: #344150; background: #ffffff; border: 1px solid #d6dee6; }
  .hero-grid, .page-grid { display: grid; gap: 12px; }
  .hero-grid { grid-template-columns: minmax(0, 1.2fr) minmax(260px, 0.8fr); }
  .page-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); align-items: start; }
  .live-panel, .control-panel, .stat, .form-panel {
    background: var(--panel);
    border: 1px solid #dfe6ed;
    border-radius: 16px;
    box-shadow: none;
  }
  .live-panel, .control-panel, .form-panel { padding: 16px; }
  .wide { grid-column: 1 / -1; }
  .setup-alert {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    margin-bottom: 12px;
    padding: 12px 14px;
    color: #17395f;
    background: #eef7ff;
    border: 1px solid #bfdbff;
    border-radius: 11px;
  }
  .setup-alert strong { display: block; margin-bottom: 3px; color: #10253b; font-size: 0.95rem; }
  .setup-alert p { margin-bottom: 0; color: #607487; font-size: 0.84rem; line-height: 1.45; }
  .setup-actions { display: flex; flex: 0 0 auto; gap: 8px; }
  .setup-actions button {
    flex: 0 0 auto;
    min-height: 34px;
    padding: 0 12px;
    color: #ffffff;
    background: var(--blue-500);
    border-radius: 8px;
    font-size: 0.86rem;
    font-weight: 700;
  }
  .setup-actions button.secondary {
    color: #17436d;
    background: #ffffff;
    border: 1px solid #bfdbff;
  }
  .setup-note {
    padding: 10px 11px;
    color: #17436d !important;
    background: #eef7ff;
    border: 1px solid #bfdbff;
    border-radius: 9px;
  }
  .link-button {
    min-height: 32px;
    padding: 0 10px;
    color: #17436d;
    background: #eef7ff;
    border: 1px solid #bfdbff;
    border-radius: 8px;
    font-size: 0.84rem;
    font-weight: 700;
  }
  .caption-window { min-height: 76px; padding: 13px 15px; color: #ffffff; background: linear-gradient(135deg, var(--blue-600), #4b9cff); border: 1px solid rgba(255, 255, 255, 0.38); border-radius: 11px; }
  .caption-meta { display: flex; align-items: center; gap: 8px; margin-bottom: 8px; color: #eaf5ff; font-size: 0.8rem; font-weight: 700; }
  .caption-window p { max-width: 680px; margin-bottom: 0; font-size: 0.92rem; line-height: 1.45; }
  .panel-title { display: flex; align-items: center; gap: 9px; margin-bottom: 12px; color: #17222e; font-weight: 800; }
  dl, .form-grid, .toggle-grid { display: grid; gap: 14px; }
  dl { margin: 0; }
  .control-grid {
    grid-template-columns: 1fr;
    gap: 10px;
  }
  .control-grid > div {
    padding: 10px 11px;
    background: #f7fbff;
    border: 1px solid #e4eef8;
    border-radius: 10px;
  }
  .control-inline {
    display: grid;
    grid-template-columns: minmax(92px, 0.4fr) minmax(0, 1fr);
    gap: 14px;
    align-items: center;
  }
  .control-inline div + div {
    padding-left: 14px;
    border-left: 1px solid #dde8f3;
  }
  .form-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .form-grid.compact { grid-template-columns: repeat(2, minmax(120px, 1fr)); }
  .toggle-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); margin-top: 14px; }
  dt, label span, .stat span { color: var(--muted); font-size: 0.78rem; font-weight: 700; text-transform: uppercase; }
  dd { margin: 0; color: #17222e; font-size: 0.98rem; font-weight: 800; }
  label { display: grid; gap: 7px; margin-bottom: 14px; }
  input, textarea, select {
    width: 100%;
    min-height: 40px;
    padding: 9px 10px;
    color: #17222e;
    background: #ffffff;
    border: 1px solid #d8e0e8;
    border-radius: 8px;
    outline: none;
  }
  textarea { min-height: 118px; resize: vertical; }
  input:focus, textarea:focus, select:focus, button:focus-visible { border-color: var(--blue-500); box-shadow: 0 0 0 3px rgba(47, 140, 255, 0.15); }
  .check { display: flex; align-items: center; gap: 9px; color: #344150; font-size: 0.9rem; }
  .check input { width: 38px; min-height: 22px; accent-color: var(--blue-500); }
  .stats-row { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 12px; margin: 12px 0 0; }
  .stats-row.nested { grid-template-columns: repeat(2, minmax(0, 1fr)); margin: 0; }
  .stat { padding: 14px 15px; }
  .stat strong { color: #17222e; font-size: 1.18rem; }
  .stat small { display: block; margin-top: 6px; color: var(--muted); font-size: 0.72rem; line-height: 1.35; }
  .stat.green, .stat.blue, .stat.amber { border-top: 4px solid var(--blue-500); }
  .section-heading h3 { margin-bottom: 5px; color: #17222e; font-size: 1.08rem; font-weight: 600; }
  .section-heading p { margin-bottom: 14px; overflow-wrap: anywhere; color: var(--muted); font-size: 0.88rem; }
  article strong, article span { display: block; }
  article strong { color: #17222e; }
  article span { margin-top: 3px; color: var(--muted); font-size: 0.88rem; }
  em { flex: 0 0 auto; min-width: 68px; padding: 5px 8px; color: #17436d; background: #e8f3ff; border-radius: 6px; font-size: 0.75rem; font-style: normal; font-weight: 800; text-align: center; }
  .day-list, .history-list { display: grid; gap: 10px; }
  .day-list div, .history-list article { display: grid; grid-template-columns: 1fr auto auto; gap: 12px; align-items: center; min-height: 42px; padding: 10px 0; border-top: 1px solid var(--line); }
  .empty { margin-bottom: 0; color: var(--muted); }
  @media (max-width: 1120px) {
    .shell { grid-template-columns: 212px minmax(0, 1fr); }
    .content { padding: 18px 20px; }
    .hero-grid, .page-grid { grid-template-columns: 1fr; }
    .stats-row { grid-template-columns: repeat(2, minmax(0, 1fr)); }
    .topbar { align-items: flex-start; }
    .live-panel, .control-panel, .form-panel { padding: 16px; }
  }
  @media (max-width: 760px) {
    .shell { grid-template-columns: 72px minmax(0, 1fr); }
    .sidebar { gap: 14px; padding: 18px 8px 12px; align-items: center; }
    .brand { justify-content: center; padding: 0; }
    .brand > div, nav button span, .language-control, .bridge-card { display: none; }
    .brand-mark { width: 42px; height: 42px; }
    nav { width: 100%; gap: 8px; }
    nav button {
      justify-content: center;
      width: 48px;
      min-height: 48px;
      margin: 0 auto;
      padding: 0;
      border-radius: 12px;
    }
    .content { padding: 18px; }
    .topbar { gap: 12px; margin-bottom: 12px; }
    h2 { font-size: 1.35rem; }
    .stats-row { grid-template-columns: 1fr; }
    .stats-row.nested { grid-template-columns: 1fr; }
    .form-grid, .form-grid.compact, .toggle-grid { grid-template-columns: 1fr; }
  }
  @media (max-width: 520px) {
    .shell { grid-template-columns: 60px minmax(0, 1fr); }
    .sidebar { padding-inline: 6px; }
    .brand-mark { width: 38px; height: 38px; }
    nav button { width: 42px; min-height: 42px; }
    .content { padding: 14px; }
    .topbar { flex-direction: column; align-items: flex-start; }
    .caption-window p { font-size: 0.94rem; }
    .setup-alert { align-items: stretch; flex-direction: column; }
    .setup-actions { flex-direction: column; }
    .setup-actions button { width: 100%; }
    .control-inline { grid-template-columns: 1fr; gap: 10px; }
    .control-inline div + div { padding-left: 0; padding-top: 10px; border-left: 0; border-top: 1px solid #dde8f3; }
  }
</style>
