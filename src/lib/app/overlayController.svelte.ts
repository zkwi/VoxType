import type { AppConfig, OverlayConfig, OverlayMode, OverlayText } from "$lib/types/app";
import {
  overlayBackgroundColor as getOverlayBackgroundColor,
  overlayBackgroundRgb as getOverlayBackgroundRgb,
  overlayOpacity as getOverlayOpacity,
  overlayOpacityPresetActive as isOverlayOpacityPresetActive,
  overlayPresetActive as isOverlayPresetActive,
  overlayTextColor as getOverlayTextColor,
} from "$lib/utils/overlayAppearance";
import {
  normalizeOverlayText,
  overlayAvailableTextHeight as getOverlayAvailableTextHeight,
  overlayMeterScale,
  resolveOverlayDisplayText,
} from "$lib/utils/overlayLayout";
import { createAdaptivePoller } from "$lib/utils/adaptivePoller";
import { overlayStatusText } from "$lib/utils/statusCodes";
import type { CopyKey } from "$lib/i18n";

type SafeInvoke = <T>(
  command: string,
  args?: Record<string, unknown>,
  quiet?: boolean,
) => Promise<T | null>;

type OverlayControllerOptions = {
  getConfig: () => AppConfig;
  updateUi: (ui: Partial<AppConfig["ui"]>) => void;
  isOverlay: () => boolean;
  isRecording: () => boolean;
  getAudioLevel: () => number;
  safeInvoke: SafeInvoke;
  t: (key: CopyKey) => string;
};

export function createOverlayController(options: OverlayControllerOptions) {
  let measureCanvas: HTMLCanvasElement | undefined;
  let text = $state("");
  let mode = $state<OverlayMode>("single");
  let fontSize = $state(20);
  let displayLines = $state<string[]>([""]);
  let textElement = $state<HTMLDivElement | null>(null);
  let visible = $state(false);
  let dismissing = $state(false);
  let visibilityRevision = 0;
  let textRevision = 0;
  let currentPayload: OverlayText | undefined;
  let configPollPending = false;
  let lastConfigPollAt = 0;
  const configPollIntervalMs = 1000;
  // 字幕和外观主要靠事件推送，轮询只是兜底：字幕窗可见时保持 250ms，
  // 隐藏时降为 2 秒心跳，避免常驻托盘时每秒数次无意义的后端往返。
  const poller = createAdaptivePoller({
    poll: pollVisibleOverlay,
    liveIntervalMs: 250,
    idleIntervalMs: 2000,
  });

  async function pollVisibleOverlay() {
    const visibilityAtRequest = visibilityRevision;
    const textAtRequest = textRevision;
    const result = await options.safeInvoke<OverlayText>("get_overlay_text", undefined, true);
    if (visibilityAtRequest !== visibilityRevision || !result) return visible;
    if (textAtRequest === textRevision && result.text?.trim()) applyOverlayText(result);
    if (result.visible !== visible) applyVisibility(result.visible === true);
    if (visible) void refreshConfig();
    return visible;
  }

  function applyVisibility(value: boolean) {
    visibilityRevision += 1;
    visible = value;
    dismissing = false;
    if (value) poller.wake();
  }

  function beginDismissal() {
    visibilityRevision += 1;
    if (visible) dismissing = true;
  }

  async function refreshConfig(force = false) {
    if (!options.isOverlay() || configPollPending) return;
    const now = Date.now();
    if (!force && now - lastConfigPollAt < configPollIntervalMs) return;
    configPollPending = true;
    lastConfigPollAt = now;
    try {
      const result = await options.safeInvoke<OverlayConfig>("get_overlay_config", undefined, true);
      if (result?.ui) applyConfig(result.ui);
    } finally {
      configPollPending = false;
    }
  }

  function refreshLayout() {
    if (options.isOverlay()) applyText(text, true);
  }

  function applyConfig(ui: AppConfig["ui"]) {
    if (!options.isOverlay()) return;
    // 后端在显示字幕窗前会先推送一次外观，借这个事件立刻恢复快速轮询。
    poller.wake();
    if (!uiConfigChanged(ui)) return;
    options.updateUi(ui);
    applyText(text, true);
  }

  function uiConfigChanged(ui: AppConfig["ui"]) {
    const current = options.getConfig().ui;
    return (
      current.width !== ui.width ||
      current.height !== ui.height ||
      current.margin_bottom !== ui.margin_bottom ||
      current.opacity !== ui.opacity ||
      current.background_color !== ui.background_color ||
      current.text_color !== ui.text_color
    );
  }

  function applyText(rawText: string, force = false) {
    const normalized = normalizeOverlayText(rawText) || options.t("overlayStatusRecording");
    if (!force && normalized === text) return;
    text = normalized;

    const layout = resolveOverlayDisplayText(
      normalized,
      availableTextHeight(),
      textContentWidth(),
      measureText,
      // 麦克风错误需要保留原因和操作提示，高 DPI 的较矮字幕窗也允许排两行。
      currentPayload?.status_code?.startsWith("MIC_") ? 12 : undefined,
    );
    mode = layout.mode;
    fontSize = layout.fontSize;
    if (layout.lines.length !== displayLines.length || layout.lines.some((line, index) => line !== displayLines[index])) {
      displayLines = layout.lines;
    }
  }

  function applyPayload(payload: OverlayText) {
    textRevision += 1;
    poller.wake();
    applyOverlayText(payload);
  }

  function applyOverlayText(payload: OverlayText) {
    currentPayload = payload;
    refreshLanguage();
  }

  function refreshLanguage() {
    applyText(currentPayload ? overlayStatusText(
      currentPayload.status_code,
      currentPayload.fallback_text ?? currentPayload.text,
      options.t,
    ) : "");
  }

  function startPolling() {
    if (options.isOverlay()) poller.start();
  }

  function dispose() {
    poller.stop();
  }

  function textContentWidth() {
    if (!textElement) {
      return Math.max(80, window.innerWidth - 88);
    }

    const styles = window.getComputedStyle(textElement);
    const paddingLeft = Number.parseFloat(styles.paddingLeft) || 0;
    const paddingRight = Number.parseFloat(styles.paddingRight) || 0;
    return Math.max(80, textElement.clientWidth - paddingLeft - paddingRight);
  }

  function measureText(value: string, size: number) {
    measureCanvas ??= document.createElement("canvas");
    const context = measureCanvas.getContext("2d");
    if (!context) return Array.from(value).length * size;
    context.font = `400 ${size}px "Microsoft YaHei", "Segoe UI", "PingFang SC", sans-serif`;
    return context.measureText(value).width;
  }

  function availableTextHeight() {
    if (textElement?.clientHeight) {
      return Math.max(1, textElement.clientHeight);
    }
    return getOverlayAvailableTextHeight(window.innerHeight);
  }

  function meterBarScale(index: number) {
    return overlayMeterScale(index, options.isRecording() ? options.getAudioLevel() : 0);
  }

  function backgroundColor() {
    return getOverlayBackgroundColor(options.getConfig().ui);
  }

  function textColor() {
    return getOverlayTextColor(options.getConfig().ui);
  }

  function backgroundRgb() {
    return getOverlayBackgroundRgb(backgroundColor());
  }

  function opacity() {
    return getOverlayOpacity(options.getConfig().ui);
  }

  function applyOpacity(value: number) {
    options.updateUi({ opacity: value });
  }

  function opacityPresetActive(value: number) {
    return isOverlayOpacityPresetActive(opacity(), value);
  }

  function applyPreset(background: string, textValue: string) {
    options.updateUi({ background_color: background, text_color: textValue });
  }

  function presetActive(background: string, textValue: string) {
    return isOverlayPresetActive(backgroundColor(), textColor(), background, textValue);
  }

  return {
    get mode() { return mode; },
    get fontSize() { return fontSize; },
    get displayLines() { return displayLines; },
    get visible() { return visible; },
    get dismissing() { return dismissing; },
    get textElement() { return textElement; },
    set textElement(value: HTMLDivElement | null) {
      textElement = value;
      if (value && options.isOverlay()) applyText(text, true);
    },
    get rootStyle() {
      return `--overlay-bg: ${backgroundColor()}; --overlay-bg-rgb: ${backgroundRgb()}; --overlay-opacity: ${opacity()}; --overlay-text: ${textColor()};`;
    },
    startPolling,
    refreshConfig,
    refreshLayout,
    refreshLanguage,
    applyConfig,
    applyPayload,
    applyVisibility,
    beginDismissal,
    dispose,
    meterBarScale,
    backgroundColor,
    textColor,
    backgroundRgb,
    opacity,
    applyOpacity,
    opacityPresetActive,
    applyPreset,
    presetActive,
  };
}
