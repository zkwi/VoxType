import type { AppConfig } from "$lib/types/app";

export function normalizeHotwords(value: string) {
  return value
    .split("\n")
    .map((item) => item.trim())
    .filter(Boolean);
}

export function dedupeHotwords(values: string[]) {
  const seen = new Set<string>();
  return values
    .map((item) => item.trim())
    .filter((item) => {
      if (!item) return false;
      const key = item.toLocaleLowerCase();
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    });
}

export function effectiveHotwords(config: AppConfig) {
  return dedupeHotwords([...config.context.hotwords, ...config.auto_hotwords.accepted_hotwords]);
}

export function hotwordCount(config: AppConfig) {
  return config.context.hotwords.filter((item) => item.trim()).length;
}

export function acceptedAutoHotwordCount(config: AppConfig) {
  return config.auto_hotwords.accepted_hotwords.filter((item) => item.trim()).length;
}

export function candidateConfidenceLabel(value: number) {
  const safeValue = Number.isFinite(value) ? Math.max(0, Math.min(1, value)) : 0;
  return `${Math.round(safeValue * 100)}%`;
}
