import type { CopyKey } from "$lib/i18n";
import type {
  AppConfig,
  AutoHotwordStatus,
  ConnectionTestResult,
  HotwordGenerationResult,
  SelectableHotwordCandidate,
} from "$lib/types/app";
import {
  applySelectedAutoHotwords as mergeSelectedAutoHotwords,
  autoHotwordStatusText as formatAutoHotwordStatusText,
  mapGeneratedHotwordCandidates,
  showAutoHotwordDetails as shouldShowAutoHotwordDetails,
} from "$lib/utils/autoHotwords";
import { acceptedAutoHotwordCount as countAcceptedAutoHotwords, dedupeHotwords, normalizeHotwords } from "$lib/utils/hotwords";
import { clonePlain } from "$lib/utils/config";

type SafeInvoke = <T>(
  command: string,
  args?: Record<string, unknown>,
  quiet?: boolean,
) => Promise<T | null>;

type NoticeKind = "success" | "info" | "warning" | "error";

type AutoHotwordsControllerOptions = {
  getConfig: () => AppConfig;
  t: (key: CopyKey, values?: Record<string, string>) => string;
  fieldError: (field: string) => string;
  effectiveHotwords: () => string[];
  getStatusMessage: () => string;
  setStatusMessage: (message: string) => void;
  showActionNotice: (message: string, kind: NoticeKind) => void;
  safeInvoke: SafeInvoke;
  canConfirm: () => boolean;
};

export function createAutoHotwordsController(options: AutoHotwordsControllerOptions) {
  let status = $state<AutoHotwordStatus | null>(null);
  let generating = $state(false);
  let clearingHistory = $state(false);
  let candidates = $state<SelectableHotwordCandidate[]>([]);
  let error = $state("");

  function acceptedCount() {
    return countAcceptedAutoHotwords(options.getConfig());
  }

  function selectedCount() {
    return candidates.filter((item) => item.selected).length;
  }

  function showDetails() {
    return shouldShowAutoHotwordDetails(options.getConfig(), options.fieldError);
  }

  function statusText() {
    return formatAutoHotwordStatusText(status, options.t);
  }

  async function refreshStatus() {
    const result = await options.safeInvoke<AutoHotwordStatus>("get_auto_hotword_status", undefined, true);
    if (result) status = result;
  }

  async function generate() {
    if (generating) return;
    generating = true;
    error = "";
    try {
      const result = await options.safeInvoke<HotwordGenerationResult>(
        "generate_hotword_candidates",
        { config: clonePlain(options.getConfig()) },
        false,
      );

      if (!result) {
        error = options.getStatusMessage();
        return;
      }

      candidates = mapGeneratedHotwordCandidates(result.candidates);
      const message = options.t("autoHotwordsGenerated", { count: String(result.candidates.length) });
      options.setStatusMessage(message);
      options.showActionNotice(message, result.candidates.length > 0 ? "success" : "warning");
      if (result.warning) options.showActionNotice(result.warning, "warning");
    } finally {
      generating = false;
    }
  }

  async function clearHistory() {
    if (clearingHistory) return;
    if (options.canConfirm() && !window.confirm(options.t("autoHotwordsClearConfirm"))) return;
    clearingHistory = true;
    error = "";
    try {
      const result = await options.safeInvoke<ConnectionTestResult>("clear_hotword_history", undefined, false);
      if (result) {
        options.showActionNotice(result.message, "success");
        candidates = [];
        await refreshStatus();
      } else {
        error = options.getStatusMessage();
      }
    } finally {
      clearingHistory = false;
    }
  }

  function applySelected() {
    if (selectedCount() === 0) {
      options.showActionNotice(options.t("autoHotwordsNoSelection"), "warning");
      return;
    }

    const config = options.getConfig();
    const { added, acceptedHotwords } = mergeSelectedAutoHotwords(
      config,
      candidates,
      options.effectiveHotwords(),
    );
    config.auto_hotwords.accepted_hotwords = acceptedHotwords;
    const message = options.t("autoHotwordsApplied", { count: String(added) });
    options.setStatusMessage(message);
    options.showActionNotice(message, added > 0 ? "success" : "warning");
  }

  function updateAccepted(value: string) {
    options.getConfig().auto_hotwords.accepted_hotwords = normalizeHotwords(value);
  }

  function tidyAccepted() {
    const config = options.getConfig();
    config.auto_hotwords.accepted_hotwords = dedupeHotwords(config.auto_hotwords.accepted_hotwords);
    options.showActionNotice(options.t("autoHotwordsAcceptedTidied"), "success");
  }

  function clearAccepted() {
    if (options.canConfirm() && !window.confirm(options.t("autoHotwordsAcceptedClearConfirm"))) return;
    options.getConfig().auto_hotwords.accepted_hotwords = [];
    options.showActionNotice(options.t("autoHotwordsAcceptedCleared"), "success");
  }

  return {
    get status() { return status; },
    get generating() { return generating; },
    get clearingHistory() { return clearingHistory; },
    get candidates() { return candidates; },
    set candidates(value: SelectableHotwordCandidate[]) { candidates = value; },
    get error() { return error; },
    acceptedCount,
    selectedCount,
    showDetails,
    statusText,
    refreshStatus,
    generate,
    clearHistory,
    applySelected,
    updateAccepted,
    tidyAccepted,
    clearAccepted,
  };
}
