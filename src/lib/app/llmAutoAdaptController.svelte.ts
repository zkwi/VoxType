import type { CopyKey } from "$lib/i18n";
import type { AppConfig } from "$lib/types/app";
import {
  hasLlmAdapterTestConfig,
  llmAdapterConfigFingerprint,
  llmModelTargetFingerprint,
  LLM_THINKING_STRATEGY_AUTO,
} from "$lib/utils/llmConfig";

type LlmConfigTestOptions = {
  automatic?: boolean;
  expectedFingerprint?: string;
  forceThinkingStrategy?: string;
};

type LlmAutoAdaptControllerOptions = {
  t: (key: CopyKey, values?: Record<string, string>) => string;
  getConfig: () => AppConfig;
  isTesting: () => boolean;
  getTestStatusMessage: () => string;
  setTestStatusMessage: (message: string) => void;
  getTestedFingerprint: () => string;
  setTestedFingerprint: (fingerprint: string) => void;
  runTest: (options?: LlmConfigTestOptions) => Promise<void>;
};

export function createLlmAutoAdaptController(options: LlmAutoAdaptControllerOptions) {
  let pendingFingerprint: string | null = null;
  let running = false;
  let manualStrategyOverride = false;
  let lastModelTarget = "";

  function markLoaded(config: AppConfig) {
    options.setTestedFingerprint(llmAdapterConfigFingerprint(config));
    lastModelTarget = llmModelTargetFingerprint(config);
    manualStrategyOverride = false;
    pendingFingerprint = null;
  }

  function markManualStrategyChange(strategy: string) {
    manualStrategyOverride = strategy !== LLM_THINKING_STRATEGY_AUTO;
    lastModelTarget = llmModelTargetFingerprint(options.getConfig());
  }

  function queueAfterSave(savedConfig: AppConfig) {
    const fingerprint = llmAdapterConfigFingerprint(savedConfig);
    if (!hasLlmAdapterTestConfig(savedConfig)) {
      options.setTestedFingerprint(fingerprint);
      options.setTestStatusMessage("");
      lastModelTarget = llmModelTargetFingerprint(savedConfig);
      pendingFingerprint = null;
      return;
    }
    if (fingerprint === options.getTestedFingerprint()) return;
    pendingFingerprint = fingerprint;
    void runPending();
  }

  async function runPending() {
    if (running || options.isTesting() || !pendingFingerprint) return;
    const fingerprint = pendingFingerprint;
    const currentConfig = options.getConfig();
    if (
      !hasLlmAdapterTestConfig(currentConfig) ||
      llmAdapterConfigFingerprint(currentConfig) !== fingerprint
    ) {
      pendingFingerprint = null;
      return;
    }
    const modelTarget = llmModelTargetFingerprint(currentConfig);
    if (lastModelTarget && modelTarget !== lastModelTarget) {
      manualStrategyOverride = false;
    }
    const useAutoStrategy = !manualStrategyOverride;
    manualStrategyOverride = false;
    lastModelTarget = modelTarget;
    pendingFingerprint = null;
    running = true;
    options.setTestedFingerprint(fingerprint);
    try {
      await options.runTest({
        automatic: true,
        expectedFingerprint: fingerprint,
        ...(useAutoStrategy ? { forceThinkingStrategy: LLM_THINKING_STRATEGY_AUTO } : {}),
      });
    } finally {
      running = false;
      if (pendingFingerprint && pendingFingerprint !== options.getTestedFingerprint()) {
        void runPending();
      }
    }
  }

  async function testManually() {
    if (options.isTesting()) return;
    const testedFingerprint = llmAdapterConfigFingerprint(options.getConfig());
    manualStrategyOverride = false;
    lastModelTarget = llmModelTargetFingerprint(options.getConfig());
    await options.runTest();
    const currentConfig = options.getConfig();
    if (
      hasLlmAdapterTestConfig(currentConfig) &&
      llmAdapterConfigFingerprint(currentConfig) === testedFingerprint
    ) {
      options.setTestedFingerprint(testedFingerprint);
      if (pendingFingerprint === testedFingerprint) pendingFingerprint = null;
    }
    void runPending();
  }

  function statusText() {
    const config = options.getConfig();
    if (!hasLlmAdapterTestConfig(config)) return "";
    if (options.isTesting()) return options.getTestStatusMessage() || options.t("testingConnection");
    if (llmAdapterConfigFingerprint(config) !== options.getTestedFingerprint()) {
      return options.t("llmAutoTestPending");
    }
    return options.getTestStatusMessage();
  }

  return {
    markLoaded,
    markManualStrategyChange,
    queueAfterSave,
    testManually,
    statusText,
  };
}
