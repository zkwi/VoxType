<script lang="ts">
  import { onMount } from "svelte";
  import { browser } from "$app/environment";
  import SetupStatusCard, {
    type SetupStatusItem,
    type SetupStatusWarning,
  } from "$lib/components/overview/SetupStatusCard.svelte";
  import SettingsToolbar from "$lib/components/settings/SettingsToolbar.svelte";
  import { copy, userErrorDetails, type CopyKey, type Language, type UserErrorDetail } from "$lib/i18n";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import {
    AlertCircle,
    BarChart3,
    CalendarDays,
    Check,
    ChevronRight,
    ClipboardCopy,
    Clock3,
    Download,
    FileText,
    Gauge,
    Globe2,
    Info,
    Keyboard,
    Maximize2,
    MessageSquareText,
    Mic,
    Minus,
    PenLine,
    Settings,
    ShieldCheck,
    Sparkles,
    Trash2,
    Zap,
    X as XIcon,
  } from "lucide-svelte";

  type Section = "Home" | "Hotwords" | "ApiConfig" | "Options" | "History";
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
  type DiagnosticReport = { text: string };
  type ConfigValidationError = {
    field: string;
    message: string;
  };
  type ConfigSaveError = {
    message: string;
    errors?: ConfigValidationError[];
  };
  type AutoHotwordStatus = {
    enabled: boolean;
    entry_count: number;
    total_chars: number;
    max_history_chars: number;
  };
  type HotwordCandidate = {
    word: string;
    category: string;
    reason: string;
    confidence: number;
    source_count: number;
  };
  type SelectableHotwordCandidate = HotwordCandidate & { selected: boolean };
  type HotwordGenerationResult = {
    candidates: HotwordCandidate[];
    used_chars: number;
    warning: string | null;
  };
  type PersistConfigOptions = {
    enforceAuth?: boolean;
    focusErrors?: boolean;
  };
  type CloseToTrayRequest = {
    first_time: boolean;
    behavior: string;
  };
  type SetupStatus = {
    ready: boolean;
    missing_auth: boolean;
    has_audio_device: boolean;
    hotkey: string;
    paste_method: string;
    privacy_recent_context_enabled: boolean;
    warnings: SetupStatusWarning[];
  };
  type HotkeyCaptureState = "idle" | "recording";
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
    error_code: string | null;
    warning: string | null;
  };

  type OverlayText = { text: string };
  type OverlayConfig = { ui: AppConfig["ui"] };
  type AudioLevel = { level: number };
  type AudioDeviceInfo = { index: number; name: string; is_default: boolean };
  type OverlayMode = "single" | "double";
  type AsrConnectionStatus =
    | "missing_auth"
    | "configured_not_tested"
    | "testing"
    | "tested_ok"
    | "tested_failed";

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
      clipboard_restore_delay_ms: number;
      clipboard_snapshot_max_bytes: number;
      clipboard_open_retry_count: number;
      clipboard_open_retry_interval_ms: number;
    };
    startup: { launch_on_startup: boolean };
    update: { auto_check_on_startup: boolean; github_repo: string };
    auto_hotwords: {
      enabled: boolean;
      max_history_chars: number;
      max_candidates: number;
      ignored_hotwords: string[];
    };
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
      background_color: string;
      text_color: string;
    };
    tray: {
      show_startup_message: boolean;
      startup_message_timeout_ms: number;
      close_behavior: string;
      close_to_tray_notice_shown: boolean;
    };
    debug: { print_transcript_to_console: boolean };
  };
  type TriggerKey = keyof AppConfig["triggers"];
  type SoftConfigNoticeKey = TriggerKey | "mute_system_volume_while_recording" | "enable_recent_context";

  const defaultLlmSystemPrompt = `你是语音输入助手。

场景：用户通过语音输入文字，语音识别（ASR）将语音转为文本后交给你处理。
你的输出将直接粘贴到用户的光标位置。永远只输出处理后的文本，不要与用户对话。如果无需处理，原样输出。

任务：
1. 修正明显的语音识别错误
2. 在不改变原意的前提下，对必要文本进行轻度润色、轻度改写或重写，使表达更清晰自然
3. 删除无意义的口头语、语气词和明显重复
4. 当文本较长、层次较多或明显属于口述长句时，可以主动分段、分行、分点整理，让结构更清晰
5. 不要扩写，不要新增事实，不要改变用户立场和语气，不要编造任何内容
6. 保留专有名词、数字、百分比、金融和编程术语
7. 如果原文本身已经简洁清楚，就尽量少改
8. 自动去掉结尾的句号
9. 最终只返回处理后的文本，不要输出任何解释、标题或多余内容`;
  const defaultLlmUserPromptTemplate = `以下是用户通过语音转写得到的文本，请按要求直接输出处理后的最终文本：

{text}

处理要求：
- 如果文本较短且表达清楚，尽量少改
- 如果文本较长、信息点较多、层次较乱，优先进行结构化整理，可按语义分段、分行、分点
- 如果存在明显识别错误、口头语、重复、语序混乱，可做必要的轻度改写，使其更清晰自然
- 不要输出解释，不要输出标题，不要输出任何额外说明`;

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
      hotwords: [],
      prompt_context: [],
      recent_context: [],
    },
    triggers: { hotkey_enabled: true, middle_mouse_enabled: false, right_alt_enabled: false },
    typing: {
      paste_delay_ms: 120,
      paste_method: "ctrl_v",
      restore_clipboard_after_paste: true,
      clipboard_restore_delay_ms: 800,
      clipboard_snapshot_max_bytes: 8 * 1024 * 1024,
      clipboard_open_retry_count: 5,
      clipboard_open_retry_interval_ms: 50,
    },
    startup: { launch_on_startup: false },
    update: { auto_check_on_startup: true, github_repo: "zkwi/VoxType" },
    auto_hotwords: {
      enabled: false,
      max_history_chars: 10000,
      max_candidates: 30,
      ignored_hotwords: [],
    },
    llm_post_edit: {
      enabled: false,
      min_chars: 40,
      base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1",
      api_key: "",
      model: "qwen3.5-plus",
      timeout_seconds: 30,
      enable_thinking: false,
      system_prompt: defaultLlmSystemPrompt,
      user_prompt_template: defaultLlmUserPromptTemplate,
    },
    ui: {
      width: 350,
      height: 64,
      margin_bottom: 52,
      opacity: 0.9,
      scroll_interval_ms: 1200,
      background_color: "#176ee6",
      text_color: "#ffffff",
    },
    tray: {
      show_startup_message: true,
      startup_message_timeout_ms: 6000,
      close_behavior: "close_to_tray",
      close_to_tray_notice_shown: false,
    },
    debug: { print_transcript_to_console: false },
  };

  const fallbackSnapshot: AppSnapshot = {
    hotkey: "ctrl+q",
    current_version: "0.1.16",
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
  const overlayMeterBars = [0, 1, 2, 3];
  const overlayColorPresets: { label: CopyKey; background: string; text: string }[] = [
    { label: "overlayPresetBlue", background: "#176ee6", text: "#ffffff" },
    { label: "overlayPresetDark", background: "#111827", text: "#f8fafc" },
    { label: "overlayPresetLight", background: "#f8fafc", text: "#111827" },
    { label: "overlayPresetAmber", background: "#92400e", text: "#fff7ed" },
  ];
  const overlayOpacityPresets = [0.6, 0.75, 0.9, 1] as const;
  const navItems: { id: Section; icon: typeof Gauge }[] = [
    { id: "Home", icon: Gauge },
    { id: "Hotwords", icon: Sparkles },
    { id: "ApiConfig", icon: ShieldCheck },
    { id: "Options", icon: Settings },
    { id: "History", icon: BarChart3 },
  ];

  const navLabelKeys: Record<Section, CopyKey> = {
    Home: "navHome",
    Hotwords: "navHotwords",
    ApiConfig: "navApiConfig",
    Options: "navOptions",
    History: "navHistory",
  };
  const setupStatusCacheKey = "voxtype-setup-status-v1";
  const autoSaveDelayMs = 700;

  function readCachedSetupStatus(): SetupStatus | null {
    if (!browser) return null;
    const params = new URLSearchParams(window.location.search);
    if (params.has("overlay") || params.has("toast")) return null;
    try {
      const raw = localStorage.getItem(setupStatusCacheKey);
      if (!raw) return null;
      const parsed = JSON.parse(raw) as Partial<SetupStatus>;
      if (
        typeof parsed.ready !== "boolean" ||
        typeof parsed.missing_auth !== "boolean" ||
        typeof parsed.has_audio_device !== "boolean"
      ) {
        return null;
      }
      return {
        ready: parsed.ready,
        missing_auth: parsed.missing_auth,
        has_audio_device: parsed.has_audio_device,
        hotkey: typeof parsed.hotkey === "string" ? parsed.hotkey : fallbackSnapshot.hotkey,
        paste_method: typeof parsed.paste_method === "string" ? parsed.paste_method : fallbackConfig.typing.paste_method,
        privacy_recent_context_enabled: Boolean(parsed.privacy_recent_context_enabled),
        warnings: Array.isArray(parsed.warnings) ? (parsed.warnings as SetupStatusWarning[]) : [],
      };
    } catch {
      return null;
    }
  }

  let measureCanvas: HTMLCanvasElement | undefined;
  let snapshot = $state<AppSnapshot>(fallbackSnapshot);
  let config = $state<AppConfig>(clonePlain(fallbackConfig));
  let savedConfigFingerprint = $state(configFingerprint(fallbackConfig));
  let settingsDirty = $derived(configFingerprint(config) !== savedConfigFingerprint);
  let stats = $state<StatsSnapshot>(emptyStats);
  let recording = $state(false);
  let sessionPhase = $state<SessionPhase>("idle");
  let sessionErrorCode = $state<string | null>(null);
  let language = $state<Language>("zh-CN");
  let statusMessage = $state(copy["zh-CN"].bridgeLoading);
  let selectedSection = $state<Section>("Home");
  let saving = $state(false);
  let configExists = $state(true);
  let configLoaded = $state(false);
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
  let actionNoticeKind = $state<"success" | "info" | "warning" | "error">("success");
  let actionNoticeTimer: number | undefined;
  let updateStatus = $state<UpdateStatus | null>(null);
  let setupStatus = $state<SetupStatus | null>(readCachedSetupStatus());
  let checkingUpdate = $state(false);
  let installingUpdate = $state(false);
  let openingLog = $state(false);
  let testingAsr = $state(false);
  let asrConnectionStatus = $state<AsrConnectionStatus>("missing_auth");
  let asrTestedConfigFingerprint = $state("");
  let testingLlm = $state(false);
  let copyingDiagnosticReport = $state(false);
  let clearingRecentContext = $state(false);
  let autoHotwordStatus = $state<AutoHotwordStatus | null>(null);
  let generatingAutoHotwords = $state(false);
  let clearingAutoHotwordHistory = $state(false);
  let autoHotwordCandidates = $state<SelectableHotwordCandidate[]>([]);
  let autoHotwordError = $state("");
  let validationErrors = $state<Record<string, string>>({});
  let closePromptVisible = $state(false);
  let closePromptFirstTime = $state(false);
  let closePromptBehavior = $state("close_to_tray");
  let succeededIdleTimer: number | undefined;
  let autoSaveTimer: number | undefined;
  let setupStatusLoading = $state(false);
  let hotkeyCaptureState = $state<HotkeyCaptureState>("idle");
  let hotkeyValidationMessage = $state("");
  let showAdvancedSettings = $state(false);
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
      }, 250);
    }
    let unlisteners: Array<Promise<() => void>> = [];
    if (hasTauriApi()) {
      const unlistenSession = listen<SessionState>("session-state-changed", (event) => {
        applySessionState(event.payload);
      });
      const unlistenAsr = listen<AsrFinalText>("asr-final-text", (event) => {
        if (event.payload.error) {
          sessionErrorCode = event.payload.error_code;
          statusMessage = userErrorMessage(event.payload.error_code, event.payload.error);
          showActionNotice(statusMessage, "error");
          if (shouldOpenSettingsForError(event.payload.error, event.payload.error_code)) {
            scrollToSettingsPanel(settingsPanelForError(event.payload.error, event.payload.error_code));
          }
          return;
        }
        if (event.payload.warning) showActionNotice(event.payload.warning, "warning");
        statusMessage = event.payload.warning ? event.payload.warning : t("sessionSucceeded");
        if (sessionPhase === "succeeded") scheduleSucceededIdleHint();
      });
      const unlistenOverlay = listen<OverlayText>("overlay-text", (event) => {
        applyOverlayText(event.payload.text || defaultOverlayText);
      });
      const unlistenOverlayConfig = listen<OverlayConfig>("overlay-config", (event) => {
        applyOverlayConfig(event.payload.ui);
      });
      const unlistenStats = listen<StatsSnapshot>("usage-stats-updated", (event) => {
        if (!isOverlay && !isToast) {
          stats = event.payload;
          void refreshAutoHotwordStatus();
        }
      });
      const unlistenAudioLevel = listen<AudioLevel>("audio-level", (event) => {
        audioLevel = clampAudioLevel(event.payload.level);
      });
      const unlistenClosePrompt = listen<CloseToTrayRequest>("close-to-tray-requested", (event) => {
        closePromptFirstTime = event.payload.first_time;
        closePromptBehavior = event.payload.behavior;
        closePromptVisible = true;
      });
      unlisteners = [
        unlistenSession,
        unlistenAsr,
        unlistenOverlay,
        unlistenOverlayConfig,
        unlistenStats,
        unlistenAudioLevel,
        unlistenClosePrompt,
      ];
      logFrontendEvent(`listeners registered mode=${frontendMode()}`);
    }
    return () => {
      if (overlayPoll !== undefined) window.clearInterval(overlayPoll);
      if (actionNoticeTimer !== undefined) window.clearTimeout(actionNoticeTimer);
      if (succeededIdleTimer !== undefined) window.clearTimeout(succeededIdleTimer);
      clearAutoSaveTimer();
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

  $effect(() => {
    const fingerprint = configFingerprint(config);
    const shouldSave =
      fingerprint !== savedConfigFingerprint &&
      configLoaded &&
      hotkeyCaptureState === "idle" &&
      !isOverlay &&
      !isToast &&
      hasTauriApi();

    if (shouldSave) {
      scheduleAutoSaveConfig();
    } else if (fingerprint === savedConfigFingerprint) {
      clearAutoSaveTimer();
    }
  });

  function clonePlain<T>(value: T): T {
    return JSON.parse(JSON.stringify(value)) as T;
  }
  function configFingerprint(value: AppConfig) {
    return JSON.stringify(value);
  }
  function clearAutoSaveTimer() {
    if (autoSaveTimer !== undefined && browser) window.clearTimeout(autoSaveTimer);
    autoSaveTimer = undefined;
  }
  function canAutoSaveConfig() {
    return configLoaded && !isOverlay && !isToast && hasTauriApi() && hotkeyCaptureState === "idle";
  }
  function scheduleAutoSaveConfig() {
    if (!canAutoSaveConfig()) return;
    clearAutoSaveTimer();
    autoSaveTimer = window.setTimeout(() => {
      autoSaveTimer = undefined;
      void autoSaveConfig();
    }, autoSaveDelayMs);
  }
  async function autoSaveConfig() {
    if (!canAutoSaveConfig() || !settingsDirty) return;
    if (saving) {
      scheduleAutoSaveConfig();
      return;
    }
    await persistConfig({ enforceAuth: false, focusErrors: false });
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
    if (requireAsrAuthGate()) return;
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

  function applyOverlayConfig(ui: AppConfig["ui"]) {
    if (!isOverlay) return;
    config.ui = { ...config.ui, ...ui };
    stopOverlayScroll();
    applyOverlayText(overlayText, true);
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

  function rememberSetupStatus(status: SetupStatus) {
    if (!browser || isOverlay || isToast) return;
    try {
      localStorage.setItem(setupStatusCacheKey, JSON.stringify(status));
    } catch {
      // 本地缓存只用于首屏体验，失败不影响真实检查。
    }
  }

  function applySetupStatus(status: SetupStatus) {
    setupStatus = status;
    setupStatusLoading = false;
    rememberSetupStatus(status);
  }

  function localSetupStatusFromConfig(configValue: AppConfig, devices = audioDevices): SetupStatus {
    const missingAuth = !configValue.auth.app_key.trim() || !configValue.auth.access_key.trim();
    const anyTriggerEnabled =
      configValue.triggers.hotkey_enabled ||
      configValue.triggers.middle_mouse_enabled ||
      configValue.triggers.right_alt_enabled;
    return {
      ready: !missingAuth && devices.length > 0 && anyTriggerEnabled,
      missing_auth: missingAuth,
      has_audio_device: devices.length > 0,
      hotkey: configValue.hotkey,
      paste_method: configValue.typing.paste_method,
      privacy_recent_context_enabled: configValue.context.enable_recent_context,
      warnings: setupStatus?.warnings ?? [],
    };
  }

  async function loadAll() {
    logFrontendEvent(`loadAll started mode=${frontendMode()}`);
    if (!isOverlay && !isToast && !setupStatus) setupStatusLoading = true;
    const [snapshotResult, configResult, statsResult, devicesResult, setupResult, autoHotwordResult] = await Promise.all([
      safeInvoke<AppSnapshot>("get_app_snapshot"),
      safeInvoke<LoadedConfig>("load_app_config"),
      safeInvoke<StatsSnapshot>("get_usage_stats"),
      safeInvoke<AudioDeviceInfo[]>("list_audio_input_devices"),
      safeInvoke<SetupStatus>("get_setup_status"),
      safeInvoke<AutoHotwordStatus>("get_auto_hotword_status"),
    ]);
    const loadedAny = Boolean(snapshotResult || configResult || statsResult || devicesResult || setupResult || autoHotwordResult);
    if (snapshotResult) snapshot = snapshotResult;
    if (configResult) {
      config = configResult.data;
      savedConfigFingerprint = configFingerprint(configResult.data);
      configExists = configResult.exists;
      configLoaded = true;
      const setupMessage = configSetupMessage(configResult);
      if (setupMessage) {
        statusMessage = setupMessage;
        if (!isOverlay && !isToast && requiresAsrAuth(configResult.data, configResult.exists)) {
          focusAsrAuthSettings();
        }
      }
    }
    if (statsResult) stats = statsResult;
    if (devicesResult) audioDevices = devicesResult;
    if (autoHotwordResult) autoHotwordStatus = autoHotwordResult;
    if (!configResult && hasTauriApi() && !isOverlay && !isToast) configLoaded = true;
    if (setupResult) {
      applySetupStatus(setupResult);
    } else if (!setupStatus && configResult) {
      setupStatus = localSetupStatusFromConfig(configResult.data, devicesResult ?? audioDevices);
    }
    if (!isOverlay && !isToast) setupStatusLoading = false;
    if ((snapshotResult || configResult || statsResult) && !configSetupMessage(configResult)) {
      statusMessage = t("bridgeConnected");
    }
    logFrontendEvent(
      `loadAll completed mode=${frontendMode()} snapshot=${Boolean(snapshotResult)} config_loaded=${Boolean(configResult)} config_exists=${configResult?.exists ?? false} stats_records=${statsResult?.history.length ?? 0} audio_devices=${devicesResult?.length ?? 0} setup_ready=${setupResult?.ready ?? false} auto_hotword_entries=${autoHotwordResult?.entry_count ?? 0}`,
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
    if (sessionPhase !== "succeeded" && succeededIdleTimer !== undefined) {
      window.clearTimeout(succeededIdleTimer);
      succeededIdleTimer = undefined;
    }
    if (!state.recording) audioLevel = 0;
    if (state.phase === "failed" && state.error_code) {
      statusMessage = userErrorMessage(state.error_code, state.message);
      if (shouldOpenSettingsForError(state.message, state.error_code)) {
        scrollToSettingsPanel(settingsPanelForError(state.message, state.error_code));
      }
      return;
    }
    if (isConfigError(state.message)) {
      statusMessage = userErrorMessage(state.error_code, state.message);
      scrollToSettingsPanel(settingsPanelForError(state.message, state.error_code));
      return;
    }
    statusMessage = state.phase === "failed" && state.message ? userErrorMessage(state.error_code, state.message) : sessionPhaseMessage(sessionPhase);
    if (sessionPhase === "succeeded") scheduleSucceededIdleHint();
  }
  function scheduleSucceededIdleHint() {
    if (succeededIdleTimer !== undefined) window.clearTimeout(succeededIdleTimer);
    succeededIdleTimer = window.setTimeout(() => {
      if (sessionPhase !== "succeeded") return;
      sessionPhase = "idle";
      statusMessage = sessionPhaseMessage("idle");
      succeededIdleTimer = undefined;
    }, 2000);
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

  async function persistConfig(options: PersistConfigOptions = {}) {
    const { enforceAuth = true, focusErrors = true } = options;
    if (saving) return null;
    const configToSave = clonePlain(config);
    const saveFingerprint = configFingerprint(configToSave);
    saving = true;
    try {
      validationErrors = {};
      const hotkeyError = validateHotkeyText(configToSave.hotkey);
      if (hotkeyError) {
        validationErrors = { hotkey: hotkeyError };
        statusMessage = hotkeyError;
        if (focusErrors) scrollToSettingsPanel("settings-output");
        return null;
      }
      if (enforceAuth && !requireAuthFields(focusErrors, focusErrors)) return null;
      if (!hasTauriApi()) {
        statusMessage = t("browserPreview");
        return null;
      }
      const result = await invoke<LoadedConfig>("save_app_config", { config: configToSave });
      if (result) {
        const resultFingerprint = configFingerprint(result.data);
        const currentFingerprint = configFingerprint(config);
        savedConfigFingerprint = resultFingerprint;
        if (currentFingerprint === saveFingerprint) config = result.data;
        snapshot = { ...snapshot, hotkey: result.data.hotkey };
        syncSetupStatusFromConfig(result.data);
        configExists = result.exists;
        configLoaded = true;
        statusMessage = t("configSaved");
      }
      return result;
    } catch (error) {
      const saveError = parseConfigSaveError(error);
      const errors = saveError.errors ?? [];
      validationErrors = validationErrorMap(errors);
      if (focusErrors) focusFirstValidationError(errors);
      statusMessage = saveError.message || t("validationFailed");
      logFrontendError(`save config failed: ${formatFrontendError(error)}`);
      return null;
    } finally {
      saving = false;
    }
  }
  function parseConfigSaveError(error: unknown): ConfigSaveError {
    if (typeof error === "object" && error !== null) {
      const maybeError = error as { message?: unknown; errors?: unknown };
      return {
        message: typeof maybeError.message === "string" ? maybeError.message : t("validationFailed"),
        errors: Array.isArray(maybeError.errors) ? (maybeError.errors as ConfigValidationError[]) : [],
      };
    }
    return {
      message: typeof error === "string" ? error : t("validationFailed"),
      errors: [],
    };
  }
  function validationErrorMap(errors: ConfigValidationError[]) {
    return Object.fromEntries(
      errors
        .filter((error) => error.field && error.message)
        .map((error) => [error.field, error.message]),
    );
  }
  function fieldError(field: string) {
    return validationErrors[field] ?? "";
  }
  function firstValidationField(errors: ConfigValidationError[]) {
    return errors.find((error) => error.field)?.field ?? "";
  }
  function focusFirstValidationError(errors: ConfigValidationError[]) {
    const field = firstValidationField(errors);
    if (!field) return;
    if (fieldRequiresAdvancedSettings(field)) showAdvancedSettings = true;
    scrollToSettingsPanel(settingsPanelForField(field));
  }
  function fieldRequiresAdvancedSettings(field: string) {
    return (
      field.startsWith("audio.") && field !== "audio.input_device" ||
      (field.startsWith("ui.") && !["ui.opacity", "ui.background_color", "ui.text_color"].includes(field)) ||
      field === "update.github_repo" ||
      field === "typing.clipboard_restore_delay_ms" ||
      field === "typing.clipboard_snapshot_max_bytes" ||
      field === "typing.clipboard_open_retry_count" ||
      field === "typing.clipboard_open_retry_interval_ms" ||
      field === "llm_post_edit.enable_thinking"
    );
  }
  function settingsPanelForField(field: string) {
    if (field.startsWith("auth.")) return "settings-auth";
    if (field.startsWith("request.")) return "settings-request";
    if (field === "llm_post_edit.system_prompt" || field === "llm_post_edit.user_prompt_template" || field === "llm_post_edit.min_chars") {
      return "settings-llm-prompt";
    }
    if (field.startsWith("llm_post_edit.")) return "settings-llm-api";
    if (field.startsWith("auto_hotwords.")) return "settings-auto-hotwords";
    if (field.startsWith("context.")) return "settings-context";
    if (field.startsWith("audio.")) return "settings-audio";
    if (field.startsWith("ui.")) return "settings-overlay";
    if (field.startsWith("update.")) return "settings-update";
    if (field === "tray.show_startup_message" || field === "tray.startup_message_timeout_ms") return "settings-overlay";
    return "settings-output";
  }
  function syncSetupStatusFromConfig(nextConfig: AppConfig) {
    const missingAuth = !nextConfig.auth.app_key.trim() || !nextConfig.auth.access_key.trim();
    const anyTriggerEnabled =
      nextConfig.triggers.hotkey_enabled ||
      nextConfig.triggers.middle_mouse_enabled ||
      nextConfig.triggers.right_alt_enabled;
    const currentStatus = setupStatus ?? localSetupStatusFromConfig(nextConfig);
    const nextStatus = {
      ...currentStatus,
      missing_auth: missingAuth,
      hotkey: nextConfig.hotkey,
      paste_method: nextConfig.typing.paste_method,
      privacy_recent_context_enabled: nextConfig.context.enable_recent_context,
      ready: !missingAuth && currentStatus.has_audio_device && anyTriggerEnabled,
    };
    applySetupStatus(nextStatus);
  }
  function authFieldErrors() {
    const errors: Record<string, string> = {};
    if (!config.auth.app_key.trim()) errors["auth.app_key"] = t("requiredField");
    if (!config.auth.access_key.trim()) errors["auth.access_key"] = t("requiredField");
    return errors;
  }
  function clearAuthFieldErrors() {
    const next = { ...validationErrors };
    delete next["auth.app_key"];
    delete next["auth.access_key"];
    validationErrors = next;
  }
  function requireAuthFields(showNotice = true, focusTarget = true) {
    const errors = authFieldErrors();
    if (Object.keys(errors).length === 0) {
      clearAuthFieldErrors();
      return true;
    }
    validationErrors = { ...validationErrors, ...errors };
    statusMessage = authGateMessage();
    if (focusTarget) focusAsrAuthSettings();
    if (showNotice) showActionNotice(statusMessage, "warning");
    return false;
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
  async function copyDiagnosticReport() {
    if (copyingDiagnosticReport) return;
    if (!hasTauriApi()) {
      statusMessage = t("browserPreview");
      showActionNotice(statusMessage, "error");
      return;
    }
    copyingDiagnosticReport = true;
    try {
      await invoke<DiagnosticReport>("copy_diagnostic_report_to_clipboard");
      statusMessage = t("diagnosticCopied");
      showActionNotice(t("diagnosticCopied"), "success");
    } catch (error) {
      statusMessage = typeof error === "string" ? error : t("browserPreview");
      logFrontendError(`copy diagnostic report failed: ${formatFrontendError(error)}`);
      showActionNotice(statusMessage, "error");
    } finally {
      copyingDiagnosticReport = false;
    }
  }
  async function clearRecentContextFromUi() {
    if (clearingRecentContext) return;
    if (!hasTauriApi()) {
      statusMessage = t("browserPreview");
      showActionNotice(statusMessage, "error");
      return;
    }
    clearingRecentContext = true;
    try {
      const result = await invoke<ConnectionTestResult>("clear_recent_context");
      statusMessage = result.message || t("recentContextCleared");
      showActionNotice(statusMessage, "success");
      await refreshSetupStatus();
    } catch (error) {
      statusMessage = typeof error === "string" ? error : t("browserPreview");
      logFrontendError(`clear recent context failed: ${formatFrontendError(error)}`);
      showActionNotice(statusMessage, "error");
    } finally {
      clearingRecentContext = false;
    }
  }
  async function testAsrConfig() {
    if (testingAsr) return;
    if (!requireAuthFields()) return;
    testingAsr = true;
    asrConnectionStatus = "testing";
    try {
      const result = await safeInvoke<ConnectionTestResult>("test_asr_config", { config: clonePlain(config) });
      if (result) {
        asrConnectionStatus = "tested_ok";
        asrTestedConfigFingerprint = asrConfigFingerprint();
        statusMessage = result.message;
        showActionNotice(result.message, "success");
      } else if (statusMessage) {
        asrConnectionStatus = "tested_failed";
        asrTestedConfigFingerprint = asrConfigFingerprint();
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
  function showActionNotice(message: string, kind: "success" | "info" | "warning" | "error") {
    actionNotice = message;
    actionNoticeKind = kind;
    if (actionNoticeTimer !== undefined) window.clearTimeout(actionNoticeTimer);
    const baseDuration = kind === "error" ? 6400 : kind === "warning" ? 5200 : 3200;
    const extraDuration = message.length > 80 ? 1800 : 0;
    actionNoticeTimer = window.setTimeout(() => {
      actionNotice = "";
      actionNoticeTimer = undefined;
    }, baseDuration + extraDuration);
  }
  function optionEnabledNotice(key: SoftConfigNoticeKey, enabled: boolean) {
    if (!enabled) return "";
    if (key === "middle_mouse_enabled" || key === "right_alt_enabled") return t("extraTriggerEnabledNotice");
    if (key === "mute_system_volume_while_recording") return t("systemAudioMuteEnabledNotice");
    if (key === "enable_recent_context") return t("recentContextEnabledNotice");
    return "";
  }
  function maybeShowOptionEnabledNotice(key: SoftConfigNoticeKey, enabled: boolean) {
    const notice = optionEnabledNotice(key, enabled);
    if (notice) showActionNotice(notice, "info");
  }
  async function toggleTrigger(key: TriggerKey) {
    if (saving) return;
    const previous = config.triggers[key];
    config.triggers[key] = !previous;
    const result = await persistConfig({ enforceAuth: false });
    if (!result) {
      config.triggers[key] = previous;
      if (statusMessage) showActionNotice(statusMessage, "error");
      return;
    }
    const notice = optionEnabledNotice(key, !previous);
    showActionNotice(notice || t("configSaved"), notice ? "info" : "success");
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
  function overlayMeterBarHeight(index: number) {
    const level = meterLevel();
    const quietHeights = [5, 8, 10, 7];
    const activeHeights = [8, 13, 17, 11];
    const threshold = 0.1 + index * 0.16;
    const target = recording && level >= threshold ? activeHeights[index] : quietHeights[index];
    return `${target}px`;
  }
  function overlayMeterBarOpacity(index: number) {
    if (!recording) return "0.42";
    const level = meterLevel();
    return level >= 0.1 + index * 0.16 ? "0.92" : "0.34";
  }
  function currentAudioDevice() {
    if (audioDevices.length === 0) return null;
    if (config.audio.input_device !== null && config.audio.input_device !== undefined) {
      const configured = audioDevices.find((device) => device.index === config.audio.input_device);
      if (configured) return configured;
    }
    return audioDevices.find((device) => device.is_default) ?? audioDevices[0];
  }
  async function refreshSetupStatus(showLoading = true) {
    if (showLoading || !setupStatus) setupStatusLoading = true;
    const [devicesResult, setupResult] = await Promise.all([
      safeInvoke<AudioDeviceInfo[]>("list_audio_input_devices", undefined, true),
      safeInvoke<SetupStatus>("get_setup_status", undefined, true),
    ]);
    if (devicesResult) audioDevices = devicesResult;
    if (setupResult) {
      applySetupStatus(setupResult);
    } else if (!setupStatus) {
      setupStatus = localSetupStatusFromConfig(config, devicesResult ?? audioDevices);
    }
    setupStatusLoading = false;
  }
  function setupStatusItems(): SetupStatusItem[] {
    if (setupStatusLoading && !setupStatus) {
      return [
        {
          label: t("setupAuthLabel"),
          value: t("setupChecking"),
          ok: false,
          checking: true,
          action: "asr_auth",
        },
        {
          label: t("setupMicLabel"),
          value: t("setupChecking"),
          ok: false,
          checking: true,
          action: "audio",
        },
        {
          label: t("setupPasteLabel"),
          value: t("setupChecking"),
          ok: false,
          checking: true,
          action: "typing",
        },
        {
          label: t("setupTriggerLabel"),
          value: t("setupChecking"),
          ok: false,
          checking: true,
          action: "hotkey",
        },
        {
          label: t("setupPrivacyLabel"),
          value: t("setupChecking"),
          ok: false,
          checking: true,
          action: "privacy",
        },
      ];
    }
    const status = setupStatus ?? localSetupStatusFromConfig(config);
    const asrStatus = currentAsrConnectionStatus(status);
    const micReady = status ? status.has_audio_device : audioDevices.length > 0;
    return [
      {
        label: t("setupAuthLabel"),
        value: asrConnectionStatusText(asrStatus),
        ok: asrConnectionStatusOk(asrStatus),
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
        value: pasteMethodLabel(configLoaded ? config.typing.paste_method : status.paste_method),
        ok: true,
        action: "typing",
      },
      {
        label: t("setupTriggerLabel"),
        value: formatEnabledTriggers(),
        ok: config.triggers.hotkey_enabled || config.triggers.middle_mouse_enabled || config.triggers.right_alt_enabled,
        action: "hotkey",
      },
      {
        label: t("setupPrivacyLabel"),
        value: t("setupPrivacyChecked"),
        ok: true,
        action: "privacy",
      },
    ];
  }
  function setupWarningCount() {
    if (setupStatusLoading && !setupStatus) return 0;
    return setupStatusItems().filter((item) => !item.ok).length;
  }
  function setupIsReady() {
    if (setupStatusLoading && !setupStatus) return false;
    const status = setupStatus ?? localSetupStatusFromConfig(config);
    const baseReady = status.ready;
    return baseReady && currentAsrConnectionStatus(status) !== "tested_failed";
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
    scrollToSettingsPanel(targetId);
  }
  function pasteMethodLabel(value: string) {
    if (value === "clipboard_only") return t("clipboardOnly");
    if (value === "shift_insert") return "Shift + Insert";
    return "Ctrl + V";
  }
  function asrConfigFingerprint(configValue = config) {
    return JSON.stringify({
      app_key: configValue.auth.app_key,
      access_key: configValue.auth.access_key,
      resource_id: configValue.auth.resource_id,
      ws_url: configValue.request.ws_url,
      model_name: configValue.request.model_name,
    });
  }
  function currentAsrConnectionStatus(status: SetupStatus | null = null): AsrConnectionStatus {
    const authReady = status ? !status.missing_auth : hasAuth();
    if (!authReady) return "missing_auth";
    if (testingAsr) return "testing";
    const currentFingerprint = asrConfigFingerprint();
    if (
      asrTestedConfigFingerprint === currentFingerprint &&
      (asrConnectionStatus === "tested_ok" || asrConnectionStatus === "tested_failed")
    ) {
      return asrConnectionStatus;
    }
    return "configured_not_tested";
  }
  function asrConnectionStatusText(status: AsrConnectionStatus) {
    if (status === "missing_auth") return t("setupAsrMissingAuth");
    if (status === "testing") return t("setupAsrTesting");
    if (status === "tested_ok") return t("setupAsrTestedOk");
    if (status === "tested_failed") return t("setupAsrTestedFailed");
    return t("setupAsrConfiguredNotTested");
  }
  function asrConnectionStatusOk(status: AsrConnectionStatus) {
    return status === "configured_not_tested" || status === "tested_ok";
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
    if (!device) return !configLoaded && setupStatus?.has_audio_device ? t("setupMicDetected") : t("micUnavailable");
    return recording
      ? t("micMonitoring", { device: device.name })
      : t("micConnected", { device: device.name });
  }
  function sidebarMicStatusText() {
    return currentAudioDevice() || (!configLoaded && setupStatus?.has_audio_device)
      ? t("sidebarMicConnected")
      : t("sidebarMicUnavailable");
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
        if (normalized === "space") return "Space";
        if (normalized === "enter") return "Enter";
        if (normalized === "tab") return "Tab";
        return part.trim().toUpperCase();
      })
      .filter(Boolean)
      .join(" + ");
  }

  function setHotkey(value: string) {
    const formatted = formatHotkey(value);
    config.hotkey = formatted;
    hotkeyValidationMessage = validateHotkeyText(formatted);
    if (!hotkeyValidationMessage) {
      const next = { ...validationErrors };
      delete next.hotkey;
      validationErrors = next;
    }
  }

  function beginHotkeyCapture() {
    hotkeyValidationMessage = "";
    hotkeyCaptureState = "recording";
  }

  function cancelHotkeyCapture() {
    hotkeyCaptureState = "idle";
    hotkeyValidationMessage = "";
  }

  function handleHotkeyKeydown(event: KeyboardEvent) {
    if (hotkeyCaptureState !== "recording") return;
    event.preventDefault();
    event.stopPropagation();
    if (event.key === "Escape") {
      cancelHotkeyCapture();
      return;
    }
    if (event.key === "Backspace" || event.key === "Delete") {
      config.hotkey = "";
      hotkeyValidationMessage = t("hotkeyRequired");
      validationErrors = { ...validationErrors, hotkey: hotkeyValidationMessage };
      return;
    }
    if (event.key === "Enter" && !event.ctrlKey && !event.altKey && !event.shiftKey && !event.metaKey) {
      hotkeyCaptureState = "idle";
      return;
    }
    const captured = hotkeyFromKeyboardEvent(event);
    if (!captured) {
      hotkeyValidationMessage = t("hotkeyUnsupported");
      return;
    }
    config.hotkey = captured;
    hotkeyValidationMessage = validateHotkeyText(captured);
    validationErrors = hotkeyValidationMessage
      ? { ...validationErrors, hotkey: hotkeyValidationMessage }
      : Object.fromEntries(Object.entries(validationErrors).filter(([field]) => field !== "hotkey"));
    if (!hotkeyValidationMessage) hotkeyCaptureState = "idle";
  }

  function hotkeyFromKeyboardEvent(event: KeyboardEvent) {
    const key = normalizedHotkeyMainKey(event.key);
    if (!key) return "";
    const parts: string[] = [];
    if (event.ctrlKey) parts.push("Ctrl");
    if (event.altKey) parts.push("Alt");
    if (event.shiftKey) parts.push("Shift");
    if (event.metaKey) parts.push("Win");
    if (parts.length === 0) return "";
    parts.push(key);
    return parts.join(" + ");
  }

  function normalizedHotkeyMainKey(key: string) {
    if (/^[a-z]$/i.test(key)) return key.toUpperCase();
    if (/^[0-9]$/.test(key)) return key;
    if (/^F([1-9]|1[0-2])$/i.test(key)) return key.toUpperCase();
    if (key === " " || key.toLowerCase() === "space" || key === "Spacebar") return "Space";
    if (key.toLowerCase() === "tab") return "Tab";
    if (key.toLowerCase() === "enter") return "Enter";
    return "";
  }

  function validateHotkeyText(value: string) {
    const parts = value
      .split("+")
      .map((part) => part.trim())
      .filter(Boolean);
    if (parts.length === 0) return t("hotkeyRequired");
    const modifiers = parts.slice(0, -1).map((part) => part.toLowerCase());
    const key = parts[parts.length - 1];
    const hasModifier = modifiers.some((part) => ["ctrl", "control", "alt", "shift", "win", "meta"].includes(part));
    if (!hasModifier) return t("hotkeyNeedsModifier");
    if (!normalizedHotkeyMainKey(key)) return t("hotkeyUnsupported");
    return "";
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
  async function hideWindowToTray(markSeen: boolean) {
    closePromptVisible = false;
    if (markSeen) {
      await saveClosePreference(config.tray.close_behavior, true);
    }
    await safeInvoke<void>("hide_main_window", undefined, true);
  }
  async function closeWindowWithoutFuturePrompt() {
    closePromptVisible = false;
    await saveClosePreference("close_to_tray", true);
    await safeInvoke<void>("hide_main_window", undefined, true);
  }
  async function exitFromClosePrompt() {
    closePromptVisible = false;
    await safeInvoke<void>("exit_application", undefined, true);
  }
  async function saveClosePreference(behavior: string, noticeShown: boolean) {
    const result = await safeInvoke<LoadedConfig>(
      "update_close_preference",
      {
        closeBehavior: behavior,
        closeToTrayNoticeShown: noticeShown,
      },
      true,
    );
    if (result) {
      config = result.data;
      savedConfigFingerprint = configFingerprint(result.data);
      configExists = result.exists;
    }
  }

  function settingsToolbarMessage() {
    if (saving) return t("settingsAutoSavingHint");
    if (Object.keys(validationErrors).length > 0 && statusMessage) return statusMessage;
    if (settingsDirty) return t("settingsAutoSavePendingHint");
    return statusMessage || t("settingsActionHint");
  }

  function updateHotwords(value: string) {
    config.context.hotwords = normalizeHotwords(value);
  }

  function normalizeHotwords(value: string) {
    return value
      .split("\n")
      .map((item) => item.trim())
      .filter(Boolean);
  }

  function hotwordCount() {
    return config.context.hotwords.filter((item) => item.trim()).length;
  }

  function tidyHotwords() {
    const seen = new Set<string>();
    config.context.hotwords = config.context.hotwords
      .map((item) => item.trim())
      .filter((item) => {
        if (!item) return false;
        const key = item.toLocaleLowerCase();
        if (seen.has(key)) return false;
        seen.add(key);
        return true;
      });
    showActionNotice(t("hotwordsTidied"), "success");
  }

  function clearHotwords() {
    if (!browser || window.confirm(t("clearHotwordsConfirm"))) {
      config.context.hotwords = [];
      showActionNotice(t("hotwordsCleared"), "success");
    }
  }

  function updatePromptContext(value: string) {
    config.context.prompt_context = value
      .split("\n")
      .map((text) => text.trim())
      .filter(Boolean)
      .map((text) => ({ text }));
  }

  function restoreDefaultLlmPrompt() {
    config.llm_post_edit.system_prompt = fallbackConfig.llm_post_edit.system_prompt;
    config.llm_post_edit.user_prompt_template = fallbackConfig.llm_post_edit.user_prompt_template;
    showActionNotice(t("defaultPromptRestored"), "success");
  }

  function previewFinalPrompt() {
    const sampleText = t("promptPreviewSampleText");
    let userPrompt = config.llm_post_edit.user_prompt_template.replace("{text}", sampleText);
    const hotwords = config.context.hotwords.map((item) => item.trim()).filter(Boolean);
    if (hotwords.length > 0) {
      userPrompt += `\n\n${t("promptPreviewUserDictionary")}\n${hotwords.join("\n")}`;
    }
    const promptContext = config.context.prompt_context.map((item) => item.text.trim()).filter(Boolean);
    if (promptContext.length > 0) {
      userPrompt += `\n\n${t("promptPreviewContextTitle")}\n${promptContext.map((item) => `- ${item}`).join("\n")}`;
    }
    window.alert(`${t("systemPrompt")}\n${config.llm_post_edit.system_prompt || t("promptPreviewEmpty")}\n\n${t("userPromptTemplate")}\n${userPrompt}`);
  }

  async function refreshAutoHotwordStatus() {
    const result = await safeInvoke<AutoHotwordStatus>("get_auto_hotword_status", undefined, true);
    if (result) autoHotwordStatus = result;
  }

  function autoHotwordStatusText() {
    if (!autoHotwordStatus) return t("autoHotwordsStatusUnknown");
    return t("autoHotwordsStatus", {
      entries: String(autoHotwordStatus.entry_count),
      chars: String(autoHotwordStatus.total_chars),
      max: String(autoHotwordStatus.max_history_chars),
    });
  }

  async function generateAutoHotwords() {
    if (generatingAutoHotwords) return;
    generatingAutoHotwords = true;
    autoHotwordError = "";
    try {
      const result = await safeInvoke<HotwordGenerationResult>(
        "generate_hotword_candidates",
        { config: clonePlain(config) },
        false,
      );
      if (!result) {
        autoHotwordError = statusMessage;
        return;
      }
      autoHotwordCandidates = result.candidates.map((item) => ({ ...item, selected: true }));
      const message = t("autoHotwordsGenerated", { count: String(result.candidates.length) });
      statusMessage = message;
      showActionNotice(message, result.candidates.length > 0 ? "success" : "warning");
      if (result.warning) showActionNotice(result.warning, "warning");
    } finally {
      generatingAutoHotwords = false;
    }
  }

  async function clearAutoHotwordHistoryFromUi() {
    if (clearingAutoHotwordHistory) return;
    if (browser && !window.confirm(t("autoHotwordsClearConfirm"))) return;
    clearingAutoHotwordHistory = true;
    autoHotwordError = "";
    try {
      const result = await safeInvoke<ConnectionTestResult>("clear_hotword_history", undefined, false);
      if (result) {
        showActionNotice(result.message, "success");
        autoHotwordCandidates = [];
        await refreshAutoHotwordStatus();
      } else {
        autoHotwordError = statusMessage;
      }
    } finally {
      clearingAutoHotwordHistory = false;
    }
  }

  function selectedAutoHotwordCount() {
    return autoHotwordCandidates.filter((item) => item.selected).length;
  }

  function applySelectedAutoHotwords() {
    const selected = autoHotwordCandidates
      .filter((item) => item.selected)
      .map((item) => item.word.trim())
      .filter(Boolean);
    if (selected.length === 0) {
      showActionNotice(t("autoHotwordsNoSelection"), "warning");
      return;
    }

    const merged = [...config.context.hotwords];
    const seen = new Set(merged.map((item) => item.trim().toLocaleLowerCase()).filter(Boolean));
    let added = 0;
    for (const word of selected) {
      const key = word.toLocaleLowerCase();
      if (seen.has(key)) continue;
      seen.add(key);
      merged.push(word);
      added += 1;
    }
    config.context.hotwords = merged;
    const message = t("autoHotwordsApplied", { count: String(added) });
    statusMessage = message;
    showActionNotice(message, added > 0 ? "success" : "warning");
  }

  function candidateConfidenceLabel(value: number) {
    const safeValue = Number.isFinite(value) ? Math.max(0, Math.min(1, value)) : 0;
    return `${Math.round(safeValue * 100)}%`;
  }

  function normalizedHexColor(value: string | undefined, fallback: string) {
    const trimmed = (value ?? "").trim();
    return /^#[0-9a-fA-F]{6}$/.test(trimmed) ? trimmed : fallback;
  }

  function overlayBackgroundColor() {
    return normalizedHexColor(config.ui.background_color, fallbackConfig.ui.background_color);
  }

  function overlayTextColor() {
    return normalizedHexColor(config.ui.text_color, fallbackConfig.ui.text_color);
  }

  function overlayBackgroundRgb() {
    const hex = overlayBackgroundColor().slice(1);
    const red = parseInt(hex.slice(0, 2), 16);
    const green = parseInt(hex.slice(2, 4), 16);
    const blue = parseInt(hex.slice(4, 6), 16);
    return `${red}, ${green}, ${blue}`;
  }

  function overlayOpacity() {
    const value = Number(config.ui.opacity);
    if (!Number.isFinite(value)) return fallbackConfig.ui.opacity;
    return Math.min(1, Math.max(0.05, value));
  }

  function overlayOpacityLabel(value: number) {
    return `${Math.round(value * 100)}%`;
  }

  function applyOverlayOpacity(value: number) {
    config.ui.opacity = value;
  }

  function overlayOpacityPresetActive(value: number) {
    return Math.abs(overlayOpacity() - value) < 0.001;
  }

  function applyOverlayPreset(background: string, text: string) {
    config.ui.background_color = background;
    config.ui.text_color = text;
  }

  function overlayPresetActive(background: string, text: string) {
    return overlayBackgroundColor().toLowerCase() === background.toLowerCase() && overlayTextColor().toLowerCase() === text.toLowerCase();
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
  function requiresAsrAuth(configValue?: AppConfig, exists?: boolean) {
    if (configValue === undefined && exists === undefined && !configLoaded) {
      return setupStatus ? setupStatus.missing_auth : false;
    }
    const targetConfig = configValue ?? config;
    const targetExists = exists ?? configExists;
    return !targetExists || !hasAuth(targetConfig);
  }
  function authGateMessage() {
    return !configExists ? t("setupMissingFile") : t("authGateNotice");
  }
  function setupRequiredMessage() {
    return !configExists ? t("setupMissingFile") : t("setupMissingAuth");
  }
  function scrollToSettingsPanel(targetId: string) {
    if (!browser) return;
    selectedSection = sectionForSettingsPanel(targetId);
    window.setTimeout(() => {
      document.getElementById(targetId)?.scrollIntoView({ block: "start", behavior: "smooth" });
    }, 50);
  }
  function sectionForSettingsPanel(targetId: string): Section {
    if (targetId === "settings-context" || targetId === "settings-prompt-context" || targetId === "settings-llm-prompt") {
      return "Hotwords";
    }
    if (targetId === "settings-auth" || targetId === "settings-request" || targetId === "settings-llm-api") {
      return "ApiConfig";
    }
    return "Options";
  }
  function focusAsrAuthSettings() {
    scrollToSettingsPanel("settings-auth");
  }
  function requireAsrAuthGate(showNotice = true) {
    if (!requiresAsrAuth()) return false;
    statusMessage = authGateMessage();
    focusAsrAuthSettings();
    if (showNotice) showActionNotice(statusMessage, "warning");
    return true;
  }
  function selectSection(section: Section) {
    selectedSection = section;
    if (section === "ApiConfig" && requiresAsrAuth()) scrollToSettingsPanel("settings-auth");
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
      message.includes("豆包 ASR Key") ||
      message.includes("Doubao ASR") ||
      message.includes("App Key") ||
      message.includes("Access Key") ||
      message.includes("Resource ID")
    );
  }
  function userErrorDetail(code: string | null | undefined, fallback = ""): UserErrorDetail {
    const matchedDetail = code ? userErrorDetails[language][code] : undefined;
    if (matchedDetail) return matchedDetail;
    if (isConfigError(fallback)) return userErrorDetails[language].ASR_AUTH_MISSING;
    if (fallback.includes("剪贴板") || fallback.toLowerCase().includes("clipboard")) {
      return userErrorDetails[language].CLIPBOARD_WRITE_FAILED;
    }
    if (fallback.includes("麦克风") || fallback.toLowerCase().includes("microphone")) {
      return userErrorDetails[language].MIC_START_FAILED;
    }
    return {
      title: t("inputError"),
      cause: fallback || t("sessionFailed"),
      action: t("genericErrorAction"),
    };
  }
  function userErrorMessage(code: string | null | undefined, fallback = "") {
    const detail = userErrorDetail(code, fallback);
    const separator = language === "en" ? ". " : "。";
    return `${detail.title}${separator}${detail.action}`;
  }
  function activeUserErrorDetail() {
    if (inputStatus() !== "error") return null;
    return userErrorDetail(sessionErrorCode, statusMessage);
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
  function shouldOpenSettingsForError(message: string, code?: string | null) {
    return (
      code === "CONFIG_MISSING" ||
      code === "ASR_AUTH_MISSING" ||
      code === "MIC_DEVICE_NOT_FOUND" ||
      isConfigError(message) ||
      message.includes("API Key") ||
      message.includes("Base URL")
    );
  }
  function settingsPanelForError(message: string, code?: string | null) {
    if (code === "MIC_DEVICE_NOT_FOUND" || message.includes("麦克风") || message.toLowerCase().includes("microphone")) {
      return "settings-audio";
    }
    return "settings-auth";
  }
  function openSettings() {
    scrollToSettingsPanel("settings-auth");
  }
  async function openSetupGuide() {
    await safeInvoke<void>("open_setup_guide");
  }
</script>

<svelte:head>
  <title>VoxType</title>
</svelte:head>

{#if isOverlay}
  <main class="overlay-root" style={`--overlay-bg: ${overlayBackgroundColor()}; --overlay-bg-rgb: ${overlayBackgroundRgb()}; --overlay-opacity: ${overlayOpacity()}; --overlay-text: ${overlayTextColor()};`}>
    <div class="overlay-caption">
      <div class:active={recording} class="overlay-voice-meter" aria-hidden="true">
        {#each overlayMeterBars as bar}
          <i style:height={overlayMeterBarHeight(bar)} style:opacity={overlayMeterBarOpacity(bar)}></i>
        {/each}
      </div>
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
        <button
          class:active={selectedSection === item.id}
          onclick={() => selectSection(item.id)}
        >
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
    class:overview-content={selectedSection === "Home"}
    class:setup-required={requiresAsrAuth()}
    class="content"
  >
    <header class="topbar">
      <div>
        <p class="eyebrow">{t("topEyebrow")}</p>
        <h2>{t(navLabelKeys[selectedSection])}</h2>
      </div>
    </header>

    {#if selectedSection === "Home"}
      <section class="voice-card">
        <div class="section-title-row">
          <h3>{t("voiceInputTitle")}</h3>
        </div>
        {#if requiresAsrAuth()}
          <div class="setup-alert">
            <div>
              <strong>{t("setupRequired")}</strong>
              <p>{setupRequiredMessage()}</p>
            </div>
            <div class="setup-actions">
              <button onclick={openSettings}>{t("setupCta")}</button>
              <button class="secondary" onclick={openSetupGuide}>{t("setupGuideCta")}</button>
            </div>
          </div>
        {/if}
        {#if inputStatus() === "error"}
          {@const detail = activeUserErrorDetail()}
          {#if detail}
            <div class="error-help-card">
              <strong>{detail.title}</strong>
              <p><span>{t("errorCauseLabel")}：</span>{detail.cause}</p>
              <p><span>{t("errorActionLabel")}：</span>{detail.action}</p>
            </div>
          {/if}
        {/if}
        <div class:listening={recording} class:error={inputStatus() === "error"} class:locked={requiresAsrAuth()} class="voice-hero">
          <button class:listening={recording || isSessionBusy()} class="mic-orb" aria-label={requiresAsrAuth() ? t("authGateTitle") : recording ? t("clickStop") : t("clickStart")} onclick={toggleRecordingFromUi} disabled={isSessionBusy() || requiresAsrAuth()}>
            <span class="mic-ring"><Mic size={uiCompact ? 34 : 42} strokeWidth={2.15} /></span>
          </button>
          <div class="voice-copy">
            <div class="hero-status">
              <span class="hero-dot" class:listening={recording} class:error={inputStatus() === "error"}></span>
              <strong>{inputStatusLabel()}</strong>
            </div>
            <h4>{requiresAsrAuth() ? t("authGateTitle") : recording ? t("clickStop") : isSessionBusy() ? inputStatusLabel() : t("clickStart")}</h4>
            <p>{requiresAsrAuth() ? t("authGateDescription") : inputStatusDesc()}</p>
            <div class="hero-features">
              <span><MessageSquareText size={17} />{t("speakAnywhere")}</span>
              <span><Globe2 size={17} />{t("mixedInput")}</span>
            </div>
          </div>
          <button class="shortcut-help" onclick={() => selectSection("Options")}>
            <Keyboard size={14} />
            <span>{formatHotkey(snapshot.hotkey)}</span>
          </button>
        </div>
      </section>
      <section class="launch-card">
        <div class="section-title-row">
          <div>
            <Keyboard size={20} />
            <h3>{t("desktopControl")}</h3>
          </div>
          <button class="link-action" onclick={() => selectSection("Options")}>
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
    {:else if selectedSection === "Hotwords"}
      <section class="settings-stack">
        <SettingsToolbar
          title={t("settingsActionTitle")}
          hint={t("settingsActionHint")}
          statusMessage={settingsToolbarMessage()}
          reloadLabel={t("reload")}
          {saving}
          dirty={settingsDirty}
          onReload={reloadConfigFromUi}
        />
        <section class="settings-group">
          <div class="settings-group-heading">
            <h3>{t("hotwordsPageTitle")}</h3>
            <p>{t("hotwordsPageDescription")}</p>
          </div>
          <div id="settings-context" class="form-panel">
            <div class="section-heading with-actions">
              <div class="section-heading-copy">
                <h3>{t("asrHotwords")}</h3>
                <p>{t("hotwordsPrivacyHint")}</p>
              </div>
              <div class="settings-inline-actions">
                <button class="test-button" type="button" onclick={tidyHotwords}><Sparkles size={16} />{t("tidyHotwords")}</button>
                <button class="test-button" type="button" onclick={clearHotwords}><Trash2 size={16} />{t("clearHotwords")}</button>
              </div>
            </div>
            <p class="field-hint">{t("hotwordCount", { count: String(hotwordCount()) })}</p>
            <label><span>{t("hotwords")}</span><textarea value={config.context.hotwords.join("\n")} oninput={(event) => updateHotwords(event.currentTarget.value)}></textarea></label>
          </div>
          <div id="settings-prompt-context" class="form-panel">
            <div class="section-heading with-actions">
              <div class="section-heading-copy"><h3>{t("sceneContext")}</h3><p>{t("sceneContextDescription")}</p></div>
              <button class="test-button" onclick={clearRecentContextFromUi} disabled={clearingRecentContext}>
                <Trash2 size={16} />{clearingRecentContext ? t("clearingRecentContext") : t("clearRecentContext")}
              </button>
            </div>
            <label><span>{t("promptContext")}</span><textarea value={config.context.prompt_context.map((item) => item.text).join("\n")} oninput={(event) => updatePromptContext(event.currentTarget.value)}></textarea></label>
            <div class="form-grid">
              <label><span>{t("recentContextRounds")}</span><input type="number" bind:value={config.context.recent_context_rounds} /></label>
            </div>
            <div class="toggle-grid">
              <label class="check"><input type="checkbox" bind:checked={config.context.enable_recent_context} onchange={(event) => maybeShowOptionEnabledNotice("enable_recent_context", event.currentTarget.checked)} />{t("useRecentContext")}</label>
            </div>
            <p class="field-hint">{t("recentContextHint")}</p>
          </div>
          <div id="settings-llm-prompt" class="form-panel">
            <div class="section-heading with-actions">
              <div class="section-heading-copy"><h3>{t("llmPromptSettings")}</h3><p>{t("llmPromptDescription")}</p></div>
              <div class="settings-inline-actions">
                <button class="test-button" type="button" onclick={restoreDefaultLlmPrompt}><Sparkles size={16} />{t("restoreDefaultPrompt")}</button>
                <button class="test-button" type="button" onclick={previewFinalPrompt}><FileText size={16} />{t("previewFinalPrompt")}</button>
              </div>
            </div>
            <div class="form-grid">
              <label><span>{t("minChars")}</span><input type="number" bind:value={config.llm_post_edit.min_chars} /></label>
            </div>
            <label><span>{t("systemPrompt")}</span><textarea bind:value={config.llm_post_edit.system_prompt}></textarea></label>
            <label class:field-invalid={Boolean(fieldError("llm_post_edit.user_prompt_template"))}>
              <span>{t("userPromptTemplate")}</span>
              <textarea bind:value={config.llm_post_edit.user_prompt_template}></textarea>
              {#if fieldError("llm_post_edit.user_prompt_template")}<small class="field-error">{fieldError("llm_post_edit.user_prompt_template")}</small>{/if}
            </label>
          </div>
          <div id="settings-auto-hotwords" class="form-panel auto-hotwords-panel">
            <div class="section-heading with-actions">
              <div class="section-heading-copy">
                <h3>{t("autoHotwordsTitle")}</h3>
                <p>{t("autoHotwordsDescription")}</p>
              </div>
              <div class="settings-inline-actions">
                <button class="test-button" type="button" onclick={generateAutoHotwords} disabled={generatingAutoHotwords}>
                  <Sparkles size={16} />{generatingAutoHotwords ? t("autoHotwordsGenerating") : t("autoHotwordsGenerate")}
                </button>
                <button class="test-button" type="button" onclick={clearAutoHotwordHistoryFromUi} disabled={clearingAutoHotwordHistory}>
                  <Trash2 size={16} />{clearingAutoHotwordHistory ? t("autoHotwordsClearing") : t("autoHotwordsClearHistory")}
                </button>
              </div>
            </div>
            <div class="toggle-grid">
              <label class="check"><input type="checkbox" bind:checked={config.auto_hotwords.enabled} />{t("autoHotwordsEnabled")}</label>
            </div>
            <p class="field-hint">{t("autoHotwordsPrivacyHint")}</p>
            <div class="form-grid">
              <label class:field-invalid={Boolean(fieldError("auto_hotwords.max_history_chars"))}>
                <span>{t("autoHotwordsMaxHistoryChars")}</span>
                <input type="number" min="1000" max="20000" bind:value={config.auto_hotwords.max_history_chars} />
                {#if fieldError("auto_hotwords.max_history_chars")}<small class="field-error">{fieldError("auto_hotwords.max_history_chars")}</small>{/if}
              </label>
              <label class:field-invalid={Boolean(fieldError("auto_hotwords.max_candidates"))}>
                <span>{t("autoHotwordsMaxCandidates")}</span>
                <input type="number" min="5" max="100" bind:value={config.auto_hotwords.max_candidates} />
                {#if fieldError("auto_hotwords.max_candidates")}<small class="field-error">{fieldError("auto_hotwords.max_candidates")}</small>{/if}
              </label>
            </div>
            <div class="auto-hotword-status">
              <Info size={16} />
              <span>{autoHotwordStatusText()}</span>
              <button class="link-button" type="button" onclick={refreshAutoHotwordStatus}>{t("refreshStatus")}</button>
            </div>
            {#if autoHotwordError}
              <p class="field-error">{autoHotwordError}</p>
            {/if}
            {#if autoHotwordCandidates.length > 0}
              <div class="auto-hotword-candidates">
                <div class="candidate-list-head">
                  <strong>{t("autoHotwordsCandidatesTitle", { count: String(autoHotwordCandidates.length) })}</strong>
                  <button class="test-button" type="button" onclick={applySelectedAutoHotwords}>
                    <Check size={16} />{t("autoHotwordsApplySelected", { count: String(selectedAutoHotwordCount()) })}
                  </button>
                </div>
                {#each autoHotwordCandidates as candidate}
                  <label class="candidate-row">
                    <input type="checkbox" bind:checked={candidate.selected} />
                    <span class="candidate-copy">
                      <strong>{candidate.word}</strong>
                      <small>{candidate.category || t("autoHotwordsUnknownCategory")} · {candidateConfidenceLabel(candidate.confidence)} · {t("autoHotwordsSourceCount", { count: String(candidate.source_count) })}</small>
                      <span>{candidate.reason}</span>
                    </span>
                  </label>
                {/each}
              </div>
            {/if}
          </div>
        </section>
      </section>
    {:else if selectedSection === "ApiConfig"}
      <section class="settings-stack">
        {#if requiresAsrAuth()}
          <section class="auth-gate-card" aria-live="polite">
            <div>
              <strong>{t("authGateTitle")}</strong>
              <p>{!configExists ? t("setupMissingFile") : t("authGateDescription")}</p>
            </div>
            <div class="setup-actions">
              <button onclick={() => scrollToSettingsPanel("settings-auth")}>{t("setupCta")}</button>
              <button class="secondary" onclick={openSetupGuide}>{t("setupGuideCta")}</button>
            </div>
          </section>
        {/if}
        <SettingsToolbar
          title={t("settingsActionTitle")}
          hint={t("settingsActionHint")}
          statusMessage={settingsToolbarMessage()}
          reloadLabel={t("reload")}
          {saving}
          dirty={settingsDirty}
          onReload={reloadConfigFromUi}
        />
        <SetupStatusCard
          ready={setupIsReady()}
          checking={setupStatusLoading && !setupStatus}
          items={setupStatusItems()}
          warnings={setupStatus?.warnings ?? []}
          texts={{
            title: t("setupHealthTitle"),
            pendingTitle: t("setupHealthPendingTitle", { count: String(setupWarningCount()) }),
            pendingDescription: t("setupHealthPendingDescription"),
            checkingTitle: t("setupHealthCheckingTitle"),
            checkingDescription: t("setupHealthCheckingDescription"),
            readyTitle: t("setupHealthReadyTitle"),
            readyDescription: t("setupHealthReadyDescription", { hotkey: formatHotkey(snapshot.hotkey) }),
            refresh: t("refreshSetup"),
            actionText: setupActionText,
          }}
          onAction={handleSetupAction}
          onRefresh={refreshSetupStatus}
        />
        <section class="settings-mode-panel">
          <div>
            <h3>{showAdvancedSettings ? t("advancedSettings") : t("basicSettings")}</h3>
            <p>{showAdvancedSettings ? t("advancedSettingsHint") : t("basicSettingsHint")}</p>
          </div>
          <button type="button" onclick={() => (showAdvancedSettings = !showAdvancedSettings)}>
            {showAdvancedSettings ? t("hideAdvancedSettings") : t("showAdvancedSettings")}
          </button>
        </section>
        <section class="settings-group">
          <div class="settings-group-heading">
            <h3>{t("apiConfigPageTitle")}</h3>
            <p>{t("apiConfigPageDescription")}</p>
          </div>
          <div id="settings-auth" class="form-panel">
            <div class="section-heading with-actions">
              <div class="section-heading-copy">
                <h3>{t("doubaoAuth")}</h3>
                {#if requiresAsrAuth()}
                  <p class="setup-note">{setupRequiredMessage()}</p>
                  <button class="link-button" onclick={openSetupGuide}>{t("setupGuideCta")}</button>
                {/if}
              </div>
              <div class="settings-inline-actions">
                <button class="test-button" onclick={testAsrConfig} disabled={testingAsr}>
                  <ShieldCheck size={16} />{testingAsr ? t("testingConnection") : t("testConnection")}
                </button>
              </div>
            </div>
            <div class="form-grid">
              <label><span>{t("resourceId")}</span><input bind:value={config.auth.resource_id} /></label>
              <label class:field-invalid={Boolean(fieldError("auth.app_key"))}>
                <span>{t("appKey")}</span>
                <input autocomplete="off" bind:value={config.auth.app_key} />
                {#if fieldError("auth.app_key")}<small class="field-error">{fieldError("auth.app_key")}</small>{/if}
              </label>
              <label class:field-invalid={Boolean(fieldError("auth.access_key"))}>
                <span>{t("accessKey")}</span>
                <input type="password" autocomplete="off" bind:value={config.auth.access_key} />
                {#if fieldError("auth.access_key")}<small class="field-error">{fieldError("auth.access_key")}</small>{/if}
              </label>
            </div>
          </div>
          <div id="settings-request" class="form-panel">
            <div class="section-heading"><h3>{t("recognitionOptions")}</h3><p>{t("asrDescription")}</p></div>
            <label class:field-invalid={Boolean(fieldError("request.ws_url"))}>
              <span>{t("websocketUrl")}</span>
              <input bind:value={config.request.ws_url} />
              {#if fieldError("request.ws_url")}<small class="field-error">{fieldError("request.ws_url")}</small>{/if}
            </label>
            <div class="form-grid">
              <label><span>{t("model")}</span><input bind:value={config.request.model_name} /></label>
              <label class:field-invalid={Boolean(fieldError("request.final_result_timeout_seconds"))}>
                <span>{t("finalTimeout")}</span>
                <input type="number" bind:value={config.request.final_result_timeout_seconds} />
                {#if fieldError("request.final_result_timeout_seconds")}<small class="field-error">{fieldError("request.final_result_timeout_seconds")}</small>{/if}
              </label>
            </div>
            {#if showAdvancedSettings}
            <div class="toggle-grid">
              <label class="check"><input type="checkbox" bind:checked={config.request.enable_nonstream} />{t("secondPass")}</label>
              <label class="check"><input type="checkbox" bind:checked={config.request.enable_itn} />{t("itn")}</label>
              <label class="check"><input type="checkbox" bind:checked={config.request.enable_punc} />{t("punctuation")}</label>
              <label class="check"><input type="checkbox" bind:checked={config.request.enable_ddc} />{t("ddc")}</label>
            </div>
            {/if}
          </div>
        </section>
        <section class="settings-group">
          <div class="settings-group-heading">
            <h3>{t("llmApiSettings")}</h3>
            <p>{t("llmApiSettingsDescription")}</p>
          </div>
          <div id="settings-llm-api" class="form-panel">
            <div class="section-heading with-actions">
              <div class="section-heading-copy"><h3>{t("llmPostEdit")}</h3><p>{t("llmDescription")}</p></div>
              <button class="test-button" onclick={testLlmConfig} disabled={testingLlm}>
                <ShieldCheck size={16} />{testingLlm ? t("testingConnection") : t("testConnection")}
              </button>
            </div>
            <label class="check"><input type="checkbox" bind:checked={config.llm_post_edit.enabled} />{t("enablePolishing")}</label>
            <div class="form-grid">
              <label class:field-invalid={Boolean(fieldError("llm_post_edit.base_url"))}>
                <span>Base URL</span>
                <input bind:value={config.llm_post_edit.base_url} />
                {#if fieldError("llm_post_edit.base_url")}<small class="field-error">{fieldError("llm_post_edit.base_url")}</small>{/if}
              </label>
              <label class:field-invalid={Boolean(fieldError("llm_post_edit.api_key"))}>
                <span>API Key</span>
                <input type="password" autocomplete="off" bind:value={config.llm_post_edit.api_key} />
                {#if fieldError("llm_post_edit.api_key")}<small class="field-error">{fieldError("llm_post_edit.api_key")}</small>{/if}
              </label>
              <label class:field-invalid={Boolean(fieldError("llm_post_edit.model"))}>
                <span>{t("model")}</span>
                <input bind:value={config.llm_post_edit.model} />
                {#if fieldError("llm_post_edit.model")}<small class="field-error">{fieldError("llm_post_edit.model")}</small>{/if}
              </label>
              <label class:field-invalid={Boolean(fieldError("llm_post_edit.timeout_seconds"))}>
                <span>{t("timeout")}</span>
                <input type="number" bind:value={config.llm_post_edit.timeout_seconds} />
                {#if fieldError("llm_post_edit.timeout_seconds")}<small class="field-error">{fieldError("llm_post_edit.timeout_seconds")}</small>{/if}
              </label>
            </div>
            {#if showAdvancedSettings}
            <div class="toggle-grid">
              <label class="check"><input type="checkbox" bind:checked={config.llm_post_edit.enable_thinking} />{t("enableThinking")}</label>
            </div>
            {/if}
          </div>
        </section>
      </section>
    {:else if selectedSection === "Options"}
      <section class="settings-stack">
        <SettingsToolbar
          title={t("settingsActionTitle")}
          hint={t("settingsActionHint")}
          statusMessage={settingsToolbarMessage()}
          reloadLabel={t("reload")}
          {saving}
          dirty={settingsDirty}
          onReload={reloadConfigFromUi}
        />
        <section class="settings-mode-panel">
          <div>
            <h3>{showAdvancedSettings ? t("advancedSettings") : t("basicSettings")}</h3>
            <p>{showAdvancedSettings ? t("advancedSettingsHint") : t("basicSettingsHint")}</p>
          </div>
          <button type="button" onclick={() => (showAdvancedSettings = !showAdvancedSettings)}>
            {showAdvancedSettings ? t("hideAdvancedSettings") : t("showAdvancedSettings")}
          </button>
        </section>
        <section class="settings-group">
          <div class="settings-group-heading">
            <h3>{t("optionsPageTitle")}</h3>
            <p>{t("optionsPageDescription")}</p>
          </div>
          <div id="settings-output" class="form-panel">
            <div class="section-heading"><h3>{t("startAndOutput")}</h3><p>{t("typingDescription")}</p></div>
            <div class="form-grid">
              <label class:field-invalid={Boolean(fieldError("hotkey") || hotkeyValidationMessage)}>
                <span>{t("hotkey")}</span>
                <button
                  type="button"
                  class:recording={hotkeyCaptureState === "recording"}
                  class="hotkey-recorder"
                  onkeydown={handleHotkeyKeydown}
                  onclick={beginHotkeyCapture}
                >
                  <Keyboard size={16} />
                  <strong>{hotkeyCaptureState === "recording" ? t("hotkeyRecording") : formatHotkey(config.hotkey) || t("hotkeyUnset")}</strong>
                </button>
                <small class="field-hint">{hotkeyValidationMessage || fieldError("hotkey") || t("hotkeyRecordHint")}</small>
              </label>
              <label class:field-invalid={Boolean(fieldError("typing.paste_delay_ms"))}>
                <span>{t("pasteDelayMs")}</span>
                <input type="number" bind:value={config.typing.paste_delay_ms} />
                {#if fieldError("typing.paste_delay_ms")}<small class="field-error">{fieldError("typing.paste_delay_ms")}</small>{/if}
              </label>
              <label class:field-invalid={Boolean(fieldError("typing.paste_method"))}>
                <span>{t("pasteMethod")}</span>
                <select bind:value={config.typing.paste_method}><option value="ctrl_v">Ctrl + V</option><option value="shift_insert">Shift + Insert</option><option value="clipboard_only">{t("clipboardOnly")}</option></select>
                {#if fieldError("typing.paste_method")}<small class="field-error">{fieldError("typing.paste_method")}</small>{/if}
              </label>
              <label class:field-invalid={Boolean(fieldError("tray.close_behavior"))}>
                <span>{t("closeBehavior")}</span>
                <select bind:value={config.tray.close_behavior}>
                  <option value="close_to_tray">{t("closeBehaviorCloseToTray")}</option>
                  <option value="direct_exit">{t("closeBehaviorDirectExit")}</option>
                  <option value="ask_every_time">{t("closeBehaviorAskEveryTime")}</option>
                </select>
                {#if fieldError("tray.close_behavior")}<small class="field-error">{fieldError("tray.close_behavior")}</small>{/if}
              </label>
              {#if showAdvancedSettings}
                <label class:field-invalid={Boolean(fieldError("typing.clipboard_restore_delay_ms"))}>
                  <span>{t("clipboardRestoreDelay")}</span>
                  <input type="number" bind:value={config.typing.clipboard_restore_delay_ms} />
                  {#if fieldError("typing.clipboard_restore_delay_ms")}<small class="field-error">{fieldError("typing.clipboard_restore_delay_ms")}</small>{/if}
                </label>
                <label class:field-invalid={Boolean(fieldError("typing.clipboard_snapshot_max_bytes"))}>
                  <span>{t("clipboardSnapshotMaxBytes")}</span>
                  <input type="number" bind:value={config.typing.clipboard_snapshot_max_bytes} />
                  {#if fieldError("typing.clipboard_snapshot_max_bytes")}<small class="field-error">{fieldError("typing.clipboard_snapshot_max_bytes")}</small>{/if}
                </label>
                <label><span>{t("clipboardRetryCount")}</span><input type="number" bind:value={config.typing.clipboard_open_retry_count} /></label>
                <label><span>{t("clipboardRetryInterval")}</span><input type="number" bind:value={config.typing.clipboard_open_retry_interval_ms} /></label>
              {/if}
            </div>
            <div class="toggle-grid">
              <label class="check"><input type="checkbox" bind:checked={config.triggers.hotkey_enabled} />{t("mainHotkey")}</label>
              <label class="check"><input type="checkbox" bind:checked={config.triggers.middle_mouse_enabled} onchange={(event) => maybeShowOptionEnabledNotice("middle_mouse_enabled", event.currentTarget.checked)} />{t("middleMouse")}</label>
              <label class="check"><input type="checkbox" bind:checked={config.triggers.right_alt_enabled} onchange={(event) => maybeShowOptionEnabledNotice("right_alt_enabled", event.currentTarget.checked)} />{t("rightAlt")}</label>
              {#if showAdvancedSettings}
                <label class="check"><input type="checkbox" bind:checked={config.typing.restore_clipboard_after_paste} />{t("restoreClipboardAfterPaste")}</label>
              {/if}
              <label class="check"><input type="checkbox" bind:checked={config.startup.launch_on_startup} />{t("launchOnStartup")}</label>
            </div>
            <p class="field-hint">{t("clipboardTextRestoreHint")}</p>
            {#if showAdvancedSettings}
              <p class="field-hint">{t("clipboardRestoreDelayHint")}</p>
              <p class="field-hint">{t("clipboardRetryHint")}</p>
            {/if}
            <p class="field-hint">{t("triggerConflictHint")}</p>
          </div>
          <div id="settings-overlay" class="form-panel">
            <div class="section-heading"><h3>{t("floatingCaptionAppearance")}</h3><p>{t("floatingCaptionAppearanceDescription")}</p></div>
            <div class="caption-theme-panel">
              <div class="caption-theme-head">
                <div>
                  <strong>{t("captionColors")}</strong>
                  <span>{t("captionColorsDescription")}</span>
                </div>
                <div class="caption-preview" style={`--preview-bg-rgb: ${overlayBackgroundRgb()}; --preview-opacity: ${overlayOpacity()}; --preview-text: ${overlayTextColor()};`}>
                  {t("captionPreviewText")}
                </div>
              </div>
              <div class="preset-row">
                {#each overlayColorPresets as preset}
                  <button
                    type="button"
                    class:active={overlayPresetActive(preset.background, preset.text)}
                    aria-pressed={overlayPresetActive(preset.background, preset.text)}
                    onclick={() => applyOverlayPreset(preset.background, preset.text)}
                  >
                    <span class="preset-swatch" style={`--preset-bg: ${preset.background}; --preset-text: ${preset.text};`}>Aa</span>
                    <span>{t(preset.label)}</span>
                  </button>
                {/each}
              </div>
              <div class="caption-opacity-row" class:field-invalid={Boolean(fieldError("ui.opacity"))}>
                <div>
                  <strong>{t("captionOpacity")}</strong>
                  <span>{t("captionOpacityDescription")}</span>
                </div>
                <div class="preset-row opacity-preset-row">
                  {#each overlayOpacityPresets as opacity}
                    <button
                      type="button"
                      class:active={overlayOpacityPresetActive(opacity)}
                      aria-pressed={overlayOpacityPresetActive(opacity)}
                      onclick={() => applyOverlayOpacity(opacity)}
                    >
                      {overlayOpacityLabel(opacity)}
                    </button>
                  {/each}
                </div>
                {#if fieldError("ui.opacity")}<small class="field-error">{fieldError("ui.opacity")}</small>{/if}
              </div>
              <div class="form-grid color-grid">
                <label class="color-field" class:field-invalid={Boolean(fieldError("ui.background_color"))}>
                  <span>{t("captionBackgroundColor")}</span>
                  <input type="color" value={overlayBackgroundColor()} oninput={(event) => (config.ui.background_color = event.currentTarget.value)} />
                  {#if fieldError("ui.background_color")}<small class="field-error">{fieldError("ui.background_color")}</small>{/if}
                </label>
                <label class="color-field" class:field-invalid={Boolean(fieldError("ui.text_color"))}>
                  <span>{t("captionTextColor")}</span>
                  <input type="color" value={overlayTextColor()} oninput={(event) => (config.ui.text_color = event.currentTarget.value)} />
                  {#if fieldError("ui.text_color")}<small class="field-error">{fieldError("ui.text_color")}</small>{/if}
                </label>
              </div>
            </div>
            {#if showAdvancedSettings}
              <div class="form-grid">
                <label class:field-invalid={Boolean(fieldError("ui.width"))}>
                  <span>{t("width")}</span>
                  <input type="number" bind:value={config.ui.width} />
                  {#if fieldError("ui.width")}<small class="field-error">{fieldError("ui.width")}</small>{/if}
                </label>
                <label class:field-invalid={Boolean(fieldError("ui.height"))}>
                  <span>{t("height")}</span>
                  <input type="number" bind:value={config.ui.height} />
                  {#if fieldError("ui.height")}<small class="field-error">{fieldError("ui.height")}</small>{/if}
                </label>
                <label><span>{t("marginBottom")}</span><input type="number" bind:value={config.ui.margin_bottom} /></label>
                <label><span>{t("scrollInterval")}</span><input type="number" bind:value={config.ui.scroll_interval_ms} /></label>
                <label><span>{t("startupTimeout")}</span><input type="number" bind:value={config.tray.startup_message_timeout_ms} /></label>
              </div>
              <div class="toggle-grid">
                <label class="check"><input type="checkbox" bind:checked={config.tray.show_startup_message} />{t("showStartupMessage")}</label>
              </div>
              <p class="field-hint">{t("closeBehaviorHint")}</p>
            {/if}
          </div>
          <div id="settings-audio" class="form-panel">
            <div class="section-heading"><h3>{t("recordingParams")}</h3><p>{t("audioDescription")}</p></div>
            <div class="form-grid">
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
              {#if showAdvancedSettings}
                <label class:field-invalid={Boolean(fieldError("audio.sample_rate"))}>
                  <span>{t("sampleRate")}</span>
                  <input type="number" bind:value={config.audio.sample_rate} />
                  {#if fieldError("audio.sample_rate")}<small class="field-error">{fieldError("audio.sample_rate")}</small>{/if}
                </label>
                <label class:field-invalid={Boolean(fieldError("audio.channels"))}>
                  <span>{t("channels")}</span>
                  <input type="number" bind:value={config.audio.channels} />
                  {#if fieldError("audio.channels")}<small class="field-error">{fieldError("audio.channels")}</small>{/if}
                </label>
                <label class:field-invalid={Boolean(fieldError("audio.segment_ms"))}>
                  <span>{t("segmentMs")}</span>
                  <input type="number" bind:value={config.audio.segment_ms} />
                  {#if fieldError("audio.segment_ms")}<small class="field-error">{fieldError("audio.segment_ms")}</small>{/if}
                </label>
                <label class:field-invalid={Boolean(fieldError("audio.max_record_seconds"))}>
                  <span>{t("maxSeconds")}</span>
                  <input type="number" bind:value={config.audio.max_record_seconds} />
                  {#if fieldError("audio.max_record_seconds")}<small class="field-error">{fieldError("audio.max_record_seconds")}</small>{/if}
                </label>
                <label class:field-invalid={Boolean(fieldError("audio.stop_grace_ms"))}>
                  <span>{t("stopGraceMs")}</span>
                  <input type="number" bind:value={config.audio.stop_grace_ms} />
                  {#if fieldError("audio.stop_grace_ms")}<small class="field-error">{fieldError("audio.stop_grace_ms")}</small>{/if}
                </label>
              {/if}
            </div>
            {#if showAdvancedSettings}
            <div class="toggle-grid">
              <label class="check"><input type="checkbox" bind:checked={config.audio.mute_system_volume_while_recording} onchange={(event) => maybeShowOptionEnabledNotice("mute_system_volume_while_recording", event.currentTarget.checked)} />{t("muteSystemAudio")}</label>
            </div>
            <p class="field-hint">{t("muteSystemAudioHint")}</p>
            {/if}
          </div>
          <div id="settings-update" class="form-panel update-panel">
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
            {#if showAdvancedSettings}
              <label class:field-invalid={Boolean(fieldError("update.github_repo"))}>
                <span>GitHub Release Repo</span>
                <input bind:value={config.update.github_repo} />
                {#if fieldError("update.github_repo")}<small class="field-error">{fieldError("update.github_repo")}</small>{/if}
              </label>
            {/if}
          </div>
          {#if showAdvancedSettings}
          <div id="settings-diagnostics" class="form-panel">
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
                <button onclick={copyDiagnosticReport} disabled={copyingDiagnosticReport}>
                  <ClipboardCopy size={16} />{copyingDiagnosticReport ? t("copyingReport") : t("copyDiagnosticReport")}
                </button>
              </div>
            </div>
          </div>
          {/if}
        </section>
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
  <div
    class:error={actionNoticeKind === "error"}
    class:info={actionNoticeKind === "info"}
    class:warning={actionNoticeKind === "warning"}
    class="action-notice"
    role="status"
    aria-live="polite"
  >
    {#if actionNoticeKind === "success"}
      <Check size={16} />
    {:else if actionNoticeKind === "info"}
      <Info size={16} />
    {:else}
      <AlertCircle size={16} />
    {/if}
    <span>{actionNotice}</span>
  </div>
{/if}
{#if closePromptVisible}
  <div class="modal-backdrop" role="presentation">
    <div class="close-prompt" role="dialog" aria-modal="true" aria-labelledby="close-prompt-title">
      <div>
        <h3 id="close-prompt-title">{t("closePromptTitle")}</h3>
        <p>{t("closePromptBody")}</p>
      </div>
      <div class="close-prompt-actions">
        <button class="primary" onclick={() => hideWindowToTray(closePromptFirstTime && closePromptBehavior === "close_to_tray")}>
          {t("closePromptGotIt")}
        </button>
        <button onclick={closeWindowWithoutFuturePrompt}>{t("closePromptDontShowAgain")}</button>
        <button class="danger" onclick={exitFromClosePrompt}>{t("closePromptExit")}</button>
      </div>
    </div>
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
    background: transparent;
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
    background: transparent;
  }
  .overlay-caption {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: flex-start;
    width: 100%;
    height: 100%;
    min-width: 0;
    padding: 8px 16px;
    overflow: hidden;
    color: var(--overlay-text, #ffffff);
    background: rgba(var(--overlay-bg-rgb, 23, 110, 230), var(--overlay-opacity, 0.9));
    border: 0;
    border-radius: 0;
    box-shadow: none;
    text-align: left;
  }
  .overlay-voice-meter {
    position: absolute;
    top: 50%;
    left: 13px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 2px;
    width: 28px;
    height: 22px;
    color: var(--overlay-text, #ffffff);
    background: color-mix(in srgb, currentColor 11%, transparent);
    border: 1px solid color-mix(in srgb, currentColor 20%, transparent);
    border-radius: 999px;
    opacity: 0.72;
    pointer-events: none;
    transform: translateY(-50%);
    transition: opacity 140ms ease, background 140ms ease, border-color 140ms ease;
  }
  .overlay-voice-meter.active {
    opacity: 0.88;
  }
  .overlay-voice-meter i {
    width: 2px;
    min-height: 4px;
    background: currentColor;
    border-radius: 999px;
    transition: height 120ms ease, opacity 120ms ease;
  }
  .overlay-caption-text {
    display: grid;
    align-content: start;
    flex: 1 1 auto;
    min-width: 0;
    height: 100%;
    max-height: 100%;
    padding: 0 36px;
    overflow: hidden;
    color: inherit;
    box-sizing: border-box;
    font-weight: 400;
    line-height: 1.18;
    text-shadow: none;
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
  .mic-orb:disabled {
    cursor: not-allowed;
    opacity: 0.72;
    transform: none;
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
  .voice-hero.locked {
    background: linear-gradient(135deg, #475569 0%, #2563eb 100%);
    box-shadow: 0 14px 28px rgba(71, 85, 105, 0.18);
  }
  .voice-hero.locked .mic-ring {
    color: #64748b;
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
  .shortcut-help span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
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
    gap: 18px;
    max-width: 1040px;
  }
  .settings-mode-panel {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    padding: 14px 16px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: #f8fbff;
  }
  .settings-mode-panel h3 {
    margin: 0;
    color: var(--text-main);
    font-size: 16px;
    font-weight: 800;
  }
  .settings-mode-panel p {
    margin: 4px 0 0;
    color: var(--text-secondary);
    font-size: 13px;
    line-height: 1.4;
  }
  .settings-mode-panel button {
    flex: 0 0 auto;
    min-height: 34px;
    padding: 0 12px;
    border: 1px solid #b8d4f4;
    border-radius: 8px;
    background: #ffffff;
    color: #1f66b1;
    font-weight: 800;
    cursor: pointer;
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
  .form-panel[id^="settings-"] {
    scroll-margin-top: 86px;
  }
  .form-panel label {
    display: grid;
    align-content: start;
    gap: 8px;
    color: var(--text-secondary);
    font-size: 14px;
  }
  .form-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    align-items: start;
    gap: 16px 14px;
  }
  .toggle-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    align-items: start;
    gap: 10px;
  }
  .color-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
  .caption-theme-panel {
    display: grid;
    gap: 12px;
    padding: 14px;
    background: #f8fbff;
    border: 1px solid var(--border);
    border-radius: 14px;
  }
  .caption-theme-head {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 12px;
  }
  .caption-theme-head > div:first-child {
    display: grid;
    gap: 4px;
    min-width: 0;
  }
  .caption-theme-head strong {
    color: var(--text-main);
    font-size: 14px;
    font-weight: 800;
  }
  .caption-theme-head span {
    color: var(--text-secondary);
    font-size: 13px;
    line-height: 1.45;
  }
  .caption-preview {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 168px;
    min-height: 36px;
    padding: 0 14px;
    overflow: hidden;
    color: var(--preview-text);
    background: rgba(var(--preview-bg-rgb), var(--preview-opacity));
    border-radius: 10px;
    font-size: 14px;
    font-weight: 700;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .preset-row {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 10px;
  }
  .preset-row button {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    min-height: 42px;
    padding: 6px 10px;
    color: var(--text-main);
    background: #ffffff;
    border: 1px solid var(--border);
    border-radius: 10px;
    font-size: 13px;
    font-weight: 700;
  }
  .preset-row button.active {
    border-color: var(--primary);
    box-shadow: 0 0 0 2px rgba(47, 128, 237, 0.12);
  }
  .preset-row button > span:last-child {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .preset-swatch {
    display: inline-flex;
    flex: 0 0 auto;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 26px;
    color: var(--preset-text);
    background: var(--preset-bg);
    border-radius: 8px;
    font-size: 12px;
    font-weight: 800;
  }
  .caption-opacity-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(220px, 0.8fr);
    align-items: center;
    gap: 12px;
  }
  .caption-opacity-row > div:first-child {
    display: grid;
    gap: 4px;
    min-width: 0;
  }
  .caption-opacity-row strong {
    color: var(--text-main);
    font-size: 14px;
    font-weight: 800;
  }
  .caption-opacity-row span {
    color: var(--text-secondary);
    font-size: 13px;
    line-height: 1.45;
  }
  .opacity-preset-row {
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }
  .opacity-preset-row button {
    justify-content: center;
    padding: 6px 8px;
  }
  .color-field input[type="color"] {
    height: 38px;
    padding: 4px;
    cursor: pointer;
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
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .section-heading.with-actions > div {
    min-width: 0;
  }
  .section-heading-copy {
    flex: 1 1 320px;
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
    min-width: 96px;
    min-height: 36px;
    padding: 0 14px;
    color: var(--primary);
    background: var(--primary-light);
    border: 1px solid rgba(47, 128, 237, 0.18);
    border-radius: 12px;
    font-size: 13px;
    font-weight: 800;
    white-space: nowrap;
  }
  .settings-inline-actions {
    display: flex;
    flex: 0 0 auto;
    flex-wrap: wrap;
    gap: 10px;
    justify-content: flex-end;
    min-width: 0;
  }
  .settings-inline-actions .test-button {
    min-width: 96px;
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
  .hotkey-recorder {
    display: inline-flex;
    align-items: center;
    justify-content: flex-start;
    gap: 9px;
    width: 100%;
    min-height: 38px;
    padding: 0 12px;
    color: var(--text-main);
    background: #ffffff;
    border: 1px solid var(--border);
    border-radius: 10px;
    text-align: left;
  }
  .hotkey-recorder strong {
    min-width: 0;
    overflow: hidden;
    color: inherit;
    font-size: 14px;
    font-weight: 800;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .hotkey-recorder :global(svg) {
    flex: 0 0 auto;
    color: var(--primary);
  }
  .hotkey-recorder.recording {
    border-color: var(--primary);
    background: var(--primary-light);
    box-shadow: 0 0 0 3px rgba(47, 128, 237, 0.14);
  }
  .field-invalid .hotkey-recorder {
    border-color: var(--danger);
    background: #fff7f7;
  }
  .field-hint {
    margin: 8px 0 0;
    color: var(--text-muted);
    font-size: 12px;
    line-height: 1.45;
  }
  .field-invalid input {
    border-color: var(--danger);
    background: #fff7f7;
  }
  .field-error {
    color: var(--danger);
    font-size: 12px;
    line-height: 1.35;
  }
  input:focus,
  textarea:focus,
  select:focus,
  button:focus-visible {
    border-color: var(--primary);
    box-shadow: 0 0 0 3px rgba(47, 128, 237, 0.14);
  }
  .update-card {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    padding: 14px;
    background: #f8fbff;
    border: 1px solid var(--border);
    border-radius: 12px;
  }
  .update-card > div:first-child {
    min-width: 0;
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
    min-width: 0;
  }
  .update-actions button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    min-height: 36px;
    min-width: 118px;
    padding: 0 12px;
    color: var(--text-main);
    background: #ffffff;
    border: 1px solid var(--border);
    border-radius: 10px;
    font-weight: 700;
    white-space: nowrap;
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
  .auto-hotword-status {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
    padding: 10px 12px;
    color: var(--text-secondary);
    background: #f8fbff;
    border: 1px solid var(--border);
    border-radius: 8px;
    font-size: 13px;
    line-height: 1.4;
  }
  .auto-hotword-status :global(svg) {
    flex: 0 0 auto;
    color: var(--primary);
  }
  .auto-hotword-status span {
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .auto-hotword-candidates {
    display: grid;
    gap: 10px;
  }
  .candidate-list-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 10px;
  }
  .candidate-list-head strong {
    color: var(--text-main);
    font-size: 14px;
  }
  .candidate-row {
    display: grid !important;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: start;
    gap: 10px;
    padding: 12px;
    background: #ffffff;
    border: 1px solid var(--border);
    border-radius: 8px;
    cursor: pointer;
  }
  .candidate-row:hover {
    border-color: rgba(47, 128, 237, 0.4);
    background: #f8fbff;
  }
  .candidate-row input {
    width: 16px;
    height: 16px;
    margin-top: 2px;
  }
  .candidate-copy {
    display: grid;
    gap: 4px;
    min-width: 0;
  }
  .candidate-copy strong {
    color: var(--text-main);
    font-size: 14px;
    overflow-wrap: anywhere;
  }
  .candidate-copy small {
    color: var(--text-muted);
    font-size: 12px;
    line-height: 1.35;
    overflow-wrap: anywhere;
  }
  .candidate-copy span {
    color: var(--text-secondary);
    font-size: 13px;
    line-height: 1.4;
    overflow-wrap: anywhere;
  }
  .action-notice {
    position: fixed;
    right: 22px;
    bottom: 20px;
    z-index: 20;
    display: inline-flex;
    align-items: flex-start;
    max-width: min(460px, calc(100vw - 44px));
    min-height: 40px;
    gap: 8px;
    padding: 10px 14px;
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
    line-height: 1.4;
    overflow-wrap: anywhere;
    white-space: normal;
  }
  .action-notice.info {
    color: #245b93;
    background: rgba(240, 247, 255, 0.98);
    border-color: rgba(47, 128, 237, 0.24);
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
  .modal-backdrop {
    position: fixed;
    inset: 0;
    z-index: 30;
    display: grid;
    place-items: center;
    padding: 24px;
    background: rgba(15, 23, 42, 0.26);
  }
  .close-prompt {
    display: grid;
    gap: 18px;
    width: min(420px, 100%);
    padding: 20px;
    background: #ffffff;
    border: 1px solid var(--border);
    border-radius: 16px;
    box-shadow: 0 20px 60px rgba(15, 23, 42, 0.2);
  }
  .close-prompt h3 {
    margin: 0 0 8px;
    color: var(--text-main);
    font-size: 18px;
    font-weight: 800;
  }
  .close-prompt p {
    margin: 0;
    color: var(--text-secondary);
    font-size: 14px;
    line-height: 1.55;
  }
  .close-prompt-actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 10px;
  }
  .close-prompt-actions button {
    min-height: 36px;
    padding: 0 13px;
    color: var(--text-main);
    background: #ffffff;
    border: 1px solid var(--border);
    border-radius: 10px;
    font-weight: 700;
  }
  .close-prompt-actions .primary {
    color: #ffffff;
    background: var(--primary);
    border-color: var(--primary);
  }
  .close-prompt-actions .danger {
    color: #b91c1c;
    background: #fff5f5;
    border-color: rgba(239, 68, 68, 0.26);
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
  .setup-alert,
  .auth-gate-card {
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
  .auth-gate-card {
    margin-bottom: 0;
  }
  .setup-alert strong,
  .auth-gate-card strong {
    color: var(--text-main);
  }
  .setup-alert p,
  .auth-gate-card p {
    margin: 4px 0 0;
    color: var(--text-secondary);
    font-size: 14px;
  }
  .error-help-card {
    display: grid;
    gap: 6px;
    margin-bottom: 16px;
    padding: 14px 16px;
    color: #991b1b;
    background: #fff5f5;
    border: 1px solid rgba(239, 68, 68, 0.24);
    border-radius: 14px;
  }
  .error-help-card strong {
    color: #7f1d1d;
    font-size: 15px;
  }
  .error-help-card p {
    margin: 0;
    color: #7f1d1d;
    font-size: 13px;
    line-height: 1.45;
  }
  .error-help-card span {
    font-weight: 800;
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
    .section-heading.with-actions {
      display: grid;
      grid-template-columns: 1fr;
      align-items: stretch;
    }
    .test-button {
      width: 100%;
    }
    .update-card {
      grid-template-columns: 1fr;
      align-items: stretch;
    }
    .update-actions {
      justify-content: stretch;
    }
    .update-actions button {
      flex: 1 1 150px;
    }
    .trigger-grid,
    .stats-row,
    .ui-compact .trigger-grid,
    .ui-compact .stats-row {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
    .form-grid,
    .toggle-grid,
    .preset-row {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
    .caption-theme-head {
      grid-template-columns: 1fr;
      align-items: stretch;
    }
    .caption-preview {
      width: 100%;
      min-width: 0;
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
  @media (max-width: 720px) {
    .form-grid,
    .toggle-grid,
    .preset-row,
    .color-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
