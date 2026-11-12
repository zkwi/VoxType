import type { CopyKey } from "$lib/i18n";
import type { AppConfig, HotkeyCaptureState } from "$lib/types/app";
import { hotkeyFromKeyboardEvent, validateHotkeyText } from "$lib/utils/hotkeys";

type HotkeyCaptureControllerOptions = {
  getConfig: () => AppConfig;
  t: (key: CopyKey, values?: Record<string, string>) => string;
  getValidationErrors: () => Record<string, string>;
  setValidationErrors: (errors: Record<string, string>) => void;
};

export function createHotkeyCaptureController(options: HotkeyCaptureControllerOptions) {
  let state = $state<HotkeyCaptureState>("idle");
  let validationMessage = $state("");

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

  function beginCapture() {
    validationMessage = "";
    state = "recording";
  }

  function cancelCapture() {
    state = "idle";
    validationMessage = "";
  }

  function handleKeydown(event: KeyboardEvent) {
    if (state !== "recording") return;
    const hasModifier = event.ctrlKey || event.altKey || event.shiftKey || event.metaKey;
    if (event.key === "Tab" && !hasModifier) return;
    event.preventDefault();
    event.stopPropagation();

    if (event.key === "Escape") {
      cancelCapture();
      return;
    }

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
