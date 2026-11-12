import { listen } from "@tauri-apps/api/event";
import type {
  AsrFinalText,
  AsrOutputWarning,
  AudioDeviceFallbackNotice,
  AudioLevel,
  AudioQualityDiagnostic,
  CloseToTrayRequest,
  ConfigExitGuardRequest,
  OverlayConfig,
  OverlayText,
  SessionState,
  StatsSnapshot,
} from "$lib/types/app";

type NativeUnlistener = Promise<() => void>;

type SessionEventHandlers = {
  applySessionState: (state: SessionState) => void;
  applyAudioLevel: (payload: AudioLevel) => void;
};

type OverlayEventHandlers = {
  overlay: true;
  applyOverlayText: (payload: OverlayText) => void;
  applyOverlayConfig: (payload: OverlayConfig) => void;
  applyOverlayVisibility: (visible: boolean) => void;
  beginOverlayDismissal: () => void;
};

type MainEventHandlers = {
  overlay: false;
  applyAsrFinalText: (payload: AsrFinalText) => void;
  applyOutputWarning: (payload: AsrOutputWarning) => void;
  applyStats: (payload: StatsSnapshot) => void;
  applyAudioQuality: (payload: AudioQualityDiagnostic) => void;
  handleAudioDeviceFallback: (payload: AudioDeviceFallbackNotice) => void;
  showClosePrompt: (payload: CloseToTrayRequest) => void;
  showConfigExitGuard: (payload: ConfigExitGuardRequest) => void;
  handleMainWindowHidden: () => void;
  handleMainWindowShown: () => void;
  checkForUpdate: () => void;
};

export function registerNativeEventController(
  options: SessionEventHandlers & (OverlayEventHandlers | MainEventHandlers),
): NativeUnlistener[] {
  const sessionListeners = [
    listen<SessionState>("session-state-changed", (event) => {
      options.applySessionState(event.payload);
    }),
    listen<AudioLevel>("audio-level", (event) => {
      options.applyAudioLevel(event.payload);
    }),
  ];

  if (options.overlay) {
    return [
      ...sessionListeners,
      listen<OverlayText>("overlay-text", (event) => {
        options.applyOverlayText(event.payload);
      }),
      listen<OverlayConfig>("overlay-config", (event) => {
        options.applyOverlayConfig(event.payload);
      }),
      listen<boolean>("overlay-visibility", (event) => {
        options.applyOverlayVisibility(event.payload);
      }),
      listen("overlay-dismissing", options.beginOverlayDismissal),
    ];
  }

  return [
    ...sessionListeners,
    listen<AsrFinalText>("asr-final-text", (event) => {
      options.applyAsrFinalText(event.payload);
    }),
    listen<AsrOutputWarning>("asr-output-warning", (event) => {
      options.applyOutputWarning(event.payload);
    }),
    listen<StatsSnapshot>("usage-stats-updated", (event) => {
      options.applyStats(event.payload);
    }),
    listen<AudioQualityDiagnostic>("audio-quality-diagnostic", (event) => {
      options.applyAudioQuality(event.payload);
    }),
    listen<AudioDeviceFallbackNotice>("audio-device-fallback", (event) => {
      options.handleAudioDeviceFallback(event.payload);
    }),
    listen<CloseToTrayRequest>("close-to-tray-requested", (event) => {
      options.showClosePrompt(event.payload);
    }),
    listen<ConfigExitGuardRequest>("config-exit-guard-requested", (event) => {
      options.showConfigExitGuard(event.payload);
    }),
    listen("main-window-hidden", () => {
      options.handleMainWindowHidden();
    }),
    listen("main-window-shown", () => {
      options.handleMainWindowShown();
    }),
    listen("check-update-requested", () => {
      options.checkForUpdate();
    }),
  ];
}

export function disposeNativeEventController(unlisteners: NativeUnlistener[]) {
  void Promise.all(unlisteners).then((disposers) => {
    for (const dispose of disposers) dispose();
  });
}
