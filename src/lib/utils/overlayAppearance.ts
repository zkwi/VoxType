import type { AppConfig } from "$lib/types/app";

export function normalizedHexColor(value: string | undefined, fallback: string) {
  const trimmed = (value ?? "").trim();
  return /^#[0-9a-fA-F]{6}$/.test(trimmed) ? trimmed : fallback;
}

export function overlayBackgroundColor(ui: AppConfig["ui"]) {
  return normalizedHexColor(ui.background_color, "#176ee6");
}

export function overlayTextColor(ui: AppConfig["ui"]) {
  return normalizedHexColor(ui.text_color, "#ffffff");
}

export function overlayBackgroundRgb(backgroundColor: string) {
  const hex = backgroundColor.slice(1);
  const red = parseInt(hex.slice(0, 2), 16);
  const green = parseInt(hex.slice(2, 4), 16);
  const blue = parseInt(hex.slice(4, 6), 16);
  return `${red}, ${green}, ${blue}`;
}

export function overlayOpacity(ui: AppConfig["ui"]) {
  const value = Number(ui.opacity);
  if (!Number.isFinite(value)) return 0.9;
  return Math.min(1, Math.max(0.05, value));
}

export function overlayOpacityLabel(value: number) {
  return `${Math.round(value * 100)}%`;
}

export function overlayOpacityPresetActive(currentOpacity: number, value: number) {
  return Math.abs(currentOpacity - value) < 0.001;
}

export function overlayPresetActive(currentBackground: string, currentText: string, background: string, text: string) {
  return currentBackground.toLowerCase() === background.toLowerCase() && currentText.toLowerCase() === text.toLowerCase();
}
