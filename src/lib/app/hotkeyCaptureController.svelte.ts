import type { CopyKey } from "$lib/i18n";
import type { AppConfig, HotkeyCaptureState } from "$lib/types/app";
import { hotkeyFromKeyboardEvent, validateHotkeyText } from "$lib/utils/hotkeys";

type HotkeyCaptureControllerOptions = {
  getConfig: () => AppConfig;
  t: (key: CopyKey, values?: Record<string, string>) => string;
  getValidationErrors: () => Record<string, string>;
  setValidationErrors: (errors: Record<string, string>) => void;
  setNativeCapture: (active: boolean) => Promise<boolean>;
};

export function createHotkeyCaptureController(options: HotkeyCaptureControllerOptions) {
  let state = $state<HotkeyCaptureState>("idle");
  let validationMessage = $state("");
  let captureSequence = 0;
  let nativeCapture = Promise.resolve(true);

  function setNativeCapture(active: boolean) {
    nativeCapture = nativeCapture.then(() => options.setNativeCapture(active)).catch(() => false);
    return nativeCapture;
  }

  function validationLabels() {
    return {
      required: options.t("hotkeyRequired"),
      needsModifier: options.t("hotkeyNeedsModifier"),
      unsupported: options.t("hotkeyUnsupported"),
    };
  }

  function validate(value: string) {
    return validateHotkeyText(value, validationLabels());
  }

  async function beginCapture() {
    if (state !== "idle") return;
    validationMessage = "";
    const sequence = ++captureSequence;
    state = "preparing";
    const enabled = await setNativeCapture(true);
    if (sequence !== captureSequence) return;
    if (!enabled) {
      void setNativeCapture(false);
      state = "idle";
      validationMessage = options.t("hotkeyCaptureUnavailable");
      return;
    }
    state = "recording";
  }

  function cancelCapture() {
    if (state === "idle") return;
    captureSequence += 1;
    state = "idle";
    validationMessage = "";
    void setNativeCapture(false);
  }

  function handleKeydown(event: KeyboardEvent) {
    if (state === "idle") return;
    const hasModifier = event.ctrlKey || event.altKey || event.shiftKey || event.metaKey;
    if (event.key === "Tab" && !hasModifier) return;
    event.preventDefault();
    event.stopPropagation();

    if (event.key === "Escape") {
      cancelCapture();
      return;
    }
    if (state === "preparing") return;

    if (event.key === "Backspace" || event.key === "Delete") {
      validationMessage = "";
      return;
    }

    if (event.key === "Enter" && !hasModifier) {
      cancelCapture();
      return;
    }

    const captured = hotkeyFromKeyboardEvent(event);
    if (!captured) {
      validationMessage = options.t("hotkeyUnsupported");
      return;
    }

    // 录制期间保留旧值，失焦、取消和切页都不会把空组合写入自动保存。
    options.getConfig().hotkey = captured;
    const next = { ...options.getValidationErrors() };
    delete next.hotkey;
    options.setValidationErrors(next);
    cancelCapture();
  }

  return {
    get state() { return state; },
    get validationMessage() { return validationMessage; },
    get isIdle() { return state === "idle"; },
    validate,
    beginCapture,
    cancelCapture,
    handleKeydown,
  };
}
