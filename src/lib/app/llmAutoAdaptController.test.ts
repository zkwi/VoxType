import { describe, expect, it, vi } from "vitest";
import { fallbackConfig } from "$lib/app/defaults";
import { llmThinkingStrategyLabelKey } from "$lib/utils/llmConfig";
import { createLlmAutoAdaptController } from "./llmAutoAdaptController.svelte";

function createTestController() {
  const config = structuredClone(fallbackConfig);
  config.llm_post_edit.base_url = "http://127.0.0.1:3425/v1";
  config.llm_post_edit.api_key = "test-key";
  config.llm_post_edit.model = "deepseek-flash";
  config.llm_post_edit.thinking_strategy = "openrouter_reasoning_none";

  let testedFingerprint = "";
  let testStatusMessage = "";
  const runTest = vi.fn<
    (options?: {
      automatic?: boolean;
      expectedFingerprint?: string;
      forceThinkingStrategy?: string;
    }) => Promise<void>
  >(async () => {});

  const controller = createLlmAutoAdaptController({
    t: (key) => key,
    getConfig: () => config,
    isTesting: () => false,
    getTestStatusMessage: () => testStatusMessage,
    setTestStatusMessage: (message) => {
      testStatusMessage = message;
    },
    getTestedFingerprint: () => testedFingerprint,
    setTestedFingerprint: (fingerprint) => {
      testedFingerprint = fingerprint;
    },
    runTest,
  });

  controller.markLoaded(config);
  return { config, controller, runTest };
}

describe("LLM auto adapt controller", () => {
  it("forces auto strategy when switching models", async () => {
    const { config, controller, runTest } = createTestController();

    config.llm_post_edit.model = "google-vertex/gemini-4-hydrogen";
    controller.queueAfterSave(config);
    await Promise.resolve();

    expect(runTest).toHaveBeenCalledTimes(1);
    expect(runTest).toHaveBeenCalledWith(
      expect.objectContaining({
        automatic: true,
        forceThinkingStrategy: "auto",
      }),
    );
  });

  it("keeps manual thinking strategy on save, then returns to auto on next model switch", async () => {
    const { config, controller, runTest } = createTestController();

    config.llm_post_edit.model = "google-vertex/gemini-4-hydrogen";
    config.llm_post_edit.thinking_strategy = "openrouter_reasoning_low";
    controller.markManualStrategyChange("openrouter_reasoning_low");
    controller.queueAfterSave(config);
    await Promise.resolve();

    expect(runTest).toHaveBeenCalledTimes(1);
    expect(runTest.mock.calls[0]?.[0]?.forceThinkingStrategy).toBeUndefined();

    config.llm_post_edit.model = "kimi-k3.1-pro";
    controller.queueAfterSave(config);
    await Promise.resolve();

    expect(runTest).toHaveBeenCalledTimes(2);
    expect(runTest.mock.calls[1]?.[0]).toEqual(
      expect.objectContaining({
        automatic: true,
        forceThinkingStrategy: "auto",
      }),
    );
  });

  it("preserves manual thinking strategy during explicit manual test", async () => {
    const { config, controller, runTest } = createTestController();
    config.llm_post_edit.thinking_strategy = "openrouter_reasoning_low";
    controller.markManualStrategyChange("openrouter_reasoning_low");

    await controller.testManually();

    expect(runTest).toHaveBeenCalledTimes(1);
    expect(runTest).toHaveBeenCalledWith();
  });

  it("maps thinking strategies to localized label keys", () => {
    expect(llmThinkingStrategyLabelKey("openrouter_reasoning_low")).toBe(
      "llmThinkingStrategyOpenRouterLow",
    );
    expect(llmThinkingStrategyLabelKey("openrouter_reasoning_none")).toBe(
      "llmThinkingStrategyOpenRouterNone",
    );
    expect(llmThinkingStrategyLabelKey("dashscope_enable_thinking")).toBe(
      "llmThinkingStrategyDashscope",
    );
    expect(llmThinkingStrategyLabelKey("unknown_strategy")).toBeNull();
  });
});
