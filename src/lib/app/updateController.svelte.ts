import type { CopyKey } from "$lib/i18n";
import type { InstallUpdateResult, UpdateStatus } from "$lib/types/app";

type SafeInvoke = <T>(
  command: string,
  args?: Record<string, unknown>,
  quiet?: boolean,
) => Promise<T | null>;

type NoticeKind = "success" | "info" | "warning" | "error";
type NoticeAction = {
  label: string;
  busyLabel?: string;
  isBusy?: () => boolean;
  onClick: () => void | Promise<void>;
};

type UpdateControllerOptions = {
  t: (key: CopyKey, values?: Record<string, string>) => string;
  safeInvoke: SafeInvoke;
  canAutoCheck: () => boolean;
  currentVersion: () => string;
  getStatusMessage: () => string;
  setStatusMessage: (message: string) => void;
  showActionNotice: (message: string, kind: NoticeKind, action?: NoticeAction) => void;
};

export function createUpdateController(options: UpdateControllerOptions) {
  let status = $state<UpdateStatus | null>(null);
  let checking = $state(false);
  let installing = $state(false);

  async function maybeAutoCheck() {
    if (!options.canAutoCheck()) return;
    await check(false);
  }

  async function check(manual = true) {
    if (checking) return;
    checking = true;
    const previousStatus = options.getStatusMessage();
    try {
      const result = await options.safeInvoke<UpdateStatus>("check_for_update", undefined, !manual);
      if (result) {
        status = result;
        if (manual || result.update_available) {
          const updateAction =
            result.update_available && result.asset_name
              ? {
                  label: options.t("updateNow"),
                  busyLabel: options.t("downloadingInstall"),
                  isBusy: () => installing,
                  onClick: downloadLatest,
                }
              : undefined;
          options.showActionNotice(result.message, result.update_available ? "warning" : "success", updateAction);
        } else {
          options.setStatusMessage(previousStatus);
        }
      } else if (manual && options.getStatusMessage()) {
        options.showActionNotice(options.getStatusMessage(), "error");
      } else {
        options.setStatusMessage(previousStatus);
      }
    } finally {
      checking = false;
    }
  }

  async function downloadLatest() {
    if (installing) return;
    installing = true;
    try {
      const result = await options.safeInvoke<InstallUpdateResult>("download_and_install_update");
      if (result) {
        options.showActionNotice(result.message, "success");
      }
    } finally {
      installing = false;
    }
  }

  function panelTitle() {
    if (!status) return options.t("updateNotChecked");
    if (status.update_available) return options.t("updateAvailable");
    return options.t("updateUpToDate");
  }

  function panelDescription() {
    if (!status) return options.t("updateIdleDescription");
    if (status.update_available && status.asset_name) {
      return options.t("updateReady", { asset: status.asset_name });
    }
    if (status.update_available) return options.t("updateNoInstaller");
    return status.message;
  }

  function metaText() {
    const current = status?.current_version ?? options.currentVersion();
    const latest = status?.latest_version ?? "-";
    const size = status?.asset_size ? ` · ${formatFileSize(status.asset_size)}` : "";
    return `${options.t("currentVersion")} v${current} · ${options.t("latestVersion")} ${latest === "-" ? "-" : `v${latest}`}${size}`;
  }

  function formatFileSize(bytes: number) {
    if (!Number.isFinite(bytes) || bytes <= 0) return "";
    if (bytes < 1024 * 1024) return `${Math.ceil(bytes / 1024)} KB`;
    return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  }

  return {
    get status() { return status; },
    get checking() { return checking; },
    get installing() { return installing; },
    maybeAutoCheck,
    check,
    downloadLatest,
    panelTitle,
    panelDescription,
    metaText,
  };
}
