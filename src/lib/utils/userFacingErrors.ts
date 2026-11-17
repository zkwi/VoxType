import type { CopyKey } from "$lib/i18n";

const INVOKE_ERROR_CODE_KEYS: Record<string, CopyKey> = {
  LLM_TEST_MISSING_CONFIG: "llmTestErrorMissingConfig",
  LLM_TEST_THINKING_ONLY_MODEL: "llmTestErrorThinkingOnlyModel",
  LLM_TEST_OUTPUT_TRUNCATED: "llmTestErrorOutputTruncated",
  LLM_TEST_REASONING_BUDGET_EXHAUSTED: "llmTestErrorReasoningExhausted",
  LLM_TEST_BUDGET_EXHAUSTED: "llmTestErrorBudgetExhausted",
  LLM_TEST_REASONING_ONLY: "llmTestErrorReasoningOnly",
  LLM_TEST_EMPTY_OUTPUT: "llmTestErrorEmptyOutput",
  LLM_TEST_AUTH_FAILED: "llmTestErrorAuthFailed",
  LLM_TEST_RATE_LIMITED: "llmTestErrorRateLimited",
  LLM_TEST_THINKING_UNSUPPORTED: "llmTestErrorThinkingUnsupported",
  LLM_TEST_MODEL_UNAVAILABLE: "llmTestErrorModelUnavailable",
  LLM_TEST_NETWORK_FAILED: "llmTestErrorNetworkFailed",
  LLM_TEST_FAILED: "llmTestErrorFailed",
};

export function isSafeShortUserMessage(value: string) {
  const message = value.trim();
  if (!message || message.length > 180) return false;
  if (/[\r\n]/.test(message)) return false;
  if (/[A-Za-z]:\\|\/Users\/|\/home\//.test(message)) return false;
  if (/[{}[\]]/.test(message)) return false;
  if (/websocket|header|stack|panic|trace/i.test(message)) return false;
  if (/token|secret|access[_ -]?key|bearer|authorization/i.test(message)) return false;
  return true;
}

export function userFacingInvokeFailure(
  command: string,
  error: unknown,
  genericMessage: string,
  t?: (key: CopyKey) => string,
) {
  const raw = typeof error === "string" ? error.trim() : "";
  if (
    command === "test_asr_config" ||
    command === "test_llm_config" ||
    command === "test_screen_context" ||
    command === "generate_hotword_candidates"
  ) {
    const codeMatch = raw.match(/^([A-Z0-9_]+):\s*(.*)$/);
    if (codeMatch) {
      const [, code, fallbackText] = codeMatch;
      const mappedKey = INVOKE_ERROR_CODE_KEYS[code];
      if (mappedKey && t) {
        return t(mappedKey);
      }
      if (isSafeShortUserMessage(fallbackText)) {
        return fallbackText.trim();
      }
      return genericMessage;
    }
    if (isSafeShortUserMessage(raw)) {
      return raw;
    }
  }
  return genericMessage;
}
