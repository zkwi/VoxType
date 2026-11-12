import { describe, expect, it } from "vitest";
import { fallbackConfig } from "$lib/app/defaults";
import { matchingPolishPreset, polishPresets } from "./polishPresets";

describe("polish presets", () => {
  it("recognizes the existing defaults as formal and edits as custom", () => {
    expect(matchingPolishPreset(fallbackConfig.llm_post_edit)?.label).toBe("polishPresetFormal");
    expect(matchingPolishPreset({ ...fallbackConfig.llm_post_edit, system_prompt: "Custom" })).toBeUndefined();
  });

  it("keeps the transcript placeholder and recognizes Windows line endings", () => {
    for (const preset of polishPresets) {
      expect(preset.user_prompt_template).toContain("{text}");
      expect(matchingPolishPreset({
        ...preset,
        user_prompt_template: preset.user_prompt_template.replaceAll("\n", "\r\n"),
      })).toBe(preset);
    }
  });
});
