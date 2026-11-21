import { describe, expect, it } from "vitest";
import { fallbackConfig } from "$lib/app/defaults";
import { clonePlain } from "$lib/utils/config";
import { buildFinalPromptPreview } from "./autoHotwords";

const labels = {
  dictionary: "[USER_DICTIONARY]",
  dictionaryPurpose: "dict-purpose",
  dictionaryEnd: "[/USER_DICTIONARY]",
  context: "[CONTEXT]",
  contextPurpose: "ctx-purpose",
  contextEnd: "[/CONTEXT]",
  recentContext: "[RECENT_CONTEXT]",
  recentContextPurpose: "recent-purpose",
  recentContextEnd: "[/RECENT_CONTEXT]",
  recentContextPlaceholder: "(recent-placeholder)",
  screenOcrContext: "[SCREEN_OCR]",
  screenOcrPurpose: "ocr-purpose",
  screenOcrEnd: "[/SCREEN_OCR]",
  screenOcrPlaceholder: "(ocr-placeholder)",
  screenScreenshotContext: "[SCREEN_SCREENSHOT]",
  screenScreenshotPurpose: "screenshot-purpose",
  screenScreenshotEnd: "[/SCREEN_SCREENSHOT]",
  screenScreenshotPlaceholder: "(screenshot-placeholder)",
  referenceRules: "[REFERENCE_RULES]",
  systemPrompt: "SYSTEM:",
  userPromptTemplate: "USER:",
  empty: "(empty)",
  summaryTitle: "SUMMARY",
  sceneContextSummary: "SCENE",
  recentContextPolicyDisabled: "RECENT_OFF",
  recentContextPolicyEnabled: "RECENT_ON",
  recentContextPolicyNeedsLocal: "RECENT_NEEDS_LOCAL",
  screenScreenshotPolicyEnabled: "SCREEN_SCREENSHOT_ON",
  screenOcrPolicyEnabled: "SCREEN_OCR_ON",
  screenOcrPolicyDisabled: "SCREEN_OFF",
  actualPromptTitle: "ACTUAL",
};

describe("buildFinalPromptPreview", () => {
  it("shows compressed screenshot reference by default when screen context is enabled", () => {
    const config = clonePlain(fallbackConfig);
    config.screen_context.enabled = true;
    config.screen_context.llm_mode = "screenshot";

    const preview = buildFinalPromptPreview(config, "今天把尾音问题处理完。", [], labels);

    expect(preview).toContain("SCREEN_SCREENSHOT_ON");
    expect(preview).toContain("[SCREEN_SCREENSHOT]");
    expect(preview).not.toContain("[SCREEN_OCR]");
    expect(preview).not.toContain("SCREEN_OCR_ON");
  });

  it("shows OCR reference block and summary in ocr mode when budget allows", () => {
    const config = clonePlain(fallbackConfig);
    config.screen_context.enabled = true;
    config.screen_context.llm_mode = "ocr";

    const preview = buildFinalPromptPreview(config, "今天把尾音问题处理完。", [], labels);

    expect(preview).toContain("SCREEN_OCR_ON");
    expect(preview).toContain("[SCREEN_OCR]");
    expect(preview).not.toContain("[SCREEN_SCREENSHOT]");
    expect(preview).not.toContain("SCREEN_SCREENSHOT_ON");
  });

  it("omits both screenshot and OCR blocks when screen context is disabled", () => {
    const config = clonePlain(fallbackConfig);
    config.screen_context.enabled = false;
    config.screen_context.llm_mode = "screenshot";

    const preview = buildFinalPromptPreview(config, "今天把尾音问题处理完。", [], labels);

    expect(preview).toContain("SCREEN_OFF");
    expect(preview).not.toContain("[SCREEN_SCREENSHOT]");
    expect(preview).not.toContain("[SCREEN_OCR]");
  });

  it("keeps screenshot reference even when OCR char budget is zero while suppressing OCR mode", () => {
    const screenshotConfig = clonePlain(fallbackConfig);
    screenshotConfig.screen_context.enabled = true;
    screenshotConfig.screen_context.llm_mode = "screenshot";
    screenshotConfig.llm_post_edit.screen_context_max_chars = 0;

    const screenshotPreview = buildFinalPromptPreview(screenshotConfig, "test", [], labels);
    expect(screenshotPreview).toContain("[SCREEN_SCREENSHOT]");
    expect(screenshotPreview).toContain("SCREEN_SCREENSHOT_ON");

    const ocrConfig = clonePlain(fallbackConfig);
    ocrConfig.screen_context.enabled = true;
    ocrConfig.screen_context.llm_mode = "ocr";
    ocrConfig.llm_post_edit.screen_context_max_chars = 0;

    const ocrPreview = buildFinalPromptPreview(ocrConfig, "test", [], labels);
    expect(ocrPreview).toContain("SCREEN_OFF");
    expect(ocrPreview).not.toContain("[SCREEN_OCR]");
  });
});
