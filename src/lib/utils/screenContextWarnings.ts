import type { CopyKey } from "$lib/i18n";
import type { ScreenContextTestResult } from "$lib/types/app";

type Translate = (key: CopyKey, values?: Record<string, string>) => string;

export function screenContextOcrWarningText(
  result: Pick<ScreenContextTestResult, "warning_code" | "warning">,
  t: Translate,
): string | null {
  switch (result.warning_code) {
    case "SCREEN_OCR_EMPTY":
      return t("screenContextNoText");
    case "SCREEN_OCR_FAILED_WITH_SCREENSHOT":
      return t("screenContextOcrFailedWithScreenshot");
    default:
      return result.warning?.trim() ? result.warning : null;
  }
}

export function screenContextScreenshotWarningText(
  result: Pick<ScreenContextTestResult, "screenshot_warning_code" | "screenshot_warning">,
  t: Translate,
): string | null {
  switch (result.screenshot_warning_code) {
    case "SCREEN_SCREENSHOT_TOO_LARGE":
      return t("screenContextScreenshotTooLarge");
    case "SCREEN_SCREENSHOT_ENCODE_FAILED":
      return t("screenContextScreenshotEncodeFailed");
    default:
      return result.screenshot_warning?.trim() ? result.screenshot_warning : null;
  }
}
