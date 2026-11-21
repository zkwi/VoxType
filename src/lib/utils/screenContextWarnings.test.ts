import { describe, expect, it } from "vitest";
import type { CopyKey } from "$lib/i18n";
import {
  screenContextOcrWarningText,
  screenContextScreenshotWarningText,
} from "./screenContextWarnings";

const t = (key: CopyKey) => `i18n:${String(key)}`;

describe("screenContextWarnings", () => {
  it("maps structured OCR warning codes to localized keys", () => {
    expect(
      screenContextOcrWarningText({ warning_code: "SCREEN_OCR_EMPTY", warning: null }, t),
    ).toBe("i18n:screenContextNoText");
    expect(
      screenContextOcrWarningText(
        { warning_code: "SCREEN_OCR_FAILED_WITH_SCREENSHOT", warning: null },
        t,
      ),
    ).toBe("i18n:screenContextOcrFailedWithScreenshot");
    expect(
      screenContextOcrWarningText({ warning_code: null, warning: null }, t),
    ).toBeNull();
  });

  it("maps structured screenshot warning codes to localized keys", () => {
    expect(
      screenContextScreenshotWarningText(
        { screenshot_warning_code: "SCREEN_SCREENSHOT_TOO_LARGE", screenshot_warning: null },
        t,
      ),
    ).toBe("i18n:screenContextScreenshotTooLarge");
    expect(
      screenContextScreenshotWarningText(
        { screenshot_warning_code: "SCREEN_SCREENSHOT_ENCODE_FAILED", screenshot_warning: null },
        t,
      ),
    ).toBe("i18n:screenContextScreenshotEncodeFailed");
    expect(
      screenContextScreenshotWarningText(
        { screenshot_warning_code: null, screenshot_warning: null },
        t,
      ),
    ).toBeNull();
  });
});
