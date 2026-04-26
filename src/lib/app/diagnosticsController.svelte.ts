import { invoke } from "@tauri-apps/api/core";
import type { CopyKey } from "$lib/i18n";
import type { DiagnosticReport } from "$lib/types/app";

type NoticeKind = "success" | "info" | "warning" | "error";

type DiagnosticsControllerOptions = {
  hasTauriApi: () => boolean;
  t: (key: CopyKey, values?: Record<string, string>) => string;
  setStatusMessage: (message: string) => void;
  showActionNotice: (message: string, kind: NoticeKind) => void;
  logError: (message: string) => void;
};

export function createDiagnosticsController(options: DiagnosticsControllerOptions) {
  let openingLog = $state(false);
  let copyingReport = $state(false);

  async function openLog() {
    if (openingLog) return;
    if (!options.hasTauriApi()) {
      const message = options.t("browserPreview");
      options.setStatusMessage(message);
      options.showActionNotice(message, "error");
      return;
    }

    openingLog = true;
    try {
      await invoke("open_log_file");
      options.showActionNotice(options.t("logOpened"), "success");
    } catch (error) {
      const message = typeof error === "string" ? error : options.t("browserPreview");
      options.setStatusMessage(message);
      options.logError(`open log failed: ${formatError(error)}`);
      options.showActionNotice(message, "error");
    } finally {
      openingLog = false;
    }
  }

  async function copyReport() {
    if (copyingReport) return;
    if (!options.hasTauriApi()) {
      const message = options.t("browserPreview");
      options.setStatusMessage(message);
      options.showActionNotice(message, "error");
      return;
    }

    copyingReport = true;
    try {
      await invoke<DiagnosticReport>("copy_diagnostic_report_to_clipboard");
      const message = options.t("diagnosticCopied");
      options.setStatusMessage(message);
      options.showActionNotice(message, "success");
    } catch (error) {
      const message = typeof error === "string" ? error : options.t("browserPreview");
      options.setStatusMessage(message);
      options.logError(`copy diagnostic report failed: ${formatError(error)}`);
      options.showActionNotice(message, "error");
    } finally {
      copyingReport = false;
    }
  }

  return {
    get openingLog() { return openingLog; },
    get copyingReport() { return copyingReport; },
    openLog,
    copyReport,
  };
}

function formatError(error: unknown) {
  if (error instanceof Error) return error.stack || error.message;
  if (typeof error === "string") return error;
  try {
    return JSON.stringify(error);
  } catch {
    return String(error);
  }
}
