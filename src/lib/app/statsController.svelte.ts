import { chineseTypingCharsPerMinute, emptyStats, emptyUsage } from "$lib/app/defaults";
import type { CopyKey, Language } from "$lib/i18n";
import type { HistoryDayRow, HistorySummaryCard } from "$lib/components/pages/HistorySection.svelte";
import type { StatsSnapshot } from "$lib/types/app";
import {
  cumulativeUsageText,
  formatMinutes,
  formatNumber,
  historySummaryCards as buildHistorySummaryCards,
  recentSevenDayDisplayRows as buildRecentSevenDayDisplayRows,
  weeklySavedHours as weeklySavedHoursForStats,
} from "$lib/utils/stats";

type StatsControllerOptions = {
  t: (key: CopyKey, values?: Record<string, string>) => string;
  getLanguage: () => Language;
};

export function createStatsController(options: StatsControllerOptions) {
  let snapshot = $state<StatsSnapshot>(emptyStats);

  function apply(nextSnapshot: StatsSnapshot) {
    snapshot = nextSnapshot;
  }

  function weeklySavedHours() {
    return weeklySavedHoursForStats(snapshot, chineseTypingCharsPerMinute);
  }

  function usageTipText() {
    return cumulativeUsageText(snapshot, options.t, options.getLanguage());
  }

  function weeklyActivityText() {
    return options.t("weeklyActivity", {
      sessions: formatNumber(snapshot.recent_7d.session_count, options.getLanguage()),
      duration: formatMinutes(snapshot.recent_7d.total_seconds, options.getLanguage()),
    });
  }

  function historySummaryCards(): HistorySummaryCard[] {
    return buildHistorySummaryCards(snapshot, options.t, options.getLanguage(), chineseTypingCharsPerMinute);
  }

  function recentSevenDayDisplayRows(): HistoryDayRow[] {
    return buildRecentSevenDayDisplayRows(snapshot, options.t, options.getLanguage(), chineseTypingCharsPerMinute, emptyUsage);
  }

  return {
    get snapshot() { return snapshot; },
    apply,
    weeklySavedHours,
    usageTipText,
    weeklyActivityText,
    historySummaryCards,
    recentSevenDayDisplayRows,
  };
}
