<script lang="ts">
  import { onMount } from "svelte";
  import { browser } from "$app/environment";
  import type {
    SetupStatusItem,
  } from "$lib/components/overview/SetupStatusCard.svelte";
  import AppContent from "$lib/components/app/AppContent.svelte";
  import AppGlobalStyles from "$lib/components/app/AppGlobalStyles.svelte";
  import AppShell from "$lib/components/app/AppShell.svelte";
  import ActionNotice from "$lib/components/common/ActionNotice.svelte";
  import CloseToTrayDialog from "$lib/components/common/CloseToTrayDialog.svelte";
  import OverlayWindow from "$lib/components/overlay/OverlayWindow.svelte";
  import StartupToast from "$lib/components/overlay/StartupToast.svelte";
  import type { HistoryDayRow, HistorySummaryCard } from "$lib/components/pages/HistorySection.svelte";
  import {
    autoSaveDelayMs,
    chineseTypingCharsPerMinute,
    defaultOverlayText,
    emptyStats,
    emptyUsage,
    fallbackConfig,
    fallbackSnapshot,
    micBars,
    overlayColorPresets,
    overlayMeterBars,
    overlayOpacityPresets,
    setupStatusCacheKey,
  } from "$lib/app/defaults";
  import { copy, type CopyKey, type Language, type UserErrorDetail } from "$lib/i18n";
  import {
    applySelectedAutoHotwords as mergeSelectedAutoHotwords,
    autoHotwordStatusText as formatAutoHotwordStatusText,
    buildFinalPromptPreview,
    mapGeneratedHotwordCandidates,
    showAutoHotwordDetails as shouldShowAutoHotwordDetails,
  } from "$lib/utils/autoHotwords";
  import {
    configSetupMessage as getConfigSetupMessage,
    hasAuth as configHasAuth,
    hasLlmApiConfig as configHasLlmApiConfig,
    isConfigError,
    isErrorStatus as isUserErrorStatus,
    requiresAsrAuth as configRequiresAsrAuth,
    sectionForSettingsPanel as getSectionForSettingsPanel,
    settingsPanelForError,
    shouldOpenSettingsForError,
    userErrorDetail as getUserErrorDetail,
    userErrorMessage as getUserErrorMessage,
  } from "$lib/utils/appRouting";
  import { clonePlain, configFingerprint, firstValidationField, validationErrorMap } from "$lib/utils/config";
  import {
    acceptedAutoHotwordCount as countAcceptedAutoHotwords,
    candidateConfidenceLabel,
    dedupeHotwords,
    effectiveHotwords as mergedEffectiveHotwords,
    hotwordCount as countManualHotwords,
    normalizeHotwords,
  } from "$lib/utils/hotwords";
  import { formatHotkey, hotkeyFromKeyboardEvent, validateHotkeyText } from "$lib/utils/hotkeys";
  import {
    overlayBackgroundColor as getOverlayBackgroundColor,
    overlayBackgroundRgb as getOverlayBackgroundRgb,
    overlayOpacity as getOverlayOpacity,
    overlayOpacityLabel,
    overlayOpacityPresetActive as isOverlayOpacityPresetActive,
    overlayPresetActive as isOverlayPresetActive,
    overlayTextColor as getOverlayTextColor,
  } from "$lib/utils/overlayAppearance";
  import {
    normalizeOverlayText,
    overlayAvailableTextHeight as getOverlayAvailableTextHeight,
    overlayVisibleLineCount as getOverlayVisibleLineCount,
    resolveOverlayLayout as getOverlayLayout,
    wrapOverlayText as wrapOverlayTextLines,
  } from "$lib/utils/overlayLayout";
  import {
    formatHours,
    formatNumber as formatNumberForLanguage,
    formatSavedHours as formatSavedHoursForLanguage,
    historySummaryCards as buildHistorySummaryCards,
    recentSevenDayDisplayRows as buildRecentSevenDayDisplayRows,
    weeklySavedHours as weeklySavedHoursForStats,
  } from "$lib/utils/stats";
  import {
    asrConfigFingerprint as buildAsrConfigFingerprint,
    asrConnectionStatusOk as isAsrConnectionStatusOk,
    asrConnectionStatusText as getAsrConnectionStatusText,
    buildLocalSetupStatus,
    buildSetupStatusItems,
    currentAsrConnectionStatus as getCurrentAsrConnectionStatus,
    formatEnabledTriggers as getEnabledTriggersText,
    mergeSetupStatusFromConfig,
    pasteMethodLabel as getPasteMethodLabel,
    readCachedSetupStatus,
    setupActionText as getSetupActionText,
    type SetupStatus,
  } from "$lib/utils/setupStatus";
  import {
    fieldAdvancedSection,
    fieldRequiresAdvancedSettings,
    settingsPanelForField,
    type AdvancedSection,
  } from "$lib/utils/settingsFields";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import type {
    AppConfig,
    AppSnapshot,
    AsrConnectionStatus,
    AsrFinalText,
    AudioDeviceInfo,
    AudioLevel,
    AutoHotwordStatus,
    CloseToTrayRequest,
    ConfigSaveError,
    ConfigValidationError,
    ConnectionTestResult,
    DiagnosticReport,
    HotkeyCaptureState,
    HotwordGenerationResult,
    InstallUpdateResult,
    LoadedConfig,
    OverlayConfig,
    OverlayMode,
    OverlayText,
    PersistConfigOptions,
    Section,
    SelectableHotwordCandidate,
    SessionPhase,
    SessionState,
    SoftConfigNoticeKey,
    StatsSnapshot,
    TriggerKey,
    UpdateStatus,
    UsageStats,
  } from "$lib/types/app";

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
  let setupStatus = $state<SetupStatus | null>(readCachedSetupStatus(browser));
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
  let advancedVisible = $state<Record<AdvancedSection, boolean>>({
    Hotwords: false,
    ApiConfig: false,
    Options: false,
  });
  let llmApiConfigVisible = $state(false);
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
        const visibleWarning = event.payload.warning && !isQuietAsrWarningCode(event.payload.warning_code)
          ? event.payload.warning
          : null;
        if (visibleWarning) {
          showActionNotice(visibleWarning, "warning");
        }
        statusMessage = visibleWarning ?? t("sessionSucceeded");
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
    const normalized = normalizeOverlayText(rawText) || defaultOverlayText;
    if (!force && normalized === overlayText) return;
    overlayText = normalized;

    if (normalized === defaultOverlayText) {
      overlaySmallLayoutLocked = false;
    }

    const availableHeight = overlayAvailableTextHeight();
    const singleFontSize = 20;
    const singleLineCount = wrapOverlayText(normalized, singleFontSize).length;
    const { mode, fontSize, lineLimit } = getOverlayLayout(
      normalized,
      overlaySmallLayoutLocked,
      availableHeight,
      singleLineCount,
    );
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

  function wrapOverlayText(text: string, fontSize: number) {
    return wrapOverlayTextLines(text, fontSize, overlayTextContentWidth(), measureOverlayText);
  }

  function overlayTextContentWidth() {
    if (!overlayTextElement) {
      return Math.max(80, window.innerWidth - 88);
    }
    const styles = window.getComputedStyle(overlayTextElement);
    const paddingLeft = Number.parseFloat(styles.paddingLeft) || 0;
    const paddingRight = Number.parseFloat(styles.paddingRight) || 0;
    return Math.max(80, overlayTextElement.clientWidth - paddingLeft - paddingRight);
  }

  function measureOverlayText(text: string, fontSize: number) {
    measureCanvas ??= document.createElement("canvas");
    const context = measureCanvas.getContext("2d");
    if (!context) return Array.from(text).length * fontSize;
    context.font = `400 ${fontSize}px "Microsoft YaHei", "Segoe UI", "PingFang SC", sans-serif`;
    return context.measureText(text).width;
  }

  function overlayVisibleLineCount() {
    return getOverlayVisibleLineCount(overlayLineLimit);
  }

  function overlayAvailableTextHeight() {
    return getOverlayAvailableTextHeight(window.innerHeight);
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
    return buildLocalSetupStatus(configValue, devices, setupStatus?.warnings ?? []);
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
      const hotkeyError = validateHotkeyText(configToSave.hotkey, {
        required: t("hotkeyRequired"),
        needsModifier: t("hotkeyNeedsModifier"),
        unsupported: t("hotkeyUnsupported"),
      });
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
  function fieldError(field: string) {
    return validationErrors[field] ?? "";
  }
  function isAdvancedVisible(section: AdvancedSection) {
    return advancedVisible[section];
  }
  function toggleAdvanced(section: AdvancedSection) {
    advancedVisible = {
      ...advancedVisible,
      [section]: !advancedVisible[section],
    };
  }
  function showAdvanced(section: AdvancedSection) {
    advancedVisible = {
      ...advancedVisible,
      [section]: true,
    };
  }
  function focusFirstValidationError(errors: ConfigValidationError[]) {
    const field = firstValidationField(errors);
    if (!field) return;
    if (fieldRequiresAdvancedSettings(field)) showAdvanced(fieldAdvancedSection(field));
    if (field.startsWith("llm_post_edit.")) llmApiConfigVisible = true;
    scrollToSettingsPanel(settingsPanelForField(field));
  }
  function syncSetupStatusFromConfig(nextConfig: AppConfig) {
    const currentStatus = setupStatus ?? localSetupStatusFromConfig(nextConfig);
    applySetupStatus(mergeSetupStatusFromConfig(nextConfig, currentStatus));
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

  function isQuietAsrWarningCode(code: string | null | undefined) {
    return code === "CLIPBOARD_PARTIAL_RESTORE";
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
    const localStatus = localSetupStatusFromConfig(config);
    return buildSetupStatusItems({
      loading: setupStatusLoading,
      configLoaded,
      config,
      setupStatus,
      localStatus,
      audioDevices,
      asrStatus: currentAsrConnectionStatus(setupStatus ?? localStatus),
      triggerText: formatEnabledTriggers(),
      t,
    });
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
    return getSetupActionText(action, t);
  }
  function handleSetupAction(action: string) {
    if (action === "audio") void refreshSetupStatus();
    if (action === "privacy") showAdvanced("Hotwords");
    const targetId =
      action === "asr_auth"
        ? "settings-auth"
        : action === "audio"
          ? "settings-audio"
          : action === "typing"
            ? "settings-output"
            : action === "privacy"
              ? "settings-prompt-context"
              : "settings-output";
    scrollToSettingsPanel(targetId);
  }
  function pasteMethodLabel(value: string) {
    return getPasteMethodLabel(value, t);
  }
  function asrConfigFingerprint(configValue = config) {
    return buildAsrConfigFingerprint(configValue);
  }
  function currentAsrConnectionStatus(status: SetupStatus | null = null): AsrConnectionStatus {
    return getCurrentAsrConnectionStatus({
      status,
      authReady: hasAuth(),
      testingAsr,
      currentFingerprint: asrConfigFingerprint(),
      testedFingerprint: asrTestedConfigFingerprint,
      asrConnectionStatus,
    });
  }
  function asrConnectionStatusText(status: AsrConnectionStatus) {
    return getAsrConnectionStatusText(status, t);
  }
  function asrConnectionStatusOk(status: AsrConnectionStatus) {
    return isAsrConnectionStatusOk(status);
  }
  function formatEnabledTriggers() {
    return getEnabledTriggersText(config, snapshot.hotkey, t, formatHotkey);
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

  function setHotkey(value: string) {
    const formatted = formatHotkey(value);
    config.hotkey = formatted;
    hotkeyValidationMessage = validateHotkeyText(formatted, {
      required: t("hotkeyRequired"),
      needsModifier: t("hotkeyNeedsModifier"),
      unsupported: t("hotkeyUnsupported"),
    });
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
    hotkeyValidationMessage = validateHotkeyText(captured, {
      required: t("hotkeyRequired"),
      needsModifier: t("hotkeyNeedsModifier"),
      unsupported: t("hotkeyUnsupported"),
    });
    validationErrors = hotkeyValidationMessage
      ? { ...validationErrors, hotkey: hotkeyValidationMessage }
      : Object.fromEntries(Object.entries(validationErrors).filter(([field]) => field !== "hotkey"));
    if (!hotkeyValidationMessage) hotkeyCaptureState = "idle";
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

  function updateAcceptedAutoHotwords(value: string) {
    config.auto_hotwords.accepted_hotwords = normalizeHotwords(value);
  }

  function effectiveHotwords() {
    return mergedEffectiveHotwords(config);
  }

  function hotwordCount() {
    return countManualHotwords(config);
  }

  function acceptedAutoHotwordCount() {
    return countAcceptedAutoHotwords(config);
  }

  function tidyHotwords() {
    config.context.hotwords = dedupeHotwords(config.context.hotwords);
    showActionNotice(t("hotwordsTidied"), "success");
  }

  function tidyAcceptedAutoHotwords() {
    config.auto_hotwords.accepted_hotwords = dedupeHotwords(config.auto_hotwords.accepted_hotwords);
    showActionNotice(t("autoHotwordsAcceptedTidied"), "success");
  }

  function clearHotwords() {
    if (!browser || window.confirm(t("clearHotwordsConfirm"))) {
      config.context.hotwords = [];
      showActionNotice(t("hotwordsCleared"), "success");
    }
  }

  function clearAcceptedAutoHotwords() {
    if (!browser || window.confirm(t("autoHotwordsAcceptedClearConfirm"))) {
      config.auto_hotwords.accepted_hotwords = [];
      showActionNotice(t("autoHotwordsAcceptedCleared"), "success");
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
    window.alert(
      buildFinalPromptPreview(config, sampleText, effectiveHotwords(), {
        dictionary: t("promptPreviewUserDictionary"),
        context: t("promptPreviewContextTitle"),
        systemPrompt: t("systemPrompt"),
        userPromptTemplate: t("userPromptTemplate"),
        empty: t("promptPreviewEmpty"),
      }),
    );
  }

  async function refreshAutoHotwordStatus() {
    const result = await safeInvoke<AutoHotwordStatus>("get_auto_hotword_status", undefined, true);
    if (result) autoHotwordStatus = result;
  }

  function autoHotwordStatusText() {
    return formatAutoHotwordStatusText(autoHotwordStatus, t);
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
      autoHotwordCandidates = mapGeneratedHotwordCandidates(result.candidates);
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
    if (selectedAutoHotwordCount() === 0) {
      showActionNotice(t("autoHotwordsNoSelection"), "warning");
      return;
    }

    const { added, acceptedHotwords } = mergeSelectedAutoHotwords(config, autoHotwordCandidates, effectiveHotwords());
    config.auto_hotwords.accepted_hotwords = acceptedHotwords;
    const message = t("autoHotwordsApplied", { count: String(added) });
    statusMessage = message;
    showActionNotice(message, added > 0 ? "success" : "warning");
  }

  function overlayBackgroundColor() {
    return getOverlayBackgroundColor(config.ui);
  }

  function overlayTextColor() {
    return getOverlayTextColor(config.ui);
  }

  function overlayBackgroundRgb() {
    return getOverlayBackgroundRgb(overlayBackgroundColor());
  }

  function overlayOpacity() {
    return getOverlayOpacity(config.ui);
  }

  function applyOverlayOpacity(value: number) {
    config.ui.opacity = value;
  }

  function overlayOpacityPresetActive(value: number) {
    return isOverlayOpacityPresetActive(overlayOpacity(), value);
  }

  function applyOverlayPreset(background: string, text: string) {
    config.ui.background_color = background;
    config.ui.text_color = text;
  }

  function overlayPresetActive(background: string, text: string) {
    return isOverlayPresetActive(overlayBackgroundColor(), overlayTextColor(), background, text);
  }

  function setInputDevice(value: string | number | null) {
    if (value === null || value === "") {
      config.audio.input_device = null;
      return;
    }
    config.audio.input_device = Number(value);
  }

  function formatNumber(value: number) {
    return formatNumberForLanguage(value, language);
  }
  function inputStatus(): "idle" | "listening" | "error" {
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
    return weeklySavedHoursForStats(stats, chineseTypingCharsPerMinute);
  }
  function formatSavedHours(hours: number) {
    return formatSavedHoursForLanguage(hours, language);
  }
  function historySummaryCards(): HistorySummaryCard[] {
    return buildHistorySummaryCards(stats, t, language, chineseTypingCharsPerMinute);
  }
  function recentSevenDayDisplayRows(): HistoryDayRow[] {
    return buildRecentSevenDayDisplayRows(stats, t, language, chineseTypingCharsPerMinute, emptyUsage);
  }
  function hasAuth(configValue = config) {
    return configHasAuth(configValue);
  }
  function hasLlmApiConfig(configValue = config) {
    return configHasLlmApiConfig(configValue);
  }
  function showAutoHotwordDetails() {
    return shouldShowAutoHotwordDetails(config, fieldError);
  }
  function openLlmApiSettings() {
    llmApiConfigVisible = true;
    scrollToSettingsPanel("settings-llm-api");
  }
  function llmApiStatusText() {
    return hasLlmApiConfig() ? t("llmApiConfigured") : t("llmApiMissing");
  }
  function requiresAsrAuth(configValue?: AppConfig, exists?: boolean) {
    return configRequiresAsrAuth({
      configLoaded,
      setupStatus,
      config,
      configExists,
      targetConfig: configValue,
      targetExists: exists,
    });
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
    if (targetId === "settings-llm-api") llmApiConfigVisible = true;
    window.setTimeout(() => {
      document.getElementById(targetId)?.scrollIntoView({ block: "start", behavior: "smooth" });
    }, 50);
  }
  function sectionForSettingsPanel(targetId: string): Section {
    return getSectionForSettingsPanel(targetId);
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
    return getConfigSetupMessage(loaded, t);
  }
  function userErrorDetail(code: string | null | undefined, fallback = ""): UserErrorDetail {
    return getUserErrorDetail(code, fallback, language, t);
  }
  function userErrorMessage(code: string | null | undefined, fallback = "") {
    return getUserErrorMessage(code, fallback, language, t);
  }
  function activeUserErrorDetail() {
    if (inputStatus() !== "error") return null;
    return userErrorDetail(sessionErrorCode, statusMessage);
  }
  function isErrorStatus(message: string) {
    return isUserErrorStatus(message);
  }
  function appShellProps() {
    return {
      uiCompact,
      selectedSection,
      language,
      recording,
      inputStatus: inputStatus(),
      inputStatusLabel: inputStatusLabel(),
      inputStatusDesc: inputStatusDesc(),
      micBars,
      snapshotHotkey: snapshot.hotkey,
      requiresAsrAuth: requiresAsrAuth(),
      t,
      formatHotkey,
      micStatusText,
      sidebarMicStatusText,
      micBarHeight,
      micBarOpacity,
      onSelectSection: selectSection,
      onSetLanguage: setLanguage,
      onClose: closeWindow,
      onMinimize: minimizeWindow,
      onToggleMaximize: toggleMaximizeWindow,
    };
  }
  function appContentProps() {
    return {
      selectedSection,
      stats,
      t,
      uiCompact,
      recording,
      saving,
      settingsDirty,
      toolbarMessage: settingsToolbarMessage(),
      inputStatus: inputStatus(),
      inputStatusLabel: inputStatusLabel(),
      inputStatusDesc: inputStatusDesc(),
      requiresAsrAuth: requiresAsrAuth(),
      setupRequiredMessage,
      activeErrorDetail: activeUserErrorDetail(),
      sessionBusy: isSessionBusy(),
      snapshotHotkey: snapshot.hotkey,
      chineseTypingCharsPerMinute,
      configExists,
      setupChecking: setupStatusLoading && !setupStatus,
      setupStatusReady: setupIsReady(),
      setupStatusItems: setupStatusItems(),
      setupWarnings: setupStatus?.warnings ?? [],
      setupWarningCount: setupWarningCount(),
      testingAsr,
      testingLlm,
      hotkeyCaptureState,
      hotkeyValidationMessage,
      overlayColorPresets,
      overlayOpacityPresets,
      audioDevices,
      updateStatus,
      checkingUpdate,
      installingUpdate,
      openingLog,
      copyingDiagnosticReport,
      clearingRecentContext,
      generatingAutoHotwords,
      clearingAutoHotwordHistory,
      autoHotwordError,
      showAutoHotwordDetails: showAutoHotwordDetails(),
      hasLlmApiConfig: hasLlmApiConfig(),
      hotwordCount: hotwordCount(),
      acceptedAutoHotwordCount: acceptedAutoHotwordCount(),
      selectedAutoHotwordCount: selectedAutoHotwordCount(),
      autoHotwordStatusText: autoHotwordStatusText(),
      llmApiStatusText: llmApiStatusText(),
      advancedHotwordsOpen: isAdvancedVisible("Hotwords"),
      advancedApiConfigOpen: isAdvancedVisible("ApiConfig"),
      advancedOptionsOpen: isAdvancedVisible("Options"),
      fieldError,
      candidateConfidenceLabel,
      formatHotkey,
      formatNumber,
      formatHours,
      formatSavedHours,
      weeklySavedHours,
      usageTipText,
      triggerLabel,
      setupActionText,
      overlayBackgroundRgb,
      overlayOpacity,
      overlayTextColor,
      overlayBackgroundColor,
      overlayPresetActive,
      overlayOpacityPresetActive,
      overlayOpacityLabel,
      updatePanelTitle,
      updatePanelDescription,
      updateMetaText,
      historySummaryCards,
      recentSevenDayDisplayRows,
      onOpenSettings: openSettings,
      onOpenSetupGuide: openSetupGuide,
      onToggleRecording: toggleRecordingFromUi,
      onSelectSection: selectSection,
      onToggleTrigger: toggleTrigger,
      onReload: reloadConfigFromUi,
      onToggleAdvanced: toggleAdvanced,
      onUpdateHotwords: updateHotwords,
      onTidyHotwords: tidyHotwords,
      onClearHotwords: clearHotwords,
      onUpdatePromptContext: updatePromptContext,
      onClearRecentContext: clearRecentContextFromUi,
      onOptionEnabledNotice: maybeShowOptionEnabledNotice,
      onRestoreDefaultPrompt: restoreDefaultLlmPrompt,
      onPreviewFinalPrompt: previewFinalPrompt,
      onOpenLlmApiSettings: openLlmApiSettings,
      onGenerateAutoHotwords: generateAutoHotwords,
      onClearAutoHotwordHistory: clearAutoHotwordHistoryFromUi,
      onRefreshAutoHotwordStatus: refreshAutoHotwordStatus,
      onUpdateAcceptedAutoHotwords: updateAcceptedAutoHotwords,
      onTidyAcceptedAutoHotwords: tidyAcceptedAutoHotwords,
      onClearAcceptedAutoHotwords: clearAcceptedAutoHotwords,
      onApplySelectedAutoHotwords: applySelectedAutoHotwords,
      onScrollToSettingsPanel: scrollToSettingsPanel,
      onRefreshSetupStatus: refreshSetupStatus,
      onSetupAction: handleSetupAction,
      onTestAsrConfig: testAsrConfig,
      onTestLlmConfig: testLlmConfig,
      onHotkeyKeydown: handleHotkeyKeydown,
      onBeginHotkeyCapture: beginHotkeyCapture,
      onApplyOverlayPreset: applyOverlayPreset,
      onApplyOverlayOpacity: applyOverlayOpacity,
      onSetInputDevice: setInputDevice,
      onCheckUpdate: checkUpdate,
      onDownloadLatestUpdate: downloadLatestUpdate,
      onOpenLog: openLogFromUi,
      onCopyDiagnosticReport: copyDiagnosticReport,
    };
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

<AppGlobalStyles />

{#if isOverlay}
  <OverlayWindow
    meterBars={overlayMeterBars}
    displayLines={overlayDisplayLines}
    {recording}
    mode={overlayMode}
    fontSize={overlayFontSize}
    rootStyle={`--overlay-bg: ${overlayBackgroundColor()}; --overlay-bg-rgb: ${overlayBackgroundRgb()}; --overlay-opacity: ${overlayOpacity()}; --overlay-text: ${overlayTextColor()};`}
    meterBarHeight={overlayMeterBarHeight}
    meterBarOpacity={overlayMeterBarOpacity}
    bind:textElement={overlayTextElement}
  />
{:else if isToast}
  <StartupToast title={t("startupToastTitle")} hint={t("startupToastHint").replace("{hotkey}", formatHotkey(toastHotkey))} />
{:else}
<AppShell
  {...appShellProps()}
>
  <AppContent
    bind:config
    bind:autoHotwordCandidates
    bind:llmApiConfigVisible
    {...appContentProps()}
  />
</AppShell>
<ActionNotice message={actionNotice} kind={actionNoticeKind} />
<CloseToTrayDialog
  visible={closePromptVisible}
  title={t("closePromptTitle")}
  body={t("closePromptBody")}
  gotItLabel={t("closePromptGotIt")}
  dontShowAgainLabel={t("closePromptDontShowAgain")}
  exitLabel={t("closePromptExit")}
  onConfirm={() => hideWindowToTray(closePromptFirstTime && closePromptBehavior === "close_to_tray")}
  onDontShowAgain={closeWindowWithoutFuturePrompt}
  onExit={exitFromClosePrompt}
/>
{/if}

