import { invoke } from "@tauri-apps/api/core";

export function frontendMode(isOverlay: boolean, isToast: boolean) {
  if (isOverlay) return "overlay";
  if (isToast) return "toast";
  return "main";
}

export function hasTauriApi() {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export function logFrontendEvent(message: string) {
  if (!hasTauriApi()) return;
  void invoke("log_frontend_event", { message: truncateLogMessage(message) }).catch(() => undefined);
}

export function logFrontendError(message: string) {
  if (!hasTauriApi()) return;
  void invoke("log_frontend_error", { message: truncateLogMessage(message) }).catch(() => undefined);
}

export function truncateLogMessage(message: string) {
  return message.length > 1200 ? `${message.slice(0, 1200)}...` : message;
}

export function formatFrontendError(error: unknown) {
  if (error instanceof Error) return error.stack || error.message;
  if (typeof error === "string") return error;
  try {
    return JSON.stringify(error);
  } catch {
    return String(error);
  }
}
