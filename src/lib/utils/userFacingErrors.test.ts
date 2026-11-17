import { describe, expect, it } from "vitest";
import { copy, type CopyKey } from "$lib/i18n";
import { userFacingInvokeFailure } from "./userFacingErrors";

describe("user facing invoke failures", () => {
  it("translates LLM connection test error codes in all supported languages", () => {
    const cases: Array<[string, CopyKey]> = [
      ["LLM_TEST_RATE_LIMITED: 限流", "llmTestErrorRateLimited"],
      ["LLM_TEST_THINKING_UNSUPPORTED: 不支持思考参数", "llmTestErrorThinkingUnsupported"],
      ["LLM_TEST_REASONING_BUDGET_EXHAUSTED: 额度耗尽", "llmTestErrorReasoningExhausted"],
      ["LLM_TEST_MODEL_UNAVAILABLE: 模型不可用", "llmTestErrorModelUnavailable"],
    ];

    for (const language of ["zh-CN", "zh-TW", "en"] as const) {
      const t = (key: CopyKey) => copy[language][key];
      for (const [rawError, expectedKey] of cases) {
        expect(userFacingInvokeFailure("test_llm_config", rawError, "generic", t)).toBe(
          copy[language][expectedKey],
        );
      }
    }
  });

  it("strips unknown safe code prefixes and blocks unsafe error payloads", () => {
    expect(
      userFacingInvokeFailure("test_llm_config", "UNKNOWN_CODE: 请重试", "generic"),
    ).toBe("请重试");
    expect(
      userFacingInvokeFailure("test_llm_config", "C:\\Users\\demo\\secret token panic", "generic"),
    ).toBe("generic");
  });
});
