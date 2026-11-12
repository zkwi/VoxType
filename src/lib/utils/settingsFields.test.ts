import { describe, expect, it } from "vitest";
import { sectionForSettingsPanel } from "./appRouting";
import { settingsPanelForField } from "./settingsFields";

describe("settings validation routing", () => {
  it.each([
    ["auth.resource_id", "settings-asr-language", "ApiConfig"],
    ["audio.mute_system_volume_while_recording", "settings-audio", "Options"],
    ["auth.console_api_key", "settings-auth", "ApiConfig"],
    ["audio.input_gain_db", "settings-recording-troubleshooting", "Options"],
    ["asr.no_feedback_auto_stop_seconds", "settings-recording-troubleshooting", "Options"],
    ["context.hotwords", "settings-hotwords", "Hotwords"],
    ["screen_context.max_chars", "settings-screen-context", "Hotwords"],
    ["screen_context.capture_scope", "settings-screen-context", "Hotwords"],
    ["llm_post_edit.min_chars", "settings-llm-api", "ApiConfig"],
  ])("opens the owning panel for %s", (field, panel, section) => {
    expect(settingsPanelForField(field)).toBe(panel);
    expect(sectionForSettingsPanel(panel)).toBe(section);
  });
});
