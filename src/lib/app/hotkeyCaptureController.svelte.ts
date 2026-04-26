import type { CopyKey } from "$lib/i18n";
import type { AppConfig, HotkeyCaptureState } from "$lib/types/app";
import { formatHotkey, hotkeyFromKeyboardEvent, validateHotkeyText } from "$lib/utils/hotkeys";

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

  function setHotkey(value: string) {
    const formatted = formatHotkey(value);
    options.getConfig().hotkey = formatted;
    validationMessage = validate(formatted);
    if (!validationMessage) {
      const next = { ...options.getValidationErrors() };
      delete next.hotkey;
      options.setValidationErrors(next);
    }
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
    event.preventDefault();
    event.stopPropagation();

    if (event.key === "Escape") {
      cancelCapture();
      return;
    }

    if (event.key === "Backspace" || event.key === "Delete") {
      options.getConfig().hotkey = "";
      validationMessage = options.t("hotkeyRequired");
      options.setValidationErrors({
        ...options.getValidationErrors(),
        hotkey: validationMessage,
      });
      return;
    }

    if (event.key === "Enter" && !event.ctrlKey && !event.altKey && !event.shiftKey && !event.metaKey) {
      state = "idle";
      return;
    }

    const captured = hotkeyFromKeyboardEvent(event);
    if (!captured) {
      validationMessage = options.t("hotkeyUnsupported");
      return;
    }

    options.getConfig().hotkey = captured;
    validationMessage = validate(captured);
    options.setValidationErrors(
      validationMessage
        ? { ...options.getValidationErrors(), hotkey: validationMessage }
        : Object.fromEntries(Object.entries(options.getValidationErrors()).filter(([field]) => field !== "hotkey")),
    );
    if (!validationMessage) state = "idle";
  }

  return {
    get state() { return state; },
    get validationMessage() { return validationMessage; },
    get isIdle() { return state === "idle"; },
    validate,
    setHotkey,
    beginCapture,
    cancelCapture,
    handleKeydown,
  };
}
