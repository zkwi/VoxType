import { overlayLineHeight } from "$lib/app/defaults";
import type { OverlayMode } from "$lib/types/app";

export function normalizeOverlayText(text: string) {
  const raw = String(text || "").replace(/\r\n/g, "\n").replace(/\r/g, "\n").trim();
  if (!raw) return "";
  const lines: string[] = [];
  let blankPending = false;
  for (const line of raw.split("\n")) {
    const cleaned = line.trim();
    if (!cleaned) {
      blankPending = lines.length > 0;
      continue;
    }
    if (blankPending) lines.push("");
    lines.push(cleaned);
    blankPending = false;
  }
  return lines.join("\n");
}

export function resolveOverlayLayout(
  text: string,
  forceSmall: boolean,
  availableHeight: number,
  singleWrappedLineCount: number,
): { mode: OverlayMode; fontSize: number; lineLimit: number } {
  const compactLength = Array.from(text.replace(/\s/g, "")).length;
  const singleFont = fontForVisibleLines(1, 20, 18, availableHeight);
  const doubleFont = fontForVisibleLines(2, 16, 14, availableHeight);
  if (!forceSmall && singleWrappedLineCount <= 1 && compactLength <= 18) {
    return { mode: "single", fontSize: singleFont, lineLimit: 1 };
  }
  return { mode: "double", fontSize: doubleFont, lineLimit: 2 };
}

export function fontForVisibleLines(lines: number, preferred: number, min: number, availableHeight: number) {
  const fitted = Math.floor((availableHeight - 2) / (lines * overlayLineHeight));
  return Math.max(min, Math.min(preferred, fitted || preferred));
}

export function wrapOverlayText(text: string, fontSize: number, maxWidth: number, measureText: (text: string, fontSize: number) => number) {
  if (!text) return [""];
  if (!maxWidth) return text.split("\n");
  const lines: string[] = [];
  for (const paragraph of text.split("\n")) {
    if (!paragraph) {
      lines.push("");
      continue;
    }
    let current = "";
    for (const char of Array.from(paragraph)) {
      const candidate = current + char;
      if (current && measureText(candidate, fontSize) > maxWidth) {
        lines.push(current);
        current = char.trimStart();
      } else {
        current = candidate;
      }
    }
    lines.push(current);
  }
  return lines.length ? lines : [""];
}

export function overlayVisibleLineCount(lineLimit: number) {
  return Math.max(1, Math.min(2, lineLimit || 1));
}

export function overlayAvailableTextHeight(windowHeight: number) {
  return Math.max(1, windowHeight - 24);
}
