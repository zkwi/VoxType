import { getCurrentWindow } from "@tauri-apps/api/window";
import type { AppConfig, CloseToTrayRequest, LoadedConfig } from "$lib/types/app";

type SafeInvoke = <T>(
  command: string,
  args?: Record<string, unknown>,
  quiet?: boolean,
) => Promise<T | null>;

type WindowControllerOptions = {
  getConfig: () => AppConfig;
  applyLoadedConfig: (loaded: LoadedConfig) => void;
  safeInvoke: SafeInvoke;
};

export function createWindowController(options: WindowControllerOptions) {
  let closePromptVisible = $state(false);
  let closePromptFirstTime = $state(false);
  let closePromptBehavior = $state("close_to_tray");

  function showClosePrompt(request: CloseToTrayRequest) {
    closePromptFirstTime = request.first_time;
    closePromptBehavior = request.behavior;
    closePromptVisible = true;
  }

  async function minimize() {
    try {
      await getCurrentWindow().minimize();
    } catch (error) {
      console.warn(error);
    }
  }

  async function toggleMaximize() {
    try {
      await getCurrentWindow().toggleMaximize();
    } catch (error) {
      console.warn(error);
    }
  }

  async function close() {
    try {
      await getCurrentWindow().close();
    } catch (error) {
      console.warn(error);
    }
  }

  async function confirmClosePrompt() {
    await hideToTray(closePromptFirstTime && closePromptBehavior === "close_to_tray");
  }

  async function hideToTray(markSeen: boolean) {
    closePromptVisible = false;
    if (markSeen) {
      await saveClosePreference(options.getConfig().tray.close_behavior, true);
    }
    await options.safeInvoke<void>("hide_main_window", undefined, true);
  }

  async function closeWithoutFuturePrompt() {
    closePromptVisible = false;
    await saveClosePreference("close_to_tray", true);
    await options.safeInvoke<void>("hide_main_window", undefined, true);
  }

  async function exitFromPrompt() {
    closePromptVisible = false;
    await options.safeInvoke<void>("exit_application", undefined, true);
  }

  async function saveClosePreference(behavior: string, noticeShown: boolean) {
    const result = await options.safeInvoke<LoadedConfig>(
      "update_close_preference",
      {
        closeBehavior: behavior,
        closeToTrayNoticeShown: noticeShown,
      },
      true,
    );
    if (result) options.applyLoadedConfig(result);
  }

  return {
    get closePromptVisible() { return closePromptVisible; },
    showClosePrompt,
    minimize,
    toggleMaximize,
    close,
    confirmClosePrompt,
    closeWithoutFuturePrompt,
    exitFromPrompt,
  };
}
