import { describe, expect, it, vi } from "vitest";
import type { LastSessionOutcome } from "$lib/types/app";
import { createSessionController } from "./sessionController.svelte";

describe("session result preview", () => {
  it("clears the polished and original texts together when the next recording starts", () => {
    const setLastOutcome = vi.fn<(outcome: LastSessionOutcome) => void>();
    const session = createSessionController({
      t: (key) => key,
      safeInvoke: async () => null,
      notify: vi.fn(),
      requireAsrAuthGate: () => false,
      userErrorMessage: (_, fallback) => fallback,
      shouldOpenSettingsForError: () => false,
      scrollToSettingsPanel: vi.fn(),
      settingsPanelForError: () => "",
      isConfigError: () => false,
      sessionPhaseMessage: (phase) => phase,
      scheduleSucceededIdleHint: vi.fn(),
      getPhase: () => "succeeded",
      setRecording: vi.fn(),
      setPhase: vi.fn(),
      setErrorCode: vi.fn(),
      setLastOutcome,
      setLastAudioQualityDiagnostic: vi.fn(),
      setAudioLevel: vi.fn(),
      setStatusMessage: vi.fn(),
      clearSensitivePreviews: () => setLastOutcome(null),
    });

    session.applyAsrFinalText({
      generation: 12,
      text: "润色结果",
      raw_text: "识别原文",
      error: null,
      error_code: null,
      warning: null,
      warning_code: null,
    });
    expect(setLastOutcome).toHaveBeenLastCalledWith(expect.objectContaining({
      text: "润色结果",
      rawText: "识别原文",
    }));

    session.applyState({ recording: true, phase: "starting", message: "", error_code: null });
    expect(setLastOutcome).toHaveBeenLastCalledWith(null);
  });
});
