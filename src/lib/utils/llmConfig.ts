import type { CopyKey } from "$lib/i18n";
import type { AppConfig } from "$lib/types/app";

export const LLM_THINKING_STRATEGY_AUTO = "auto";

const THINKING_STRATEGY_LABEL_KEYS: Record<string, CopyKey> = {
  auto: "llmThinkingStrategyAuto",
  dashscope_enable_thinking: "llmThinkingStrategyDashscope",
  thinking_disabled: "llmThinkingStrategyDisabledObject",
  openrouter_reasoning_none: "llmThinkingStrategyOpenRouterNone",
  openrouter_reasoning_low: "llmThinkingStrategyOpenRouterLow",
  openrouter_reasoning_minimal: "llmThinkingStrategyOpenRouterMinimal",
  omit: "llmThinkingStrategyOmit",
};

export function llmThinkingStrategyLabelKey(strategy: string): CopyKey | null {
  return THINKING_STRATEGY_LABEL_KEYS[strategy] ?? null;
}

export function hasLlmAdapterTestConfig(config: AppConfig) {
  const llm = config.llm_post_edit;
  return Boolean(llm.base_url?.trim() && llm.api_key?.trim() && llm.model?.trim());
}

export function llmAdapterConfigFingerprint(config: AppConfig) {
  const llm = config.llm_post_edit;
  return JSON.stringify({
    base_url: llm.base_url.trim().replace(/\/+$/, ""),
    api_key: llm.api_key.trim(),
    model: llm.model.trim(),
    enable_thinking: llm.enable_thinking,
  });
}

export function llmModelTargetFingerprint(config: AppConfig) {
  const llm = config.llm_post_edit;
  return JSON.stringify({
    base_url: llm.base_url.trim().replace(/\/+$/, ""),
    model: llm.model.trim(),
    enable_thinking: llm.enable_thinking,
  });
}
