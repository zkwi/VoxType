<script lang="ts">
  import { onMount } from "svelte";
  import { browser } from "$app/environment";
  import SetupStatusCard, {
    type SetupStatusItem,
    type SetupStatusWarning,
  } from "$lib/components/overview/SetupStatusCard.svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import {
    AlertCircle,
    BarChart3,
    CalendarDays,
    Check,
    ChevronRight,
    Clock3,
    Download,
    FileText,
    Gauge,
    Globe2,
    Keyboard,
    Maximize2,
    MessageSquareText,
    Mic,
    Minus,
    PenLine,
    Save,
    Settings,
    ShieldCheck,
    Sparkles,
    Zap,
    X as XIcon,
  } from "lucide-svelte";

  type Section = "Overview" | "Settings" | "History";
  type Language = "zh-CN" | "zh-TW" | "en";

  type AppSnapshot = {
    hotkey: string;
    current_version: string;
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

  type UpdateStatus = {
    current_version: string;
    latest_version: string;
    update_available: boolean;
    asset_name: string | null;
    asset_size: number | null;
    message: string;
  };

  type InstallUpdateResult = {
    version: string;
    asset_name: string;
    message: string;
  };

  type ConnectionTestResult = { message: string };

  type SetupStatus = {
    ready: boolean;
    missing_auth: boolean;
    has_audio_device: boolean;
    hotkey: string;
    paste_method: string;
    privacy_recent_context_enabled: boolean;
    warnings: SetupStatusWarning[];
  };

  type LoadedConfig = {
    path: string;
    exists: boolean;
    data: AppConfig;
  };

  type SessionPhase =
    | "idle"
    | "starting"
    | "recording"
    | "stopping"
    | "waiting_final_result"
    | "post_editing"
    | "pasting"
    | "succeeded"
    | "failed";

  type SessionState = {
    recording: boolean;
    phase: SessionPhase;
    message: string;
    error_code: string | null;
  };

  type AsrFinalText = {
    text: string;
    error: string | null;
    warning: string | null;
  };

  type AsrPartialText = { text: string };
  type OverlayText = { text: string };
  type AudioLevel = { level: number };
  type AudioDeviceInfo = { index: number; name: string; is_default: boolean };
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
    triggers: {
      hotkey_enabled: boolean;
      middle_mouse_enabled: boolean;
      right_alt_enabled: boolean;
    };
    typing: {
      paste_delay_ms: number;
      paste_method: string;
      restore_clipboard_after_paste: boolean;
      clipboard_open_retry_count: number;
      clipboard_open_retry_interval_ms: number;
    };
    startup: { launch_on_startup: boolean };
    update: { auto_check_on_startup: boolean; github_repo: string };
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

  type TriggerKey = keyof AppConfig["triggers"];

  const fallbackConfig: AppConfig = {
    hotkey: "ctrl+q",
    auth: { app_key: "", access_key: "", resource_id: "volc.seedasr.sauc.duration" },
    audio: {
      sample_rate: 16000,
      channels: 1,
      segment_ms: 200,
      max_record_seconds: 300,
      stop_grace_ms: 500,
      mute_system_volume_while_recording: false,
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
      enable_recent_context: false,
      recent_context_rounds: 5,
      image_url: null,
      hotwords: [],
      prompt_context: [],
      recent_context: [],
    },
    triggers: { hotkey_enabled: true, middle_mouse_enabled: false, right_alt_enabled: false },
    typing: {
      paste_delay_ms: 120,
      paste_method: "ctrl_v",
      restore_clipboard_after_paste: true,
      clipboard_open_retry_count: 5,
      clipboard_open_retry_interval_ms: 50,
    },
    startup: { launch_on_startup: false },
    update: { auto_check_on_startup: true, github_repo: "zkwi/VoxType" },
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
    debug: { print_transcript_to_console: false },
  };

  const fallbackSnapshot: AppSnapshot = {
    hotkey: "ctrl+q",
    current_version: "0.1.9",
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
  const micBars = [0, 1, 2, 3, 4, 5];
  const navItems: { id: Section; icon: typeof Gauge }[] = [
    { id: "Overview", icon: Gauge },
    { id: "Settings", icon: Settings },
    { id: "History", icon: BarChart3 },
  ];

  const copy = {
    "zh-CN": {
      appTitle: "声写",
      language: "语言",
      minimizeToTray: "最小化到托盘",
      navOverview: "输入",
      voiceInputTitle: "语音输入",
      navSettings: "配置",
      navHistory: "统计历史",
      topEyebrow: "VoxType",
      recordingPreview: "录音中",
      idle: "空闲",
      clickStart: "点击开始语音输入",
      clickStop: "点击停止语音输入",
      quickStart: "按 {hotkey} 快速启动",
      speakAnywhere: "在任何输入框直接说话",
      mixedInput: "支持中英文混合输入",
      mainHotkey: "主快捷键",
      enabled: "已启用",
      disabled: "已关闭",
      middleMouse: "鼠标中键",
      rightAlt: "右 Alt 键",
      todayInput: "今日输入",
      inputSpeed: "输入速度",
      savedTime: "节省时间",
      perMinute: "字每分钟",
      savedToday: "约节省 {hours} 小时",
      weeklySavedShort: "本周节省时间",
      micConnected: "麦克风：{device}",
      micMonitoring: "正在监听：{device}",
      micUnavailable: "未检测到麦克风",
      sidebarMicConnected: "麦克风：已连接",
      sidebarMicUnavailable: "麦克风：未连接",
      sidebarShortcut: "快捷键：{hotkey}",
      usageTipEmpty: "完成一次语音输入后，这里会显示真实使用统计。",
      usageTipData: "已记录 {sessions} 次语音输入，共 {chars} 字。",
      waitingVoice: "等待语音输入",
      listeningPreview: "正在监听麦克风，实时字幕显示在屏幕下方。",
      pressHotkey: "按 {hotkey}、右 Alt 或鼠标中键，也可从托盘启动，向任意输入框语音输入。",
      sessionIdleHint: "把光标放到输入框，然后按 {hotkey} 开始说话。",
      sessionStarting: "正在启动麦克风...",
      sessionRecording: "正在听你说话，再按 {hotkey} 停止。",
      sessionStopping: "正在收尾...",
      sessionWaitingFinal: "正在等待最后一句识别完成...",
      sessionPostEditing: "正在润色文本...",
      sessionPasting: "正在粘贴到当前输入框...",
      sessionSucceeded: "已粘贴。",
      sessionFailed: "本次语音输入失败，请查看提示。",
      setupRequired: "需要先完成配置",
      inputError: "输入异常",
      setupMissingFile: "未找到配置文件。请在配置页填写认证信息并保存，或打开配置文件手动编辑。",
      setupMissingAuth: "ASR 认证信息未填写。请在配置页填写 App Key 和 Access Key 后保存。",
      setupCta: "去配置",
      setupGuideCta: "查看配置指南",
      setupHealthTitle: "配置健康检查",
      setupHealthPendingTitle: "还差 {count} 步即可使用",
      setupHealthPendingDescription: "先处理密钥、麦克风和输出设置，就能更安全地开始语音输入。",
      setupHealthReadyTitle: "VoxType 已准备好",
      setupHealthReadyDescription: "把光标放到输入框，然后按 {hotkey} 开始说话。",
      refreshSetup: "重新检查",
      setupAuthLabel: "ASR 密钥",
      setupMicLabel: "麦克风",
      setupPasteLabel: "粘贴方式",
      setupTriggerLabel: "触发方式",
      setupPrivacyLabel: "隐私设置",
      setupOk: "已完成",
      setupMissing: "未完成",
      setupMicDetected: "已检测",
      setupMicMissing: "未检测",
      setupRecentOff: "最近上下文已关闭",
      setupRecentOn: "最近上下文已开启",
      setupActionAsr: "填写 ASR 密钥",
      setupActionAudio: "检查麦克风",
      setupActionTyping: "调整粘贴方式",
      setupActionHotkey: "查看触发方式",
      setupActionPrivacy: "查看隐私设置",
      shortcutSettings: "修改快捷键",
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
      softwareSettings: "软件相关设置",
      softwareSettingsDescription: "启动方式、输入输出、悬浮字幕、托盘和软件更新。",
      doubaoAsrSettings: "豆包 ASR 相关设置",
      doubaoAsrSettingsDescription: "豆包认证、录音参数、识别请求和上下文增强。",
      llmSettings: "大模型相关设置",
      llmSettingsDescription: "可选文本润色能力，未启用时不会影响语音输入主流程。",
      startAndOutput: "启动与输出",
      floatingCaptionAndTray: "悬浮字幕与托盘",
      doubaoAuth: "豆包认证",
      recordingParams: "录音参数",
      recognitionOptions: "识别选项",
      timeColumn: "时间",
      charsColumn: "字数",
      durationColumn: "时长",
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
      restoreClipboardAfterPaste: "粘贴后恢复原剪贴板",
      clipboardRetryCount: "剪贴板重试次数",
      clipboardRetryInterval: "重试间隔毫秒",
      triggerConflictHint: "右 Alt 和鼠标中键可能和其他软件操作冲突，建议确认后再开启。",
      muteSystemAudioHint: "可减少回声，但可能影响会议、视频或系统提示音。",
      recentContextHint: "最近上下文会在本地保存最近识别片段，用于改善连续识别；默认关闭。",
      launchOnStartup: "开机自动启动",
      softwareUpdate: "软件更新",
      softwareUpdateDescription: "通过 GitHub Release 检查最新版，并下载 Windows 安装包。",
      autoCheckUpdates: "启动时自动检查更新",
      checkUpdates: "检查更新",
      checkingUpdates: "检查中",
      downloadInstall: "下载并安装",
      downloadingInstall: "下载中",
      updateNotChecked: "尚未检查更新",
      updateIdleDescription: "可手动检查，或开启启动时自动检查。",
      updateAvailable: "发现新版本",
      updateReady: "可下载 {asset}",
      updateUpToDate: "已是最新版本",
      updateNoInstaller: "未找到 Windows 安装包",
      currentVersion: "当前版本",
      latestVersion: "最新版本",
      diagnosticsAndLogs: "诊断与日志",
      diagnosticsDescription: "遇到识别、粘贴、网络或更新问题时，可打开日志协助排查。",
      logStatusTitle: "本地日志已启用",
      logStatusDescription: "日志会记录关键状态和错误，密钥会自动脱敏。",
      openLog: "查看日志",
      openingLog: "打开中",
      logOpened: "日志文件已打开。",
      testConnection: "测试",
      testingConnection: "测试中",
      llmPostEdit: "大模型润色",
      llmDescription: "OpenAI 兼容接口的润色回退。",
      enablePolishing: "启用润色",
      minChars: "最少字符数",
      timeout: "超时",
      saveConfig: "保存配置",
      saving: "保存中",
      reload: "重新加载",
      configReloaded: "已重新加载配置。",
      recentUsage: "输入表现",
      chars24h: "24 小时字数",
      chars7d: "7 日字数",
      avgCpm: "平均每分钟字数",
      weeklySavedHours: "每周节省",
      weeklySavedHoursHint: "按中文输入约每分钟 50 字估算。",
      byDay: "按日统计",
      lastSevenDays: "最近七个自然日。",
      dateColumn: "日期",
      dailyInputChars: "输入字数",
      voiceDuration: "语音时长",
      averageInputSpeed: "平均输入速度",
      dailySavedTime: "节省时间",
      recentRecords: "最近记录",
      historyDescription: "最近保存的输入记录。",
      noUsageRecords: "暂无使用记录。",
      bridgeLoading: "正在准备...",
      browserPreview: "请在桌面应用中使用。",
      bridgeConnected: "准备就绪。",
      configSaved: "保存成功。",
      previewRecording: "录音中。",
      previewStopped: "等待语音输入",
      startupToastTitle: "声写已启动",
      startupToastHint: "{hotkey} / 右 Alt / 鼠标中键",
    },
    "zh-TW": {
      appTitle: "聲寫",
      language: "語言",
      minimizeToTray: "最小化到系統匣",
      navOverview: "輸入",
      voiceInputTitle: "語音輸入",
      navSettings: "配置",
      navHistory: "統計歷史",
      topEyebrow: "VoxType",
      recordingPreview: "錄音中",
      idle: "閒置",
      clickStart: "點擊開始語音輸入",
      clickStop: "點擊停止語音輸入",
      quickStart: "按 {hotkey} 快速啟動",
      speakAnywhere: "在任何輸入框直接說話",
      mixedInput: "支援中英文混合輸入",
      mainHotkey: "主快捷鍵",
      enabled: "已啟用",
      disabled: "已關閉",
      middleMouse: "滑鼠中鍵",
      rightAlt: "右 Alt 鍵",
      todayInput: "今日輸入",
      inputSpeed: "輸入速度",
      savedTime: "節省時間",
      perMinute: "字每分鐘",
      savedToday: "約節省 {hours} 小時",
      weeklySavedShort: "本週節省時間",
      micConnected: "麥克風：{device}",
      micMonitoring: "正在監聽：{device}",
      micUnavailable: "未偵測到麥克風",
      sidebarMicConnected: "麥克風：已連接",
      sidebarMicUnavailable: "麥克風：未連接",
      sidebarShortcut: "快捷鍵：{hotkey}",
      usageTipEmpty: "完成一次語音輸入後，這裡會顯示真實使用統計。",
      usageTipData: "已記錄 {sessions} 次語音輸入，共 {chars} 字。",
      waitingVoice: "等待語音輸入",
      listeningPreview: "正在監聽麥克風，即時字幕顯示在螢幕下方。",
      pressHotkey: "按 {hotkey}、右 Alt 或滑鼠中鍵，也可從系統匣啟動，向任意輸入框語音輸入。",
      sessionIdleHint: "把游標放到輸入框，然後按 {hotkey} 開始說話。",
      sessionStarting: "正在啟動麥克風...",
      sessionRecording: "正在聽你說話，再按 {hotkey} 停止。",
      sessionStopping: "正在收尾...",
      sessionWaitingFinal: "正在等待最後一句識別完成...",
      sessionPostEditing: "正在潤飾文字...",
      sessionPasting: "正在貼到目前輸入框...",
      sessionSucceeded: "已貼上。",
      sessionFailed: "本次語音輸入失敗，請查看提示。",
      setupRequired: "需要先完成配置",
      inputError: "輸入異常",
      setupMissingFile: "未找到配置檔案。請在配置頁填寫認證資訊並儲存，或打開配置檔案手動編輯。",
      setupMissingAuth: "ASR 認證資訊未填寫。請在配置頁填寫 App Key 和 Access Key 後儲存。",
      setupCta: "去配置",
      setupGuideCta: "查看配置指南",
      setupHealthTitle: "配置健康檢查",
      setupHealthPendingTitle: "還差 {count} 步即可使用",
      setupHealthPendingDescription: "先處理密鑰、麥克風和輸出設定，就能更安全地開始語音輸入。",
      setupHealthReadyTitle: "VoxType 已準備好",
      setupHealthReadyDescription: "把游標放到輸入框，然後按 {hotkey} 開始說話。",
      refreshSetup: "重新檢查",
      setupAuthLabel: "ASR 密鑰",
      setupMicLabel: "麥克風",
      setupPasteLabel: "貼上方式",
      setupTriggerLabel: "觸發方式",
      setupPrivacyLabel: "隱私設定",
      setupOk: "已完成",
      setupMissing: "未完成",
      setupMicDetected: "已偵測",
      setupMicMissing: "未偵測",
      setupRecentOff: "最近上下文已關閉",
      setupRecentOn: "最近上下文已開啟",
      setupActionAsr: "填寫 ASR 密鑰",
      setupActionAudio: "檢查麥克風",
      setupActionTyping: "調整貼上方式",
      setupActionHotkey: "查看觸發方式",
      setupActionPrivacy: "查看隱私設定",
      shortcutSettings: "修改快捷鍵",
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
      softwareSettings: "軟體相關設定",
      softwareSettingsDescription: "啟動方式、輸入輸出、懸浮字幕、系統匣和軟體更新。",
      doubaoAsrSettings: "豆包 ASR 相關設定",
      doubaoAsrSettingsDescription: "豆包認證、錄音參數、識別請求和上下文增強。",
      llmSettings: "大模型相關設定",
      llmSettingsDescription: "可選文字潤飾能力，未啟用時不會影響語音輸入主流程。",
      startAndOutput: "啟動與輸出",
      floatingCaptionAndTray: "懸浮字幕與系統匣",
      doubaoAuth: "豆包認證",
      recordingParams: "錄音參數",
      recognitionOptions: "識別選項",
      timeColumn: "時間",
      charsColumn: "字數",
      durationColumn: "時長",
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
      restoreClipboardAfterPaste: "貼上後恢復原剪貼簿",
      clipboardRetryCount: "剪貼簿重試次數",
      clipboardRetryInterval: "重試間隔毫秒",
      triggerConflictHint: "右 Alt 和滑鼠中鍵可能和其他軟體操作衝突，建議確認後再開啟。",
      muteSystemAudioHint: "可減少回聲，但可能影響會議、影片或系統提示音。",
      recentContextHint: "最近上下文會在本地保存最近識別片段，用於改善連續識別；預設關閉。",
      launchOnStartup: "開機自動啟動",
      softwareUpdate: "軟體更新",
      softwareUpdateDescription: "透過 GitHub Release 檢查最新版，並下載 Windows 安裝包。",
      autoCheckUpdates: "啟動時自動檢查更新",
      checkUpdates: "檢查更新",
      checkingUpdates: "檢查中",
      downloadInstall: "下載並安裝",
      downloadingInstall: "下載中",
      updateNotChecked: "尚未檢查更新",
      updateIdleDescription: "可手動檢查，或開啟啟動時自動檢查。",
      updateAvailable: "發現新版本",
      updateReady: "可下載 {asset}",
      updateUpToDate: "已是最新版本",
      updateNoInstaller: "未找到 Windows 安裝包",
      currentVersion: "目前版本",
      latestVersion: "最新版本",
      diagnosticsAndLogs: "診斷與日誌",
      diagnosticsDescription: "遇到識別、貼上、網路或更新問題時，可打開日誌協助排查。",
      logStatusTitle: "本地日誌已啟用",
      logStatusDescription: "日誌會記錄關鍵狀態和錯誤，密鑰會自動脫敏。",
      openLog: "查看日誌",
      openingLog: "打開中",
      logOpened: "日誌檔案已打開。",
      testConnection: "測試",
      testingConnection: "測試中",
      llmPostEdit: "大模型潤飾",
      llmDescription: "OpenAI 相容介面的潤飾回退。",
      enablePolishing: "啟用潤飾",
      minChars: "最少字元數",
      timeout: "逾時",
      saveConfig: "儲存配置",
      saving: "儲存中",
      reload: "重新載入",
      configReloaded: "已重新載入配置。",
      recentUsage: "輸入表現",
      chars24h: "24 小時字數",
      chars7d: "7 日字數",
      avgCpm: "平均每分鐘字數",
      weeklySavedHours: "每週節省",
      weeklySavedHoursHint: "按中文輸入約每分鐘 50 字估算。",
      byDay: "按日統計",
      lastSevenDays: "最近七個自然日。",
      dateColumn: "日期",
      dailyInputChars: "輸入字數",
      voiceDuration: "語音時長",
      averageInputSpeed: "平均輸入速度",
      dailySavedTime: "節省時間",
      recentRecords: "最近記錄",
      historyDescription: "最近保存的輸入記錄。",
      noUsageRecords: "暫無使用記錄。",
      bridgeLoading: "正在準備...",
      browserPreview: "請在桌面應用中使用。",
      bridgeConnected: "準備就緒。",
      configSaved: "儲存成功。",
      previewRecording: "錄音中。",
      previewStopped: "等待語音輸入",
      startupToastTitle: "聲寫已啟動",
      startupToastHint: "{hotkey} / 右 Alt / 滑鼠中鍵",
    },
    en: {
      appTitle: "VoxType",
      language: "Language",
      minimizeToTray: "Minimize to tray",
      navOverview: "Input",
      voiceInputTitle: "Voice input",
      navSettings: "Settings",
      navHistory: "Stats",
      topEyebrow: "VoxType",
      recordingPreview: "Recording",
      idle: "Idle",
      clickStart: "Click to start voice input",
      clickStop: "Click to stop voice input",
      quickStart: "Press {hotkey} to start quickly",
      speakAnywhere: "Dictate into any input box",
      mixedInput: "Supports mixed Chinese and English",
      mainHotkey: "Primary hotkey",
      enabled: "Enabled",
      disabled: "Off",
      middleMouse: "Middle mouse",
      rightAlt: "Right Alt",
      todayInput: "Today",
      inputSpeed: "Input speed",
      savedTime: "Saved time",
      perMinute: "chars per min",
      savedToday: "About {hours} h saved",
      weeklySavedShort: "Saved this week",
      micConnected: "Mic: {device}",
      micMonitoring: "Listening: {device}",
      micUnavailable: "No microphone detected",
      sidebarMicConnected: "Mic: connected",
      sidebarMicUnavailable: "Mic: unavailable",
      sidebarShortcut: "Shortcut: {hotkey}",
      usageTipEmpty: "After one dictation, real usage stats will appear here.",
      usageTipData: "{sessions} dictations recorded, {chars} chars total.",
      waitingVoice: "Waiting for voice input",
      listeningPreview: "Listening to the microphone. Live captions appear near the bottom of the screen.",
      pressHotkey: "Press {hotkey}, Right Alt, or the middle mouse button, or start from the tray.",
      sessionIdleHint: "Place the cursor in an input box, then press {hotkey} to speak.",
      sessionStarting: "Starting microphone...",
      sessionRecording: "Listening. Press {hotkey} again to stop.",
      sessionStopping: "Finishing up...",
      sessionWaitingFinal: "Waiting for the final recognition result...",
      sessionPostEditing: "Polishing text...",
      sessionPasting: "Pasting into the current input box...",
      sessionSucceeded: "Pasted.",
      sessionFailed: "This dictation failed. Check the hint.",
      setupRequired: "Setup required",
      inputError: "Input issue",
      setupMissingFile: "No config file found. Fill credentials on the Settings page and save, or open the config file manually.",
      setupMissingAuth: "ASR credentials are missing. Fill App Key and Access Key on the Settings page, then save.",
      setupCta: "Open Settings",
      setupGuideCta: "Setup Guide",
      setupHealthTitle: "Setup health",
      setupHealthPendingTitle: "{count} steps left",
      setupHealthPendingDescription: "Finish credentials, microphone, and output settings before dictating.",
      setupHealthReadyTitle: "VoxType is ready",
      setupHealthReadyDescription: "Place the cursor in an input box, then press {hotkey} to speak.",
      refreshSetup: "Check again",
      setupAuthLabel: "ASR keys",
      setupMicLabel: "Microphone",
      setupPasteLabel: "Paste method",
      setupTriggerLabel: "Trigger",
      setupPrivacyLabel: "Privacy",
      setupOk: "Ready",
      setupMissing: "Missing",
      setupMicDetected: "Detected",
      setupMicMissing: "Not detected",
      setupRecentOff: "Recent context off",
      setupRecentOn: "Recent context on",
      setupActionAsr: "Fill ASR keys",
      setupActionAudio: "Check mic",
      setupActionTyping: "Adjust paste",
      setupActionHotkey: "View triggers",
      setupActionPrivacy: "View privacy",
      shortcutSettings: "Edit shortcuts",
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
      softwareSettings: "App settings",
      softwareSettingsDescription: "Launch methods, output behavior, floating captions, tray behavior, and updates.",
      doubaoAsrSettings: "Doubao ASR settings",
      doubaoAsrSettingsDescription: "Doubao credentials, recording parameters, recognition requests, and context.",
      llmSettings: "LLM settings",
      llmSettingsDescription: "Optional text polishing. When disabled, it does not affect the main dictation flow.",
      startAndOutput: "Launch and output",
      floatingCaptionAndTray: "Floating caption and tray",
      doubaoAuth: "Doubao credentials",
      recordingParams: "Recording parameters",
      recognitionOptions: "Recognition options",
      timeColumn: "Time",
      charsColumn: "Chars",
      durationColumn: "Duration",
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
      restoreClipboardAfterPaste: "Restore clipboard after paste",
      clipboardRetryCount: "Clipboard retries",
      clipboardRetryInterval: "Retry interval ms",
      triggerConflictHint: "Right Alt and middle mouse may conflict with other apps. Enable them only after confirming.",
      muteSystemAudioHint: "This can reduce echo, but may affect meetings, videos, or system notifications.",
      recentContextHint: "Recent context stores recent recognized snippets locally to improve continuous dictation. It is off by default.",
      launchOnStartup: "Launch at startup",
      softwareUpdate: "Software update",
      softwareUpdateDescription: "Check the latest GitHub Release and download the Windows installer.",
      autoCheckUpdates: "Check for updates on startup",
      checkUpdates: "Check updates",
      checkingUpdates: "Checking",
      downloadInstall: "Download and install",
      downloadingInstall: "Downloading",
      updateNotChecked: "Not checked yet",
      updateIdleDescription: "Check manually, or enable startup update checks.",
      updateAvailable: "Update available",
      updateReady: "Ready to download {asset}",
      updateUpToDate: "Up to date",
      updateNoInstaller: "No Windows installer found",
      currentVersion: "Current",
      latestVersion: "Latest",
      diagnosticsAndLogs: "Diagnostics and logs",
      diagnosticsDescription: "Open logs when dictation, paste, network, or update issues need troubleshooting.",
      logStatusTitle: "Local logging is on",
      logStatusDescription: "Logs record key states and errors. Secrets are redacted automatically.",
      openLog: "View logs",
      openingLog: "Opening",
      logOpened: "Log file opened.",
      testConnection: "Test",
      testingConnection: "Testing",
      llmPostEdit: "LLM Post Edit",
      llmDescription: "OpenAI-compatible polishing fallback.",
      enablePolishing: "Enable polishing",
      minChars: "Min chars",
      timeout: "Timeout",
      saveConfig: "Save config",
      saving: "Saving",
      reload: "Reload",
      configReloaded: "Config reloaded.",
      recentUsage: "Input performance",
      chars24h: "24h chars",
      chars7d: "7d chars",
      avgCpm: "Avg cpm",
      weeklySavedHours: "Weekly saved",
      weeklySavedHoursHint: "Estimated at 50 Chinese chars per min.",
      byDay: "By day",
      lastSevenDays: "Last seven calendar days.",
      dateColumn: "Date",
      dailyInputChars: "Input chars",
      voiceDuration: "Voice time",
      averageInputSpeed: "Avg input speed",
      dailySavedTime: "Saved time",
      recentRecords: "Recent records",
      historyDescription: "Recently saved dictation records.",
      noUsageRecords: "No usage records yet.",
      bridgeLoading: "Preparing...",
      browserPreview: "Open the desktop app to use this feature.",
      bridgeConnected: "Ready.",
      configSaved: "Saved.",
      previewRecording: "Recording.",
      previewStopped: "Waiting for voice input",
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
  let config = $state<AppConfig>(clonePlain(fallbackConfig));
  let stats = $state<StatsSnapshot>(emptyStats);
  let recording = $state(false);
  let sessionPhase = $state<SessionPhase>("idle");
  let sessionErrorCode = $state<string | null>(null);
  let language = $state<Language>("zh-CN");
  let statusMessage = $state(copy["zh-CN"].bridgeLoading);
  let selectedSection = $state<Section>("Overview");
  let saving = $state(false);
  let configExists = $state(true);
  let audioLevel = $state(0);
  const initialParams = browser ? new URLSearchParams(window.location.search) : new URLSearchParams();
  let audioDevices = $state<AudioDeviceInfo[]>([]);
  let isOverlay = $state(initialParams.has("overlay"));
  let isToast = $state(initialParams.has("toast"));
  let toastHotkey = $state(initialParams.get("hotkey") || "Ctrl + Q");
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
  let uiCompact = $state(false);
  let actionNotice = $state("");
  let actionNoticeKind = $state<"success" | "warning" | "error">("success");
  let actionNoticeTimer: number | undefined;
  let updateStatus = $state<UpdateStatus | null>(null);
  let setupStatus = $state<SetupStatus | null>(null);
  let checkingUpdate = $state(false);
  let installingUpdate = $state(false);
  let openingLog = $state(false);
  let testingAsr = $state(false);
  let testingLlm = $state(false);

  onMount(() => {
    const onError = (event: ErrorEvent) => {
      logFrontendError(`${event.message} (${event.filename}:${event.lineno}:${event.colno})`);
    };
    const onUnhandledRejection = (event: PromiseRejectionEvent) => {
      const reason = event.reason instanceof Error ? event.reason.stack || event.reason.message : String(event.reason);
      logFrontendError(`unhandled rejection: ${reason}`);
    };
    window.addEventListener("error", onError);
    window.addEventListener("unhandledrejection", onUnhandledRejection);
    document.getElementById("boot-fallback")?.remove();

    const params = new URLSearchParams(window.location.search);
    isOverlay = params.has("overlay");
    isToast = params.has("toast");
    toastHotkey = params.get("hotkey") || toastHotkey;
    refreshMainDensity();
    window.addEventListener("resize", refreshMainDensity);
    logFrontendEvent(
      `mounted mode=${frontendMode()} viewport=${window.innerWidth}x${window.innerHeight} dpr=${window.devicePixelRatio.toFixed(2)} compact=${uiCompact} language=${navigator.language} userAgent=${navigator.userAgent}`,
    );
    const savedLanguage = localStorage.getItem("voxtype-language");
    if (savedLanguage === "zh-CN" || savedLanguage === "zh-TW" || savedLanguage === "en") {
      language = savedLanguage;
      statusMessage = t("bridgeLoading");
    }
    void bootstrapApp();
    let overlayPoll: number | undefined;
    if (isOverlay) {
      applyOverlayText(overlayText, true);
      window.addEventListener("resize", refreshOverlayLayout);
      overlayPoll = window.setInterval(() => {
        void refreshOverlayText();
      }, 80);
    }
    let unlisteners: Array<Promise<() => void>> = [];
    if (hasTauriApi()) {
      const unlistenSession = listen<SessionState>("session-state-changed", (event) => {
        applySessionState(event.payload);
      });
      const unlistenAsr = listen<AsrFinalText>("asr-final-text", (event) => {
        if (event.payload.error) {
          statusMessage = event.payload.error;
          showActionNotice(event.payload.error, "error");
          if (shouldOpenSettingsForError(event.payload.error)) selectedSection = "Settings";
          return;
        }
        if (isOverlay && event.payload.text.trim()) {
          applyOverlayText(event.payload.text);
        }
        if (event.payload.warning) showActionNotice(event.payload.warning, "warning");
        statusMessage = event.payload.warning ? event.payload.warning : t("sessionSucceeded");
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
      const unlistenAudioLevel = listen<AudioLevel>("audio-level", (event) => {
        audioLevel = clampAudioLevel(event.payload.level);
      });
      unlisteners = [
        unlistenSession,
        unlistenAsr,
        unlistenPartial,
        unlistenOverlay,
        unlistenStats,
        unlistenAudioLevel,
      ];
      logFrontendEvent(`listeners registered mode=${frontendMode()}`);
    }
    return () => {
      if (overlayPoll !== undefined) window.clearInterval(overlayPoll);
      if (actionNoticeTimer !== undefined) window.clearTimeout(actionNoticeTimer);
      stopOverlayScroll();
      window.removeEventListener("resize", refreshMainDensity);
      window.removeEventListener("resize", refreshOverlayLayout);
      window.removeEventListener("error", onError);
      window.removeEventListener("unhandledrejection", onUnhandledRejection);
      void Promise.all(unlisteners).then((disposers) => {
        for (const dispose of disposers) dispose();
      });
    };
  });

  function clonePlain<T>(value: T): T {
    return JSON.parse(JSON.stringify(value)) as T;
  }

  function refreshMainDensity() {
    if (isOverlay || isToast) {
      uiCompact = false;
      return;
    }
    uiCompact = window.innerHeight <= 820 || window.innerWidth <= 1260;
  }

  async function bootstrapApp() {
    const startedAt = performance.now();
    logFrontendEvent(`bootstrap started mode=${frontendMode()}`);
    try {
      await loadAll();
      await hydrateSession();
      void maybeAutoCheckUpdate();
      logFrontendEvent(
        `bootstrap completed mode=${frontendMode()} elapsed_ms=${Math.round(performance.now() - startedAt)} config_exists=${configExists} recording=${recording}`,
      );
    } catch (error) {
      logFrontendError(`bootstrap failed: ${formatFrontendError(error)}`);
    }
  }

  function frontendMode() {
    if (isOverlay) return "overlay";
    if (isToast) return "toast";
    return "main";
  }

  function hasTauriApi() {
    return browser && typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
  }

  function logFrontendEvent(message: string) {
    if (!hasTauriApi()) return;
    void invoke("log_frontend_event", { message: truncateLogMessage(message) }).catch(() => undefined);
  }

  function logFrontendError(message: string) {
    if (!hasTauriApi()) return;
    void invoke("log_frontend_error", { message: truncateLogMessage(message) }).catch(() => undefined);
  }

  function truncateLogMessage(message: string) {
    return message.length > 1200 ? `${message.slice(0, 1200)}...` : message;
  }

  function formatFrontendError(error: unknown) {
    if (error instanceof Error) return error.stack || error.message;
    if (typeof error === "string") return error;
    try {
      return JSON.stringify(error);
    } catch {
      return String(error);
    }
  }

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

  async function safeInvoke<T>(command: string, args?: Record<string, unknown>, quiet = false): Promise<T | null> {
    if (!hasTauriApi()) {
      if (!quiet) statusMessage = t("browserPreview");
      return null;
    }
    try {
      return await invoke<T>(command, args);
    } catch (error) {
      if (!quiet) statusMessage = typeof error === "string" ? error : t("browserPreview");
      logFrontendError(`invoke failed command=${command}: ${formatFrontendError(error)}`);
      return null;
    }
  }

  async function toggleRecordingFromUi() {
    if (isSessionBusy()) return;
    const result = await safeInvoke<SessionState>("toggle_recording");
    if (result) applySessionState(result);
  }

  function isSessionBusy() {
    return ["waiting_final_result", "post_editing", "pasting"].includes(sessionPhase);
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
    logFrontendEvent(`loadAll started mode=${frontendMode()}`);
    const [snapshotResult, configResult, statsResult, devicesResult, setupResult] = await Promise.all([
      safeInvoke<AppSnapshot>("get_app_snapshot"),
      safeInvoke<LoadedConfig>("load_app_config"),
      safeInvoke<StatsSnapshot>("get_usage_stats"),
      safeInvoke<AudioDeviceInfo[]>("list_audio_input_devices"),
      safeInvoke<SetupStatus>("get_setup_status"),
    ]);
    const loadedAny = Boolean(snapshotResult || configResult || statsResult || devicesResult || setupResult);
    if (snapshotResult) snapshot = snapshotResult;
    if (configResult) {
      config = configResult.data;
      configExists = configResult.exists;
      const setupMessage = configSetupMessage(configResult);
      if (setupMessage) {
        statusMessage = setupMessage;
      }
    }
    if (statsResult) stats = statsResult;
    if (devicesResult) audioDevices = devicesResult;
    if (setupResult) setupStatus = setupResult;
    if ((snapshotResult || configResult || statsResult) && !configSetupMessage(configResult)) {
      statusMessage = t("bridgeConnected");
    }
    logFrontendEvent(
      `loadAll completed mode=${frontendMode()} snapshot=${Boolean(snapshotResult)} config_loaded=${Boolean(configResult)} config_exists=${configResult?.exists ?? false} stats_records=${statsResult?.history.length ?? 0} audio_devices=${devicesResult?.length ?? 0} setup_ready=${setupResult?.ready ?? false}`,
    );
    return loadedAny;
  }

  async function hydrateSession() {
    logFrontendEvent(`hydrateSession started mode=${frontendMode()}`);
    const result = await safeInvoke<SessionState>("get_session_state");
    if (result) applySessionState(result);
    logFrontendEvent(
      `hydrateSession completed mode=${frontendMode()} state_loaded=${Boolean(result)} recording=${result?.recording ?? false}`,
    );
  }

  function applySessionState(state: SessionState) {
    recording = state.recording;
    sessionPhase = state.phase ?? (state.recording ? "recording" : "idle");
    sessionErrorCode = state.error_code;
    if (!state.recording) audioLevel = 0;
    if (isConfigError(state.message)) {
      statusMessage = state.message;
      selectedSection = "Settings";
      return;
    }
    statusMessage = state.phase === "failed" && state.message ? state.message : sessionPhaseMessage(sessionPhase);
  }

  function sessionPhaseMessage(phase: SessionPhase) {
    const hotkey = formatHotkey(snapshot.hotkey);
    switch (phase) {
      case "starting":
        return t("sessionStarting");
      case "recording":
        return t("sessionRecording", { hotkey });
      case "stopping":
        return t("sessionStopping");
      case "waiting_final_result":
        return t("sessionWaitingFinal");
      case "post_editing":
        return t("sessionPostEditing");
      case "pasting":
        return t("sessionPasting");
      case "succeeded":
        return t("sessionSucceeded");
      case "failed":
        return t("sessionFailed");
      case "idle":
      default:
        return t("sessionIdleHint", { hotkey });
    }
  }

  async function persistConfig() {
    if (saving) return null;
    saving = true;
    try {
      const result = await safeInvoke<LoadedConfig>("save_app_config", { config });
      if (result) {
        config = result.data;
        configExists = result.exists;
        statusMessage = t("configSaved");
      }
      return result;
    } finally {
      saving = false;
    }
  }

  async function saveConfig() {
    const result = await persistConfig();
    if (result) {
      await loadAll();
      showActionNotice(t("configSaved"), "success");
    } else if (statusMessage) {
      showActionNotice(statusMessage, "error");
    }
  }

  async function reloadConfigFromUi() {
    const loaded = await loadAll();
    if (loaded) {
      showActionNotice(t("configReloaded"), "success");
    } else if (hasTauriApi() && statusMessage) {
      showActionNotice(statusMessage, "error");
    }
  }

  async function maybeAutoCheckUpdate() {
    if (isOverlay || isToast || !configExists || !config.update.auto_check_on_startup) return;
    await checkUpdate(false);
  }

  async function checkUpdate(manual = true) {
    if (checkingUpdate) return;
    checkingUpdate = true;
    const previousStatus = statusMessage;
    try {
      const result = await safeInvoke<UpdateStatus>("check_for_update", undefined, !manual);
      if (result) {
        updateStatus = result;
        if (manual || result.update_available) {
          showActionNotice(result.message, result.update_available ? "warning" : "success");
        }
      } else if (manual && statusMessage) {
        showActionNotice(statusMessage, "error");
      } else {
        statusMessage = previousStatus;
      }
    } finally {
      checkingUpdate = false;
    }
  }

  async function downloadLatestUpdate() {
    if (installingUpdate) return;
    installingUpdate = true;
    try {
      const result = await safeInvoke<InstallUpdateResult>("download_and_install_update");
      if (result) {
        showActionNotice(result.message, "success");
      } else if (statusMessage) {
        showActionNotice(statusMessage, "error");
      }
    } finally {
      installingUpdate = false;
    }
  }

  async function openLogFromUi() {
    if (openingLog) return;
    if (!hasTauriApi()) {
      statusMessage = t("browserPreview");
      showActionNotice(statusMessage, "error");
      return;
    }
    openingLog = true;
    try {
      await invoke("open_log_file");
      showActionNotice(t("logOpened"), "success");
    } catch (error) {
      statusMessage = typeof error === "string" ? error : t("browserPreview");
      logFrontendError(`open log failed: ${formatFrontendError(error)}`);
      showActionNotice(statusMessage, "error");
    } finally {
      openingLog = false;
    }
  }

  async function testAsrConfig() {
    if (testingAsr) return;
    testingAsr = true;
    try {
      const result = await safeInvoke<ConnectionTestResult>("test_asr_config", { config: clonePlain(config) });
      if (result) {
        statusMessage = result.message;
        showActionNotice(result.message, "success");
      } else if (statusMessage) {
        showActionNotice(statusMessage, "error");
      }
    } finally {
      testingAsr = false;
    }
  }

  async function testLlmConfig() {
    if (testingLlm) return;
    testingLlm = true;
    try {
      const result = await safeInvoke<ConnectionTestResult>("test_llm_config", { config: clonePlain(config) });
      if (result) {
        statusMessage = result.message;
        showActionNotice(result.message, "success");
      } else if (statusMessage) {
        showActionNotice(statusMessage, "error");
      }
    } finally {
      testingLlm = false;
    }
  }

  function showActionNotice(message: string, kind: "success" | "warning" | "error") {
    actionNotice = message;
    actionNoticeKind = kind;
    if (actionNoticeTimer !== undefined) window.clearTimeout(actionNoticeTimer);
    actionNoticeTimer = window.setTimeout(() => {
      actionNotice = "";
      actionNoticeTimer = undefined;
    }, 2800);
  }

  async function toggleTrigger(key: TriggerKey) {
    if (saving) return;
    const previous = config.triggers[key];
    config.triggers[key] = !previous;
    const result = await persistConfig();
    if (!result) {
      config.triggers[key] = previous;
      if (statusMessage) showActionNotice(statusMessage, "error");
      return;
    }
    showActionNotice(t("configSaved"), "success");
  }

  function triggerLabel(enabled: boolean) {
    return enabled ? t("enabled") : t("disabled");
  }

  function updatePanelTitle() {
    if (!updateStatus) return t("updateNotChecked");
    if (updateStatus.update_available) return t("updateAvailable");
    return t("updateUpToDate");
  }

  function updatePanelDescription() {
    if (!updateStatus) return t("updateIdleDescription");
    if (updateStatus.update_available && updateStatus.asset_name) {
      return t("updateReady", { asset: updateStatus.asset_name });
    }
    if (updateStatus.update_available) return t("updateNoInstaller");
    return updateStatus.message;
  }

  function updateMetaText() {
    const current = updateStatus?.current_version ?? snapshot.current_version;
    const latest = updateStatus?.latest_version ?? "-";
    const size = updateStatus?.asset_size ? ` · ${formatFileSize(updateStatus.asset_size)}` : "";
    return `${t("currentVersion")} v${current} · ${t("latestVersion")} ${latest === "-" ? "-" : `v${latest}`}${size}`;
  }

  function formatFileSize(bytes: number) {
    if (!Number.isFinite(bytes) || bytes <= 0) return "";
    if (bytes < 1024 * 1024) return `${Math.ceil(bytes / 1024)} KB`;
    return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  }

  function clampAudioLevel(value: number) {
    if (!Number.isFinite(value)) return 0;
    return Math.max(0, Math.min(1, value));
  }

  function meterLevel() {
    return recording ? clampAudioLevel(audioLevel * 3.2) : 0;
  }

  function micBarHeight(index: number) {
    const level = meterLevel();
    const quietHeights = [5, 7, 9, 11, 9, 7];
    const activeHeights = [7, 11, 15, 19, 16, 12];
    const threshold = 0.08 + index * 0.105;
    const target = level >= threshold ? activeHeights[index] : quietHeights[index];
    return `${target}px`;
  }

  function micBarOpacity(index: number) {
    if (!recording) return "0.45";
    const level = meterLevel();
    return level >= 0.08 + index * 0.105 ? "1" : "0.38";
  }

  function currentAudioDevice() {
    if (audioDevices.length === 0) return null;
    if (config.audio.input_device !== null && config.audio.input_device !== undefined) {
      const configured = audioDevices.find((device) => device.index === config.audio.input_device);
      if (configured) return configured;
    }
    return audioDevices.find((device) => device.is_default) ?? audioDevices[0];
  }

  async function refreshSetupStatus() {
    const [devicesResult, setupResult] = await Promise.all([
      safeInvoke<AudioDeviceInfo[]>("list_audio_input_devices", undefined, true),
      safeInvoke<SetupStatus>("get_setup_status", undefined, true),
    ]);
    if (devicesResult) audioDevices = devicesResult;
    if (setupResult) setupStatus = setupResult;
  }

  function setupStatusItems(): SetupStatusItem[] {
    const status = setupStatus;
    const authReady = hasAuth();
    const micReady = status ? status.has_audio_device : audioDevices.length > 0;
    return [
      {
        label: t("setupAuthLabel"),
        value: authReady ? t("setupOk") : t("setupMissing"),
        ok: authReady,
        action: "asr_auth",
      },
      {
        label: t("setupMicLabel"),
        value: micReady ? t("setupMicDetected") : t("setupMicMissing"),
        ok: micReady,
        action: "audio",
      },
      {
        label: t("setupPasteLabel"),
        value: pasteMethodLabel(config.typing.paste_method),
        ok: true,
        action: "typing",
      },
      {
        label: t("setupTriggerLabel"),
        value: formatEnabledTriggers(),
        ok: config.triggers.hotkey_enabled,
        action: "hotkey",
      },
      {
        label: t("setupPrivacyLabel"),
        value: config.context.enable_recent_context ? t("setupRecentOn") : t("setupRecentOff"),
        ok: !config.context.enable_recent_context,
        action: "privacy",
      },
    ];
  }

  function setupWarningCount() {
    const warnings = setupStatus?.warnings.length ?? 0;
    const blocking = setupStatusItems().filter((item) => !item.ok).length;
    return Math.max(warnings, blocking);
  }

  function setupIsReady() {
    return setupStatusItems().every((item) => item.ok);
  }

  function setupActionText(action: string) {
    if (action === "asr_auth") return t("setupActionAsr");
    if (action === "audio") return t("setupActionAudio");
    if (action === "typing") return t("setupActionTyping");
    if (action === "hotkey") return t("setupActionHotkey");
    if (action === "privacy") return t("setupActionPrivacy");
    return t("setupCta");
  }

  function handleSetupAction(action: string) {
    if (action === "audio") void refreshSetupStatus();
    selectedSection = "Settings";
    const targetId =
      action === "asr_auth"
        ? "settings-auth"
        : action === "audio"
          ? "settings-audio"
          : action === "typing"
            ? "settings-output"
            : action === "privacy"
              ? "settings-context"
              : "settings-output";
    window.setTimeout(() => {
      document.getElementById(targetId)?.scrollIntoView({ block: "start", behavior: "smooth" });
    }, 50);
  }

  function pasteMethodLabel(value: string) {
    if (value === "clipboard_only") return t("clipboardOnly");
    if (value === "shift_insert") return "Shift + Insert";
    return "Ctrl + V";
  }

  function formatEnabledTriggers() {
    const triggers = [];
    if (config.triggers.hotkey_enabled) triggers.push(formatHotkey(snapshot.hotkey));
    if (config.triggers.middle_mouse_enabled) triggers.push(t("middleMouse"));
    if (config.triggers.right_alt_enabled) triggers.push(t("rightAlt"));
    return triggers.length > 0 ? triggers.join(" / ") : t("disabled");
  }

  function micStatusText() {
    const device = currentAudioDevice();
    if (!device) return t("micUnavailable");
    return recording
      ? t("micMonitoring", { device: device.name })
      : t("micConnected", { device: device.name });
  }

  function sidebarMicStatusText() {
    return currentAudioDevice() ? t("sidebarMicConnected") : t("sidebarMicUnavailable");
  }

  function usageTipText() {
    if (stats.recent_7d.session_count <= 0) return t("usageTipEmpty");
    return t("usageTipData", {
      sessions: formatNumber(stats.recent_7d.session_count),
      chars: formatNumber(stats.recent_7d.total_chars),
    });
  }

  function formatHotkey(value: string) {
    return value
      .split("+")
      .map((part) => {
        const normalized = part.trim().toLowerCase();
        if (normalized === "ctrl" || normalized === "control") return "Ctrl";
        if (normalized === "alt") return "Alt";
        if (normalized === "shift") return "Shift";
        if (normalized === "win" || normalized === "meta") return "Win";
        return part.trim().toUpperCase();
      })
      .filter(Boolean)
      .join(" + ");
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

  function formatNumber(value: number) {
    return new Intl.NumberFormat(language).format(Math.round(value || 0));
  }

  function inputStatus() {
    if (sessionPhase === "failed" || isErrorStatus(statusMessage)) return "error";
    return recording || isSessionBusy() ? "listening" : "idle";
  }

  function inputStatusLabel() {
    const status = inputStatus();
    if (status === "error") return isConfigError(statusMessage) ? t("setupRequired") : t("inputError");
    if (sessionPhase === "starting") return t("sessionStarting");
    if (sessionPhase === "stopping") return t("sessionStopping");
    if (sessionPhase === "waiting_final_result") return t("sessionWaitingFinal");
    if (sessionPhase === "post_editing") return t("sessionPostEditing");
    if (sessionPhase === "pasting") return t("sessionPasting");
    if (sessionPhase === "succeeded") return t("sessionSucceeded");
    return recording ? t("recordingPreview") : t("idle");
  }

  function inputStatusDesc() {
    const status = inputStatus();
    if (status === "error") return statusMessage;
    return sessionPhaseMessage(sessionPhase);
  }

  function weeklySavedHours() {
    const typingHours = stats.recent_7d.total_chars / chineseTypingCharsPerMinute / 60;
    const recordingHours = stats.recent_7d.total_seconds / 3600;
    return Math.max(0, typingHours - recordingHours);
  }

  function savedHoursForUsage(usage: UsageStats) {
    const typingHours = usage.total_chars / chineseTypingCharsPerMinute / 60;
    const recordingHours = usage.total_seconds / 3600;
    return Math.max(0, typingHours - recordingHours);
  }

  function formatHours(hours: number) {
    if (hours < 0.05) return "0 h";
    return `${hours.toFixed(1)} h`;
  }

  function formatSavedHours(hours: number) {
    const value = hours < 0.05 ? "0" : hours.toFixed(1);
    if (language === "en") return `${value} h`;
    return `${value} ${language === "zh-TW" ? "小時" : "小时"}`;
  }

  function localDateKey(date: Date) {
    const year = date.getFullYear();
    const month = String(date.getMonth() + 1).padStart(2, "0");
    const day = String(date.getDate()).padStart(2, "0");
    return `${year}-${month}-${day}`;
  }

  function recentSevenDayRows() {
    const byDay = new Map(stats.by_day.map((day) => [day.day, day.stats]));
    const today = new Date();
    return Array.from({ length: 7 }, (_, index) => {
      const date = new Date(today);
      date.setDate(today.getDate() - index);
      const day = localDateKey(date);
      return {
        day,
        stats: byDay.get(day) ?? emptyUsage(),
      };
    });
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
      message.includes("access_key") ||
      message.includes("豆包 ASR 认证") ||
      message.includes("App Key") ||
      message.includes("Access Key") ||
      message.includes("Resource ID")
    );
  }

  function isErrorStatus(message: string) {
    return (
      isConfigError(message) ||
      message.includes("无法连接豆包 ASR") ||
      message.includes("连接豆包 ASR 失败") ||
      message.includes("豆包 ASR 服务返回错误码") ||
      message.includes("开机自启动设置失败") ||
      message.includes("启动录音失败")
    );
  }

  function shouldOpenSettingsForError(message: string) {
    return isConfigError(message) || message.includes("API Key") || message.includes("Base URL");
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
<div class:ui-compact={uiCompact} class="app-frame">
<header class="window-titlebar" data-tauri-drag-region>
  <div class="window-title" data-tauri-drag-region>
    <span class="window-title-mark"><Mic size={12} strokeWidth={2.6} /></span>
    <strong data-tauri-drag-region>{t("appTitle")}</strong>
    <span data-tauri-drag-region>VoxType</span>
  </div>
  <div class="window-controls">
    <button class="tray-action" aria-label={t("minimizeToTray")} title={t("minimizeToTray")} onclick={closeWindow}>
      <Download size={15} />
      <span>{t("minimizeToTray")}</span>
    </button>
    <button aria-label="最小化" title="最小化" onclick={minimizeWindow}><Minus size={13} /></button>
    <button aria-label="最大化或还原" title="最大化或还原" onclick={toggleMaximizeWindow}><Maximize2 size={12} /></button>
    <button class="close" aria-label="关闭" title="关闭" onclick={closeWindow}><XIcon size={14} /></button>
  </div>
</header>
<main class="shell">
  <aside class="sidebar">
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

    <section class:error={inputStatus() === "error"} class:listening={recording} class="bridge-card">
      <div class="bridge-top">
        <span class="pulse" class:recording class:error={inputStatus() === "error"}></span>
        <span>{inputStatusLabel()}</span>
      </div>
      <p>{inputStatusDesc()}</p>
      <div class:active={recording} class="mic-line">
        <span title={micStatusText()}>{sidebarMicStatusText()}</span>
        {#if recording}
          {#each micBars as bar}
            <i style:height={micBarHeight(bar)} style:opacity={micBarOpacity(bar)}></i>
          {/each}
        {/if}
      </div>
      <div class="shortcut-line">{t("sidebarShortcut", { hotkey: formatHotkey(snapshot.hotkey) })}</div>
    </section>
  </aside>

  <section
    class:overview-content={selectedSection === "Overview"}
    class:setup-required={!configExists || !hasAuth()}
    class="content"
  >
    <header class="topbar">
      <div>
        <p class="eyebrow">{t("topEyebrow")}</p>
        <h2>{selectedSection === "Overview" ? t("navOverview") : t(navLabelKeys[selectedSection])}</h2>
      </div>
    </header>

    {#if selectedSection === "Overview"}
      <SetupStatusCard
        ready={setupIsReady()}
        items={setupStatusItems()}
        warnings={setupStatus?.warnings ?? []}
        texts={{
          title: t("setupHealthTitle"),
          pendingTitle: t("setupHealthPendingTitle", { count: String(setupWarningCount()) }),
          pendingDescription: t("setupHealthPendingDescription"),
          readyTitle: t("setupHealthReadyTitle"),
          readyDescription: t("setupHealthReadyDescription", { hotkey: formatHotkey(snapshot.hotkey) }),
          refresh: t("refreshSetup"),
          actionText: setupActionText,
        }}
        onAction={handleSetupAction}
        onRefresh={refreshSetupStatus}
      />

      <section class="voice-card">
        <div class="section-title-row">
          <h3>{t("voiceInputTitle")}</h3>
        </div>
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
        <div class:listening={recording} class:error={inputStatus() === "error"} class="voice-hero">
          <button class:listening={recording || isSessionBusy()} class="mic-orb" aria-label={recording ? t("clickStop") : t("clickStart")} onclick={toggleRecordingFromUi} disabled={isSessionBusy()}>
            <span class="mic-ring"><Mic size={uiCompact ? 34 : 42} strokeWidth={2.15} /></span>
          </button>
          <div class="voice-copy">
            <div class="hero-status">
              <span class="hero-dot" class:listening={recording} class:error={inputStatus() === "error"}></span>
              <strong>{inputStatusLabel()}</strong>
            </div>
            <h4>{recording ? t("clickStop") : isSessionBusy() ? inputStatusLabel() : t("clickStart")}</h4>
            <p>{inputStatusDesc()}</p>
            <div class="hero-features">
              <span><MessageSquareText size={17} />{t("speakAnywhere")}</span>
              <span><Globe2 size={17} />{t("mixedInput")}</span>
            </div>
          </div>
          <button class="shortcut-help" onclick={() => (selectedSection = "Settings")}>
            {t("shortcutSettings")}
            <ChevronRight size={16} />
          </button>
        </div>
      </section>

      <section class="launch-card">
        <div class="section-title-row">
          <div>
            <Keyboard size={20} />
            <h3>{t("desktopControl")}</h3>
          </div>
          <button class="link-action" onclick={() => (selectedSection = "Settings")}>
            {t("shortcutSettings")} <ChevronRight size={16} />
          </button>
        </div>
        <div class="trigger-grid">
          <label
            class:active={config.triggers.hotkey_enabled}
            class:disabled={saving}
            class="trigger-item"
          >
            <input class="trigger-input" type="checkbox" checked={config.triggers.hotkey_enabled} disabled={saving} onchange={() => toggleTrigger("hotkey_enabled")} />
            <span class="trigger-check">
              {#if config.triggers.hotkey_enabled}<Check size={uiCompact ? 18 : 24} />{/if}
            </span>
            <div>
              <strong>{formatHotkey(snapshot.hotkey)}</strong>
              <p>{config.triggers.hotkey_enabled ? t("mainHotkey") : t("disabled")}</p>
            </div>
          </label>
          <label
            class:active={config.triggers.middle_mouse_enabled}
            class:disabled={saving}
            class="trigger-item"
          >
            <input class="trigger-input" type="checkbox" checked={config.triggers.middle_mouse_enabled} disabled={saving} onchange={() => toggleTrigger("middle_mouse_enabled")} />
            <span class="trigger-check">
              {#if config.triggers.middle_mouse_enabled}<Check size={uiCompact ? 18 : 24} />{/if}
            </span>
            <div>
              <strong>{t("middleMouse")}</strong>
              <p>{triggerLabel(config.triggers.middle_mouse_enabled)}</p>
            </div>
          </label>
          <label
            class:active={config.triggers.right_alt_enabled}
            class:disabled={saving}
            class="trigger-item"
          >
            <input class="trigger-input" type="checkbox" checked={config.triggers.right_alt_enabled} disabled={saving} onchange={() => toggleTrigger("right_alt_enabled")} />
            <span class="trigger-check">
              {#if config.triggers.right_alt_enabled}<Check size={uiCompact ? 18 : 24} />{/if}
            </span>
            <div>
              <strong>{t("rightAlt")}</strong>
              <p>{triggerLabel(config.triggers.right_alt_enabled)}</p>
            </div>
          </label>
        </div>
      </section>

      <section class="performance-card">
        <div class="section-title-row">
          <h3>{t("recentUsage")}</h3>
        </div>
        <div class="stats-row" aria-label="Usage summary">
          <article class="stat-card blue">
            <span class="stat-icon"><PenLine size={uiCompact ? 16 : 20} /></span>
            <p>{t("todayInput")}</p>
            <strong>{formatNumber(stats.recent_24h.total_chars)} {t("chars")}</strong>
            <small>{t("savedToday", { hours: formatHours(stats.recent_24h.total_chars / chineseTypingCharsPerMinute / 60).replace(" h", "") })}</small>
          </article>
          <article class="stat-card purple">
            <span class="stat-icon"><CalendarDays size={uiCompact ? 16 : 20} /></span>
            <p>{t("recent7d")}</p>
            <strong>{formatNumber(stats.recent_7d.total_chars)} {t("chars")}</strong>
            <small>{t("savedToday", { hours: formatHours(stats.recent_7d.total_chars / chineseTypingCharsPerMinute / 60).replace(" h", "") })}</small>
          </article>
          <article class="stat-card green">
            <span class="stat-icon"><Zap size={uiCompact ? 16 : 20} /></span>
            <p>{t("inputSpeed")}</p>
            <strong>{stats.recent_7d.avg_chars_per_minute.toFixed(0)} {t("perMinute")}</strong>
            <small>{t("avgCpm")}</small>
          </article>
          <article class="stat-card orange">
            <span class="stat-icon"><Clock3 size={uiCompact ? 16 : 20} /></span>
            <p>{t("savedTime")}</p>
            <strong>{formatSavedHours(weeklySavedHours())}</strong>
            <small>{t("weeklySavedShort")}</small>
          </article>
        </div>
        <p class="usage-tip"><Sparkles size={15} />{usageTipText()}</p>
      </section>
    {:else if selectedSection === "Settings"}
      <section class="settings-stack">
        <section class="settings-group">
          <div class="settings-group-heading">
            <h3>{t("softwareSettings")}</h3>
            <p>{t("softwareSettingsDescription")}</p>
          </div>

          <div id="settings-output" class="form-panel">
            <div class="section-heading"><h3>{t("startAndOutput")}</h3><p>{t("typingDescription")}</p></div>
            <div class="form-grid">
              <label><span>{t("hotkey")}</span><input value={formatHotkey(config.hotkey)} oninput={(event) => setHotkey(event.currentTarget.value)} /></label>
              <label><span>{t("pasteDelayMs")}</span><input type="number" bind:value={config.typing.paste_delay_ms} /></label>
              <label><span>{t("pasteMethod")}</span><select bind:value={config.typing.paste_method}><option value="ctrl_v">Ctrl + V</option><option value="shift_insert">Shift + Insert</option><option value="clipboard_only">{t("clipboardOnly")}</option></select></label>
              <label><span>{t("clipboardRetryCount")}</span><input type="number" bind:value={config.typing.clipboard_open_retry_count} /></label>
              <label><span>{t("clipboardRetryInterval")}</span><input type="number" bind:value={config.typing.clipboard_open_retry_interval_ms} /></label>
            </div>
            <div class="toggle-grid">
              <label class="check"><input type="checkbox" bind:checked={config.triggers.hotkey_enabled} />{t("mainHotkey")}</label>
              <label class="check"><input type="checkbox" bind:checked={config.triggers.middle_mouse_enabled} />{t("middleMouse")}</label>
              <label class="check"><input type="checkbox" bind:checked={config.triggers.right_alt_enabled} />{t("rightAlt")}</label>
              <label class="check"><input type="checkbox" bind:checked={config.typing.restore_clipboard_after_paste} />{t("restoreClipboardAfterPaste")}</label>
              <label class="check"><input type="checkbox" bind:checked={config.startup.launch_on_startup} />{t("launchOnStartup")}</label>
            </div>
            <p class="field-hint">{t("triggerConflictHint")}</p>
          </div>

          <div class="form-panel">
            <div class="section-heading"><h3>{t("floatingCaptionAndTray")}</h3><p>{t("interfaceDescription")}</p></div>
            <div class="form-grid">
              <label><span>{t("width")}</span><input type="number" bind:value={config.ui.width} /></label>
              <label><span>{t("height")}</span><input type="number" bind:value={config.ui.height} /></label>
              <label><span>{t("marginBottom")}</span><input type="number" bind:value={config.ui.margin_bottom} /></label>
              <label><span>{t("opacity")}</span><input type="number" step="0.05" bind:value={config.ui.opacity} /></label>
              <label><span>{t("scrollInterval")}</span><input type="number" bind:value={config.ui.scroll_interval_ms} /></label>
              <label><span>{t("startupTimeout")}</span><input type="number" bind:value={config.tray.startup_message_timeout_ms} /></label>
            </div>
            <div class="toggle-grid">
              <label class="check"><input type="checkbox" bind:checked={config.tray.show_startup_message} />{t("showStartupMessage")}</label>
            </div>
          </div>

          <div class="form-panel update-panel">
            <div class="section-heading"><h3>{t("softwareUpdate")}</h3><p>{t("softwareUpdateDescription")}</p></div>
            <div class:available={updateStatus?.update_available} class="update-card">
              <div>
                <strong>{updatePanelTitle()}</strong>
                <p>{updatePanelDescription()}</p>
                <small>{updateMetaText()}</small>
              </div>
              <div class="update-actions">
                <button onclick={() => checkUpdate(true)} disabled={checkingUpdate}>
                  <ShieldCheck size={16} />{checkingUpdate ? t("checkingUpdates") : t("checkUpdates")}
                </button>
                {#if updateStatus?.update_available && updateStatus.asset_name}
                  <button class="primary" onclick={downloadLatestUpdate} disabled={installingUpdate}>
                    <Download size={16} />{installingUpdate ? t("downloadingInstall") : t("downloadInstall")}
                  </button>
                {/if}
              </div>
            </div>
            <div class="toggle-grid">
              <label class="check"><input type="checkbox" bind:checked={config.update.auto_check_on_startup} />{t("autoCheckUpdates")}</label>
            </div>
          </div>

          <div class="form-panel">
            <div class="section-heading"><h3>{t("diagnosticsAndLogs")}</h3><p>{t("diagnosticsDescription")}</p></div>
            <div class="update-card">
              <div>
                <strong>{t("logStatusTitle")}</strong>
                <p>{t("logStatusDescription")}</p>
              </div>
              <div class="update-actions">
                <button onclick={openLogFromUi} disabled={openingLog}>
                  <FileText size={16} />{openingLog ? t("openingLog") : t("openLog")}
                </button>
              </div>
            </div>
          </div>
        </section>

        <section class="settings-group">
          <div class="settings-group-heading">
            <h3>{t("doubaoAsrSettings")}</h3>
            <p>{t("doubaoAsrSettingsDescription")}</p>
          </div>

          <div id="settings-auth" class="form-panel">
            <div class="section-heading with-actions">
              <div>
                <h3>{t("doubaoAuth")}</h3>
                {#if !configExists || !hasAuth()}
                  <p class="setup-note">{!configExists ? t("setupMissingFile") : t("setupMissingAuth")}</p>
                  <button class="link-button" onclick={openSetupGuide}>{t("setupGuideCta")}</button>
                {/if}
              </div>
              <button class="test-button" onclick={testAsrConfig} disabled={testingAsr}>
                <ShieldCheck size={16} />{testingAsr ? t("testingConnection") : t("testConnection")}
              </button>
            </div>
            <div class="form-grid">
              <label><span>{t("resourceId")}</span><input bind:value={config.auth.resource_id} /></label>
              <label><span>{t("appKey")}</span><input autocomplete="off" bind:value={config.auth.app_key} /></label>
              <label><span>{t("accessKey")}</span><input type="password" autocomplete="off" bind:value={config.auth.access_key} /></label>
            </div>
          </div>

          <div id="settings-audio" class="form-panel">
            <div class="section-heading"><h3>{t("recordingParams")}</h3><p>{t("audioDescription")}</p></div>
            <div class="form-grid">
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
            <div class="toggle-grid">
              <label class="check"><input type="checkbox" bind:checked={config.audio.mute_system_volume_while_recording} />{t("muteSystemAudio")}</label>
            </div>
            <p class="field-hint">{t("muteSystemAudioHint")}</p>
          </div>

          <div class="form-panel">
            <div class="section-heading"><h3>{t("recognitionOptions")}</h3><p>{t("asrDescription")}</p></div>
            <label><span>{t("websocketUrl")}</span><input bind:value={config.request.ws_url} /></label>
            <div class="form-grid">
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

          <div id="settings-context" class="form-panel">
            <div class="section-heading"><h3>{t("context")}</h3><p>{t("contextDescription")}</p></div>
            <label><span>{t("hotwords")}</span><textarea value={config.context.hotwords.join("\n")} oninput={(event) => updateHotwords(event.currentTarget.value)}></textarea></label>
            <label><span>{t("promptContext")}</span><textarea value={config.context.prompt_context.map((item) => item.text).join("\n")} oninput={(event) => updatePromptContext(event.currentTarget.value)}></textarea></label>
            <label><span>{t("imageUrl")}</span><input value={config.context.image_url ?? ""} oninput={(event) => setOptionalImageUrl(event.currentTarget.value)} /></label>
            <div class="toggle-grid">
              <label class="check"><input type="checkbox" bind:checked={config.context.enable_recent_context} />{t("useRecentContext")}</label>
            </div>
            <p class="field-hint">{t("recentContextHint")}</p>
          </div>
        </section>

        <section class="settings-group">
          <div class="settings-group-heading">
            <h3>{t("llmSettings")}</h3>
            <p>{t("llmSettingsDescription")}</p>
          </div>

          <div class="form-panel">
            <div class="section-heading with-actions">
              <div><h3>{t("llmPostEdit")}</h3><p>{t("llmDescription")}</p></div>
              <button class="test-button" onclick={testLlmConfig} disabled={testingLlm}>
                <ShieldCheck size={16} />{testingLlm ? t("testingConnection") : t("testConnection")}
              </button>
            </div>
            <label class="check"><input type="checkbox" bind:checked={config.llm_post_edit.enabled} />{t("enablePolishing")}</label>
            <div class="form-grid">
              <label><span>{t("minChars")}</span><input type="number" bind:value={config.llm_post_edit.min_chars} /></label>
              <label><span>{t("timeout")}</span><input type="number" bind:value={config.llm_post_edit.timeout_seconds} /></label>
              <label><span>Base URL</span><input bind:value={config.llm_post_edit.base_url} /></label>
              <label><span>{t("model")}</span><input bind:value={config.llm_post_edit.model} /></label>
              <label><span>API Key</span><input type="password" autocomplete="off" bind:value={config.llm_post_edit.api_key} /></label>
            </div>
            <label><span>{t("systemPrompt")}</span><textarea bind:value={config.llm_post_edit.system_prompt}></textarea></label>
            <label><span>{t("userPromptTemplate")}</span><textarea bind:value={config.llm_post_edit.user_prompt_template}></textarea></label>
          </div>
        </section>

        <div class="form-actions">
          <button class="primary" onclick={saveConfig} disabled={saving}><Save size={16} />{saving ? t("saving") : t("saveConfig")}</button>
          <button onclick={reloadConfigFromUi}><ShieldCheck size={16} />{t("reload")}</button>
        </div>
      </section>
    {:else if selectedSection === "History"}
      <section class="history-page">
      <section class="history-summary">
        <article class="history-card blue">
          <p>{t("todayInput")}</p>
          <strong>{formatNumber(stats.recent_24h.total_chars)} {t("chars")}</strong>
          <span>{t("savedToday", { hours: formatHours(stats.recent_24h.total_chars / chineseTypingCharsPerMinute / 60).replace(" h", "") })}</span>
        </article>
        <article class="history-card purple">
          <p>{t("recent7d")}</p>
          <strong>{formatNumber(stats.recent_7d.total_chars)} {t("chars")}</strong>
          <span>{t("savedToday", { hours: formatHours(weeklySavedHours()).replace(" h", "") })}</span>
        </article>
        <article class="history-card green">
          <p>{t("avgCpm")}</p>
          <strong>{stats.recent_7d.avg_chars_per_minute.toFixed(0)} {t("perMinute")}</strong>
          <span>{t("weeklySavedHoursHint")}</span>
        </article>
        <article class="history-card orange">
          <p>{t("savedTime")}</p>
          <strong>{formatSavedHours(weeklySavedHours())}</strong>
          <span>{t("weeklySavedShort")}</span>
        </article>
      </section>
      <section class="daily-panel form-panel">
          <div class="section-heading"><h3>{t("byDay")}</h3><p>{t("lastSevenDays")}</p></div>
          <div class="day-list">
            <div class="day-list-head">
              <span>{t("dateColumn")}</span>
              <span>{t("dailyInputChars")}</span>
              <span>{t("voiceDuration")}</span>
              <span>{t("averageInputSpeed")}</span>
              <span>{t("dailySavedTime")}</span>
            </div>
            {#each recentSevenDayRows() as day}
              <article>
                <span>{day.day}</span>
                <strong>{formatNumber(day.stats.total_chars)} {t("chars")}</strong>
                <span>{formatSeconds(day.stats.total_seconds)}</span>
                <span>{day.stats.avg_chars_per_minute.toFixed(0)} {t("perMinute")}</span>
                <strong>{formatSavedHours(savedHoursForUsage(day.stats))}</strong>
              </article>
            {/each}
          </div>
      </section>
      </section>
    {/if}

  </section>
</main>
{#if actionNotice}
  <div class:error={actionNoticeKind === "error"} class:warning={actionNoticeKind === "warning"} class="action-notice" role="status" aria-live="polite">
    {#if actionNoticeKind === "success"}
      <Check size={16} />
    {:else}
      <AlertCircle size={16} />
    {/if}
    <span>{actionNotice}</span>
  </div>
{/if}
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
  :global(button), :global(input), :global(textarea), :global(select) {
    font: inherit;
    transition: all 160ms ease;
  }
  :global(button) {
    cursor: pointer;
    border: 0;
    background: transparent;
  }
  :global(:root) {
    --primary: #2f80ed;
    --primary-hover: #256fe0;
    --primary-light: #eaf3ff;
    --gradient-start: #2f80ed;
    --gradient-end: #7c3aed;
    --text-main: #111827;
    --text-secondary: #64748b;
    --text-muted: #94a3b8;
    --bg-page: #f8fafc;
    --bg-sidebar: #f3f8ff;
    --bg-card: #ffffff;
    --border: #dde6f3;
    --border-strong: #cbd5e1;
    --success: #10b981;
    --warning: #f59e0b;
    --danger: #ef4444;
    --radius-sm: 8px;
    --radius-md: 12px;
    --radius-lg: 16px;
    --radius-xl: 24px;
    --shadow-card: 0 8px 24px rgba(15, 23, 42, 0.06);
    --shadow-soft: 0 4px 12px rgba(15, 23, 42, 0.08);
  }

  :global(html:not(:has(.overlay-root)):not(:has(.toast-root))),
  :global(body:not(:has(.overlay-root)):not(:has(.toast-root))) {
    background: var(--bg-page);
    font-family: "Microsoft YaHei", "Segoe UI", "PingFang SC", "SF Pro Display", "Noto Sans CJK SC", sans-serif;
  }

  .app-frame {
    position: relative;
    display: grid;
    grid-template-rows: 48px minmax(0, 1fr);
    width: 100vw;
    height: 100vh;
    overflow: hidden;
    background: var(--bg-page);
  }

  .app-frame.ui-compact {
    grid-template-rows: 44px minmax(0, 1fr);
  }

  .window-titlebar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 48px;
    padding: 0 18px;
    background: #ffffff;
    border-bottom: 1px solid var(--border);
    box-shadow: 0 1px 0 rgba(15, 23, 42, 0.02);
    user-select: none;
    -webkit-app-region: drag;
  }

  .ui-compact .window-titlebar {
    height: 44px;
    padding: 0 16px;
  }

  .window-title {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
    overflow: hidden;
    color: var(--text-main);
    font-size: 15px;
    font-weight: 400;
    text-transform: none;
  }

  .window-title strong {
    min-width: 0;
    overflow: hidden;
    font-size: 16px;
    font-weight: 700;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .window-title > span:last-child {
    min-width: 0;
    overflow: hidden;
    color: var(--text-secondary);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .window-title-mark {
    display: grid;
    width: 28px;
    height: 28px;
    flex: 0 0 auto;
    place-items: center;
    color: #ffffff;
    background: linear-gradient(135deg, var(--gradient-start), var(--gradient-end));
    border: 0;
    border-radius: 10px;
    box-shadow: 0 6px 16px rgba(47, 128, 237, 0.24);
  }

  .window-controls {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 0 0 auto;
    -webkit-app-region: no-drag;
  }

  .window-controls button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    color: var(--text-main);
    background: #ffffff;
    border: 1px solid transparent;
    border-radius: 10px;
    transition: all 160ms ease;
    -webkit-app-region: no-drag;
  }

  .ui-compact .window-controls button {
    width: 30px;
    height: 30px;
  }

  .ui-compact .window-controls .tray-action {
    width: auto;
    padding: 0 10px;
    font-size: 13px;
  }

  .window-controls button:hover {
    color: var(--text-main);
    background: #f1f5f9;
    border-color: var(--border);
  }

  .window-controls .tray-action {
    display: inline-flex;
    width: auto;
    max-width: 170px;
    gap: 9px;
    padding: 0 12px;
    color: var(--text-secondary);
    background: #fbfdff;
    border-color: var(--border);
    font-size: 14px;
    font-weight: 500;
  }

  .window-controls .tray-action span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .window-controls button.close:hover {
    color: #ffffff;
    background: var(--danger);
    border-color: var(--danger);
  }

  .shell {
    display: grid;
    grid-template-columns: 230px minmax(0, 1fr);
    min-height: 0;
    overflow: hidden;
    background: var(--bg-page);
  }

  .ui-compact .shell {
    grid-template-columns: 212px minmax(0, 1fr);
  }

  .sidebar {
    display: flex;
    flex-direction: column;
    gap: 14px;
    min-width: 0;
    min-height: 0;
    padding: 18px 20px;
    overflow: hidden;
    background: var(--bg-sidebar);
    border-right: 1px solid var(--border);
  }

  .ui-compact .sidebar {
    gap: 11px;
    padding: 14px 16px;
  }

  nav {
    display: grid;
    gap: 8px;
  }

  .ui-compact nav {
    gap: 6px;
  }

  nav button {
    display: flex;
    align-items: center;
    width: 100%;
    min-height: 42px;
    margin: 0;
    padding: 0 14px;
    gap: 12px;
    color: var(--text-main);
    border-radius: var(--radius-md);
    font-size: 15px;
    font-weight: 500;
    transition: all 160ms ease;
  }

  nav button span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .ui-compact nav button {
    min-height: 38px;
    padding: 0 12px;
    font-size: 14px;
  }

  nav button:hover {
    color: var(--primary);
    background: var(--primary-light);
  }

  nav button.active {
    color: #ffffff;
    background: var(--primary);
    box-shadow: 0 12px 28px rgba(47, 128, 237, 0.24);
  }

  .language-control {
    display: grid;
    gap: 10px;
    margin: 6px 0 0;
  }

  .ui-compact .language-control {
    gap: 8px;
    margin-top: 4px;
  }

  .language-control span,
  label span,
  .stat-card p,
  .history-card p {
    color: var(--text-secondary);
    font-size: 14px;
    font-weight: 700;
    text-transform: none;
  }

  .language-control select {
    width: 100%;
    min-height: 38px;
    padding: 0 12px;
    color: var(--text-main);
    background: #ffffff;
    border: 1px solid var(--border);
    border-radius: 10px;
    font-size: 15px;
  }

  .ui-compact .language-control select {
    min-height: 34px;
    font-size: 14px;
  }

  .bridge-card {
    display: grid;
    gap: 7px;
    margin: auto 0 0;
    padding: 12px;
    min-width: 0;
    overflow: hidden;
    background: #ffffff;
    border: 1px solid var(--border);
    border-radius: 14px;
    box-shadow: var(--shadow-card);
  }

  .ui-compact .bridge-card {
    gap: 6px;
    padding: 10px;
  }

  .bridge-card.listening {
    border-color: rgba(47, 128, 237, 0.28);
  }

  .bridge-card.error {
    border-color: rgba(239, 68, 68, 0.28);
  }

  .bridge-top {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 0;
    min-width: 0;
    color: var(--text-main);
    font-size: 15px;
    font-weight: 800;
  }

  .bridge-top span:last-child {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .bridge-card p {
    margin: 0;
    min-width: 0;
    color: var(--text-secondary);
    font-size: 13px;
    line-height: 1.35;
    display: -webkit-box;
    overflow-wrap: anywhere;
    line-clamp: 2;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .pulse {
    width: 10px;
    height: 10px;
    flex: 0 0 auto;
    background: var(--success);
    border-radius: 999px;
  }

  .pulse.recording {
    background: var(--primary);
    box-shadow: 0 0 0 8px rgba(47, 128, 237, 0.14);
  }

  .pulse.error {
    background: var(--danger);
    box-shadow: 0 0 0 8px rgba(239, 68, 68, 0.12);
  }

  .mic-line {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    min-height: 28px;
    padding-top: 8px;
    color: var(--text-secondary);
    border-top: 1px solid var(--border);
    font-size: 12px;
  }

  .shortcut-line {
    min-width: 0;
    color: var(--text-secondary);
    font-size: 12px;
    line-height: 1.35;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .mic-line span {
    margin-right: auto;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .mic-line i {
    display: block;
    width: 4px;
    background: var(--success);
    border-radius: 999px;
    transform-origin: bottom center;
    transition: height 90ms ease, opacity 90ms ease, background-color 160ms ease;
  }

  .mic-line.active i {
    background: var(--primary);
  }

  .content {
    min-width: 0;
    min-height: 0;
    padding: 16px 20px;
    overflow: auto;
    overflow-x: hidden;
    background: var(--bg-page);
  }

  .content::-webkit-scrollbar {
    width: 10px;
  }

  .content::-webkit-scrollbar-thumb {
    background: #cbd8e7;
    border: 3px solid var(--bg-page);
    border-radius: 999px;
  }

  .content > section,
  .content > header {
    width: min(100%, 1120px);
    margin-left: auto;
    margin-right: auto;
  }

  .ui-compact .content {
    padding: 14px 16px;
  }

  .content.overview-content {
    display: grid;
    grid-auto-rows: max-content;
    gap: 14px;
    align-content: start;
    overflow: auto;
  }

  .ui-compact .content.overview-content {
    gap: 12px;
  }

  .content.overview-content.setup-required {
    overflow: auto;
  }

  .overview-content .voice-card,
  .overview-content .launch-card,
  .overview-content .performance-card {
    min-height: max-content;
  }

  .topbar {
    display: none;
  }

  .voice-card,
  .launch-card,
  .performance-card,
  .form-panel,
  .history-card {
    min-width: 0;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 16px;
    box-shadow: var(--shadow-card);
  }

  .voice-card,
  .launch-card,
  .performance-card {
    padding: 14px;
    overflow: hidden;
  }

  .ui-compact .voice-card,
  .ui-compact .launch-card,
  .ui-compact .performance-card {
    padding: 12px;
  }

  .launch-card,
  .performance-card {
    margin-top: 10px;
  }

  .overview-content .launch-card,
  .overview-content .performance-card {
    margin-top: 0;
  }

  .section-title-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 8px;
    min-width: 0;
  }

  .section-title-row > div {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }

  .section-title-row h3 {
    margin: 0;
    min-width: 0;
    color: var(--text-main);
    font-size: 17px;
    font-weight: 800;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .ui-compact .section-title-row h3 {
    font-size: 16px;
  }

  .voice-hero {
    position: relative;
    display: grid;
    grid-template-columns: 116px minmax(0, 1fr);
    align-items: center;
    gap: 24px;
    min-height: 168px;
    height: auto;
    padding: 22px 28px;
    overflow: hidden;
    color: #ffffff;
    background: linear-gradient(135deg, var(--gradient-start) 0%, var(--gradient-end) 100%);
    border-radius: 16px;
    box-shadow: 0 16px 34px rgba(47, 128, 237, 0.2);
  }

  .ui-compact .voice-hero {
    grid-template-columns: 92px minmax(0, 1fr);
    gap: 18px;
    min-height: 148px;
    height: auto;
    padding: 18px 22px;
  }

  .voice-hero::after {
    position: absolute;
    inset: 0;
    content: "";
    background: linear-gradient(
      118deg,
      transparent 0%,
      transparent 62%,
      rgba(255, 255, 255, 0.12) 62%,
      rgba(255, 255, 255, 0.06) 74%,
      transparent 74%
    );
    pointer-events: none;
  }

  .mic-orb {
    position: relative;
    z-index: 1;
    display: grid;
    width: 108px;
    height: 108px;
    place-items: center;
    color: var(--primary);
    background: rgba(255, 255, 255, 0.18);
    border-radius: 999px;
    transition: all 160ms ease;
  }

  .ui-compact .mic-orb {
    width: 88px;
    height: 88px;
  }

  .mic-orb:hover {
    transform: translateY(-2px);
  }

  .mic-ring {
    display: grid;
    width: 86px;
    height: 86px;
    place-items: center;
    background: #ffffff;
    border-radius: 999px;
    box-shadow: 0 8px 22px rgba(15, 23, 42, 0.12);
  }

  .ui-compact .mic-ring {
    width: 70px;
    height: 70px;
  }

  .mic-orb.listening {
    animation: mic-pulse 1.4s ease-in-out infinite;
  }

  .mic-orb.listening .mic-ring {
    color: var(--danger);
  }

  .voice-copy {
    position: relative;
    z-index: 1;
    min-width: 0;
    max-width: min(100%, 640px);
    padding-right: 150px;
  }

  .hero-status {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 7px;
    font-size: 24px;
    font-weight: 800;
  }

  .hero-status strong {
    min-width: 0;
    overflow-wrap: anywhere;
    line-height: 1.18;
  }

  .ui-compact .hero-status {
    gap: 10px;
    margin-bottom: 5px;
    font-size: 20px;
  }

  .hero-dot {
    width: 16px;
    height: 16px;
    border: 4px solid rgba(255, 255, 255, 0.55);
    background: transparent;
    border-radius: 999px;
  }

  .ui-compact .hero-dot {
    width: 12px;
    height: 12px;
    border-width: 3px;
  }

  .hero-dot.listening {
    background: #ffffff;
    border-color: rgba(255, 255, 255, 0.2);
    animation: status-blink 1s ease-in-out infinite;
  }

  .hero-dot.error {
    background: var(--danger);
    border-color: rgba(255, 255, 255, 0.35);
  }

  .voice-copy h4 {
    margin: 0 0 6px;
    color: #ffffff;
    font-size: 19px;
    font-weight: 700;
    line-height: 1.25;
    overflow-wrap: anywhere;
  }

  .ui-compact .voice-copy h4 {
    margin-bottom: 4px;
    font-size: 16px;
  }

  .voice-copy p {
    margin: 0;
    color: rgba(255, 255, 255, 0.86);
    font-size: 14px;
    line-height: 1.35;
    overflow-wrap: anywhere;
  }

  .ui-compact .voice-copy p {
    font-size: 13px;
  }

  .shortcut-help {
    position: absolute;
    top: 12px;
    right: 12px;
    z-index: 2;
    display: inline-flex;
    align-items: center;
    gap: 10px;
    height: 30px;
    padding: 0 10px;
    color: #ffffff;
    background: rgba(255, 255, 255, 0.15);
    border: 1px solid rgba(255, 255, 255, 0.18);
    border-radius: 12px;
    font-size: 13px;
    font-weight: 600;
    max-width: 150px;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .shortcut-help :global(svg) {
    flex: 0 0 auto;
  }

  .ui-compact .shortcut-help {
    top: 10px;
    right: 10px;
    height: 28px;
    max-width: 142px;
    font-size: 12px;
  }

  .ui-compact .voice-copy {
    padding-right: 116px;
  }

  .hero-features {
    display: flex;
    flex-wrap: wrap;
    gap: 10px 14px;
    margin-top: 10px;
    color: rgba(255, 255, 255, 0.94);
    font-size: 12px;
    font-weight: 600;
  }

  .ui-compact .hero-features {
    gap: 10px;
    margin-top: 8px;
    font-size: 11px;
  }

  .hero-features span {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    max-width: 100%;
    line-height: 1.25;
    white-space: normal;
  }

  .link-action {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--primary);
    background: transparent;
    font-size: 15px;
    font-weight: 600;
  }

  .trigger-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 12px;
  }

  .ui-compact .trigger-grid {
    gap: 8px;
  }

  .trigger-item {
    position: relative;
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    min-height: 68px;
    min-width: 0;
    padding: 14px 18px;
    color: inherit;
    font: inherit;
    text-align: left;
    background: #ffffff;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: all 160ms ease;
  }

  .trigger-item > div {
    min-width: 0;
  }

  .trigger-item:hover {
    border-color: rgba(47, 128, 237, 0.55);
    background: #f8fbff;
  }

  .trigger-item.disabled {
    cursor: wait;
    opacity: 0.72;
  }

  .trigger-item:focus-within {
    outline: 2px solid rgba(47, 128, 237, 0.28);
    outline-offset: 2px;
  }

  .trigger-input {
    position: absolute;
    width: 1px;
    height: 1px;
    opacity: 0;
    pointer-events: none;
  }

  .ui-compact .trigger-item {
    min-height: 64px;
    padding: 12px 14px;
  }

  .trigger-item.active {
    background: var(--primary-light);
    border-color: var(--primary);
  }

  .trigger-check {
    display: grid;
    width: 32px;
    height: 32px;
    flex: 0 0 auto;
    place-items: center;
    color: #ffffff;
    background: var(--primary);
    border-radius: 8px;
  }

  .ui-compact .trigger-check {
    width: 28px;
    height: 28px;
  }

  .trigger-item:not(.active) .trigger-check {
    color: transparent;
    background: #ffffff;
    border: 1px solid #aebbd0;
  }

  .trigger-item strong {
    color: var(--text-main);
    font-size: 16px;
    font-weight: 700;
    overflow-wrap: anywhere;
  }

  .ui-compact .trigger-item strong {
    font-size: 15px;
  }

  .trigger-item p {
    margin: 5px 0 0;
    color: var(--text-secondary);
    font-size: 12px;
  }

  .ui-compact .trigger-item p {
    margin-top: 3px;
    font-size: 11px;
  }

  .stats-row {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 10px;
    margin: 0;
  }

  .ui-compact .stats-row {
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 8px;
  }

  .overview-content .stats-row {
    height: auto;
    min-height: 0;
  }

  .stat-card {
    position: relative;
    display: grid;
    grid-template-rows: 22px auto auto auto;
    align-content: start;
    align-items: start;
    gap: 2px;
    min-height: 112px;
    min-width: 0;
    padding: 12px 14px 18px;
    overflow: hidden;
    background: #ffffff;
    border: 1px solid var(--border);
    border-radius: 14px;
    box-shadow: 0 4px 12px rgba(15, 23, 42, 0.04);
  }

  .ui-compact .stat-card {
    min-height: 104px;
    padding: 10px 11px 16px;
  }

  .stat-card::after {
    position: absolute;
    left: 14px;
    bottom: 12px;
    width: 42px;
    height: 3px;
    content: "";
    background: currentColor;
    border-radius: 999px;
    opacity: 0.42;
  }

  .ui-compact .stat-card::after {
    left: 12px;
    bottom: 10px;
    width: 36px;
  }

  .overview-content .stat-card {
    height: auto;
  }

  .stat-card.blue { --stat-accent: var(--primary); color: var(--stat-accent); }
  .stat-card.purple { --stat-accent: var(--gradient-end); color: var(--stat-accent); }
  .stat-card.green { --stat-accent: var(--success); color: var(--stat-accent); }
  .stat-card.orange { --stat-accent: #f97316; color: var(--stat-accent); }

  .stat-icon {
    display: grid;
    width: 20px;
    height: 20px;
    place-items: center;
    color: #ffffff;
    background: var(--stat-accent);
    border-radius: 7px;
  }

  .ui-compact .stat-icon {
    width: 18px;
    height: 18px;
  }

  .stat-card p {
    margin: 2px 0 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .ui-compact .stat-card p {
    margin-top: 1px;
  }

  .stat-card strong {
    display: block;
    margin: 0;
    color: var(--text-main);
    font-size: 16px;
    font-weight: 800;
    line-height: 1.18;
    overflow-wrap: anywhere;
  }

  .ui-compact .stat-card strong {
    font-size: 15px;
  }

  .stat-card small {
    display: block;
    margin-top: 1px;
    color: var(--text-secondary);
    font-size: 11px;
    line-height: 1.35;
    overflow-wrap: anywhere;
  }

  .ui-compact .stat-card small {
    margin-top: 1px;
    font-size: 11px;
  }

  .usage-tip {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    margin: 8px 0 0;
    color: var(--text-secondary);
    font-size: 12px;
    line-height: 1.35;
    overflow-wrap: anywhere;
  }

  .usage-tip :global(svg) {
    flex: 0 0 auto;
  }

  .ui-compact .usage-tip {
    margin-top: 6px;
    font-size: 11px;
  }

  .settings-stack {
    display: grid;
    gap: 22px;
    max-width: 1040px;
  }

  .settings-group {
    display: grid;
    gap: 12px;
  }

  .settings-group-heading {
    display: grid;
    gap: 4px;
    padding: 0 2px;
  }

  .settings-group-heading h3 {
    margin: 0;
    color: var(--text-main);
    font-size: 20px;
    font-weight: 800;
  }

  .settings-group-heading p {
    margin: 0;
    color: var(--text-secondary);
    font-size: 13px;
  }

  .form-panel {
    display: grid;
    gap: 14px;
    padding: 18px;
    border-radius: 18px;
    box-shadow: none;
  }

  .form-panel label {
    display: grid;
    gap: 8px;
    color: var(--text-secondary);
    font-size: 14px;
  }

  .form-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 14px;
  }

  .toggle-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
  }

  .check {
    display: flex !important;
    grid-template-columns: none;
    align-items: center;
    gap: 10px;
    min-height: 38px;
    min-width: 0;
    line-height: 1.35;
    overflow-wrap: anywhere;
  }

  .check input {
    width: 18px;
    min-height: 18px;
    accent-color: var(--primary);
  }

  .section-heading {
    display: grid;
    gap: 4px;
  }

  .section-heading h3 {
    margin: 0;
    margin-bottom: 6px;
    color: var(--text-main);
    font-size: 16px;
    font-weight: 800;
  }

  .section-heading p {
    margin: 0;
    color: var(--text-secondary);
    font-size: 13px;
  }

  .section-heading.with-actions {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
  }

  .section-heading.with-actions > div {
    min-width: 0;
  }

  .section-heading.with-actions .link-button {
    margin-top: 8px;
  }

  .test-button {
    display: inline-flex;
    flex: 0 0 auto;
    align-items: center;
    gap: 7px;
    justify-content: center;
    min-width: 78px;
    min-height: 34px;
    padding: 0 12px;
    color: var(--primary);
    background: var(--primary-light);
    border: 1px solid rgba(47, 128, 237, 0.18);
    border-radius: 10px;
    font-size: 13px;
    font-weight: 800;
    white-space: nowrap;
  }

  .test-button:disabled {
    cursor: wait;
    opacity: 0.68;
  }

  input,
  textarea,
  select {
    width: 100%;
    min-height: 38px;
    padding: 0 12px;
    color: var(--text-main);
    background: #ffffff;
    border: 1px solid var(--border);
    border-radius: 10px;
  }

  select,
  input {
    min-width: 0;
    text-overflow: ellipsis;
  }

  textarea {
    min-height: 84px;
    padding: 10px 12px;
    resize: vertical;
  }

  .field-hint {
    margin: 8px 0 0;
    color: var(--text-muted);
    font-size: 12px;
    line-height: 1.45;
  }

  input:focus,
  textarea:focus,
  select:focus,
  button:focus-visible {
    border-color: var(--primary);
    box-shadow: 0 0 0 3px rgba(47, 128, 237, 0.14);
  }

  .form-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    grid-column: 1 / -1;
    justify-content: flex-end;
  }

  .form-actions button {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    min-height: 38px;
    padding: 0 16px;
    color: var(--text-main);
    border: 1px solid var(--border);
    border-radius: 10px;
  }

  .form-actions .primary {
    color: #ffffff;
    background: var(--primary);
  }

  .update-card {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    padding: 14px;
    background: #f8fbff;
    border: 1px solid var(--border);
    border-radius: 12px;
  }

  .update-card > div:first-child {
    min-width: min(100%, 320px);
    flex: 1 1 320px;
  }

  .update-card.available {
    background: #fff7ed;
    border-color: #fed7aa;
  }

  .update-card strong {
    display: block;
    margin-bottom: 4px;
    color: var(--text-main);
    font-size: 15px;
    font-weight: 800;
  }

  .update-card p {
    margin: 0;
    color: var(--text-secondary);
    font-size: 13px;
    line-height: 1.4;
    overflow-wrap: anywhere;
  }

  .update-card small {
    display: block;
    margin-top: 6px;
    color: var(--text-muted);
    font-size: 12px;
  }

  .update-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    justify-content: flex-end;
  }

  .update-actions button {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    min-height: 36px;
    padding: 0 12px;
    color: var(--text-main);
    background: #ffffff;
    border: 1px solid var(--border);
    border-radius: 10px;
    font-weight: 700;
  }

  .update-actions .primary {
    color: #ffffff;
    background: var(--primary);
    border-color: var(--primary);
  }

  .update-actions button:disabled {
    cursor: wait;
    opacity: 0.66;
  }

  .action-notice {
    position: fixed;
    right: 22px;
    bottom: 20px;
    z-index: 20;
    display: inline-flex;
    align-items: center;
    max-width: min(340px, calc(100vw - 44px));
    min-height: 40px;
    gap: 8px;
    padding: 0 14px;
    color: #0f5132;
    background: rgba(240, 253, 244, 0.98);
    border: 1px solid rgba(34, 197, 94, 0.26);
    border-radius: 12px;
    box-shadow: 0 14px 34px rgba(15, 23, 42, 0.12);
    font-size: 14px;
    font-weight: 700;
  }

  .action-notice span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .action-notice.warning {
    color: #854d0e;
    background: rgba(255, 251, 235, 0.98);
    border-color: rgba(245, 158, 11, 0.32);
  }

  .action-notice.error {
    color: #991b1b;
    background: rgba(254, 242, 242, 0.98);
    border-color: rgba(239, 68, 68, 0.3);
  }

  @media (prefers-reduced-motion: no-preference) {
    .action-notice {
      animation: action-notice-enter 180ms ease-out;
    }

    @keyframes action-notice-enter {
      from {
        opacity: 0;
        transform: translateY(8px);
      }
      to {
        opacity: 1;
        transform: translateY(0);
      }
    }
  }

  .setup-alert {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 16px;
    padding: 14px 16px;
    background: #fff7ed;
    border: 1px solid #fed7aa;
    border-radius: 14px;
  }

  .setup-alert strong {
    color: var(--text-main);
  }

  .setup-alert p {
    margin: 4px 0 0;
    color: var(--text-secondary);
    font-size: 14px;
  }

  .setup-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    flex: 0 0 auto;
  }

  .setup-actions button,
  .link-button {
    min-height: 36px;
    padding: 0 12px;
    color: #ffffff;
    background: var(--primary);
    border-radius: 10px;
    font-weight: 600;
  }

  .setup-actions .secondary,
  .link-button {
    color: var(--primary);
    background: var(--primary-light);
  }

  .history-page {
    display: grid;
    gap: 14px;
    max-width: 1120px;
  }

  .history-summary {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(170px, 1fr));
    gap: 12px;
  }

  .history-card {
    min-height: 104px;
    padding: 16px;
  }

  .history-card strong {
    display: block;
    margin-top: 10px;
    color: var(--text-main);
    font-size: 20px;
    font-weight: 800;
    line-height: 1.2;
    overflow-wrap: anywhere;
  }

  .history-card span {
    display: block;
    margin-top: 8px;
    color: var(--text-secondary);
    font-size: 13px;
  }

  .history-card.blue { border-top: 4px solid var(--primary); }
  .history-card.purple { border-top: 4px solid var(--gradient-end); }
  .history-card.green { border-top: 4px solid var(--success); }
  .history-card.orange { border-top: 4px solid #f97316; }

  .daily-panel {
    min-width: 0;
    padding: 18px;
  }

  .day-list {
    display: grid;
    gap: 0;
    min-width: 0;
    overflow: hidden;
  }

  .day-list-head,
  .day-list article {
    display: grid;
    grid-template-columns: minmax(120px, 1.05fr) repeat(4, minmax(92px, 1fr));
    align-items: center;
    gap: 12px;
    min-height: 48px;
    padding: 10px 0;
    border-bottom: 1px solid var(--border);
  }

  .day-list-head {
    min-height: 34px;
    padding-top: 0;
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 700;
  }

  .day-list article:last-child {
    border-bottom: 0;
  }

  .day-list span {
    color: var(--text-secondary);
    font-size: 14px;
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .day-list strong {
    color: var(--text-main);
    font-size: 15px;
    font-weight: 800;
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .day-list-head span:nth-child(n + 2),
  .day-list article span:nth-child(n + 3),
  .day-list article strong {
    text-align: right;
  }

  .day-list article span:first-child {
    text-align: left;
  }

  @keyframes mic-pulse {
    0%, 100% { box-shadow: 0 0 0 0 rgba(255, 255, 255, 0.18); }
    50% { box-shadow: 0 0 0 16px rgba(255, 255, 255, 0.08); }
  }

  @keyframes status-blink {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.46; }
  }

  @media (max-width: 920px) {
    .shell { grid-template-columns: 210px minmax(0, 1fr); }
    .content { padding: 16px; }
    .content.overview-content { overflow: auto; }
    .trigger-grid,
    .stats-row,
    .ui-compact .trigger-grid,
    .ui-compact .stats-row {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
    .voice-hero {
      grid-template-columns: 86px minmax(0, 1fr);
      padding: 18px 22px;
    }
    .mic-orb {
      width: 80px;
      height: 80px;
    }
    .mic-ring {
      width: 64px;
      height: 64px;
    }
    .voice-copy {
      padding-right: 106px;
    }
    .hero-status {
      font-size: 21px;
    }
    .voice-copy h4 {
      font-size: 16px;
    }
    .day-list-head,
    .day-list article {
      grid-template-columns: minmax(104px, 1fr) repeat(4, minmax(78px, 0.82fr));
      gap: 8px;
    }
  }
</style>
