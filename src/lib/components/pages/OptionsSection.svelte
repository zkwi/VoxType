<script lang="ts">
  import { onDestroy } from "svelte";
  import type {
    AppConfig,
    AudioDeviceInfo,
    HotkeyCaptureState,
    SoftConfigNoticeKey,
    UpdateStatus,
  } from "$lib/types/app";
  import ActionPanel from "$lib/components/common/ActionPanel.svelte";
  import AdvancedSettings from "$lib/components/common/AdvancedSettings.svelte";
  import SettingsJumpNav from "$lib/components/common/SettingsJumpNav.svelte";
  import type { CopyKey, Language } from "$lib/i18n";
  import { ClipboardCopy, Download, FileText, Keyboard, RefreshCw, ShieldCheck, Trash2 } from "lucide-svelte";

  type Translate = (key: CopyKey, values?: Record<string, string>) => string;
  type OverlayColorPreset = { label: CopyKey; background: string; text: string };
  type OptionJumpTarget = { id: string; label: CopyKey };

  const optionJumpTargets: OptionJumpTarget[] = [
    { id: "settings-output", label: "usageModeTitle" },
    { id: "settings-audio", label: "microphoneTitle" },
    { id: "settings-basic-output", label: "inputResultTitle" },
    { id: "settings-overlay", label: "floatingCaptionAppearance" },
    { id: "settings-window", label: "windowBehaviorTitle" },
    { id: "settings-update", label: "updatesAndDiagnostics" },
  ];

  type Props = {
    config: AppConfig;
    t: Translate;
    language: Language;
    onSetLanguage: (value: string) => void;
    requestedPanel: { id: string } | null;
    hotkeyCaptureState: HotkeyCaptureState;
    hotkeyValidationMessage: string;
    overlayColorPresets: OverlayColorPreset[];
    overlayOpacityPresets: readonly number[];
    audioDevices: AudioDeviceInfo[];
    refreshingAudioDevices: boolean;
    updateStatus: UpdateStatus | null;
    checkingUpdate: boolean;
    installingUpdate: boolean;
    openingLog: boolean;
    copyingDiagnosticReport: boolean;
    fieldError: (field: string) => string;
    formatHotkey: (value: string) => string;
    overlayBackgroundRgb: () => string;
    overlayOpacity: () => number;
    overlayTextColor: () => string;
    overlayPresetActive: (background: string, text: string) => boolean;
    overlayOpacityPresetActive: (opacity: number) => boolean;
    overlayOpacityLabel: (opacity: number) => string;
    updatePanelTitle: () => string;
    updatePanelDescription: () => string;
    updateMetaText: () => string;
    onHotkeyKeydown: (event: KeyboardEvent) => void;
    onBeginHotkeyCapture: () => void;
    onCancelHotkeyCapture: () => void;
    onOptionEnabledNotice: (key: SoftConfigNoticeKey, enabled: boolean) => void;
    onApplyOverlayPreset: (background: string, text: string) => void;
    onApplyOverlayOpacity: (opacity: number) => void;
    onSetInputDevice: (value: string) => void;
    onRefreshAudioDevices: () => void;
    onCheckUpdate: (manual: boolean) => void;
    onDownloadLatestUpdate: () => void;
    onOpenLog: () => void;
    onCopyDiagnosticReport: () => void;
    onScrollToSettingsPanel: (id: string) => void;
  };

  let {
    config = $bindable<AppConfig>(),
    t,
    language,
    onSetLanguage,
    requestedPanel,
    hotkeyCaptureState,
    hotkeyValidationMessage,
    overlayColorPresets,
    overlayOpacityPresets,
    audioDevices,
    refreshingAudioDevices,
    updateStatus,
    checkingUpdate,
    installingUpdate,
    openingLog,
    copyingDiagnosticReport,
    fieldError,
    formatHotkey,
    overlayBackgroundRgb,
    overlayOpacity,
    overlayTextColor,
    overlayPresetActive,
    overlayOpacityPresetActive,
    overlayOpacityLabel,
    updatePanelTitle,
    updatePanelDescription,
    updateMetaText,
    onHotkeyKeydown,
    onBeginHotkeyCapture,
    onCancelHotkeyCapture,
    onOptionEnabledNotice,
    onApplyOverlayPreset,
    onApplyOverlayOpacity,
    onSetInputDevice,
    onRefreshAudioDevices,
    onCheckUpdate,
    onDownloadLatestUpdate,
    onOpenLog,
    onCopyDiagnosticReport,
    onScrollToSettingsPanel,
  }: Props = $props();

  onDestroy(() => onCancelHotkeyCapture());

  let selectedInputDeviceValue = $derived.by(() => {
    const configuredName = config.audio.input_device_name?.trim();
    if (config.audio.input_device !== null && config.audio.input_device !== undefined) {
      const indexedDevice = audioDevices.find((device) => device.index === config.audio.input_device);
      if (indexedDevice && (!configuredName || indexedDevice.name.trim().toLowerCase() === configuredName.toLowerCase())) {
        return String(indexedDevice.index);
      }
    }
    if (configuredName) {
      const namedDevices = audioDevices.filter((device) => device.name.trim().toLowerCase() === configuredName.toLowerCase());
      if (namedDevices.length === 1) return String(namedDevices[0].index);
      return "";
    }
    return "";
  });

  let showBackupTriggers = $state(false);
  let showRecordingTroubleshooting = $state(false);
  $effect(() => {
    if (requestedPanel?.id === "settings-backup-triggers") showBackupTriggers = true;
    if (requestedPanel?.id === "settings-recording-troubleshooting") showRecordingTroubleshooting = true;
  });
  let backupTriggersActive = $derived(Boolean(
    config.triggers.middle_mouse_enabled ||
      config.triggers.right_alt_enabled ||
      config.triggers.right_ctrl_enabled,
  ));
  let recordingTroubleshootingActive = $derived(Boolean(
    config.asr.no_feedback_auto_stop_seconds !== 30 ||
      config.audio.input_gain_db !== 0 ||
      fieldError("asr.no_feedback_auto_stop_seconds") ||
      fieldError("audio.input_gain_db"),
  ));
  let backupTriggersExpanded = $derived(showBackupTriggers || backupTriggersActive);
  let recordingTroubleshootingExpanded = $derived(showRecordingTroubleshooting || recordingTroubleshootingActive);

  function handleHotkeyRecorderKeydown(event: KeyboardEvent) {
    if (hotkeyCaptureState === "idle" && (event.key === "Enter" || event.key === " ")) {
      event.preventDefault();
      onBeginHotkeyCapture();
      return;
    }
    onHotkeyKeydown(event);
  }
</script>

<section class="settings-stack">
  <section class="settings-group">
    <div class="settings-group-heading">
      <h3>{t("navOptions")}</h3>
      <p>{t("optionsPageDescription")}</p>
    </div>
    <SettingsJumpNav targets={optionJumpTargets} {t} onJump={onScrollToSettingsPanel} />
    <div class="settings-cluster-heading">
      <span>{t("optionsEssentialsTitle")}</span>
    </div>
    <div id="settings-output" class="form-panel">
      <div class="section-heading"><h3>{t("usageModeTitle")}</h3><p>{t("usageModeDescription")}</p></div>
      <label class="check"><input type="checkbox" bind:checked={config.triggers.hotkey_enabled} />{t("enableMainHotkey")}</label>
      <div class="form-grid">
        <label class:field-invalid={Boolean(fieldError("hotkey") || hotkeyValidationMessage)}>
          <span>{t("hotkey")}</span>
          <div class:recording={hotkeyCaptureState !== "idle"} class="hotkey-recorder">
            <Keyboard size={16} />
            <input
              id="setting-hotkey"
              data-config-field="hotkey"
              aria-invalid={Boolean(fieldError("hotkey") || hotkeyValidationMessage)}
              aria-describedby="setting-hotkey-help"
              readonly
              onblur={onCancelHotkeyCapture}
              value={hotkeyCaptureState === "idle"
                ? formatHotkey(config.hotkey) || t("hotkeyUnset")
                : t(hotkeyCaptureState === "preparing" ? "hotkeyPreparing" : "hotkeyRecording")}
              onkeydown={handleHotkeyRecorderKeydown}
              onclick={onBeginHotkeyCapture}
            />
          </div>
          <small id="setting-hotkey-help" class:field-error={Boolean(hotkeyValidationMessage || fieldError("hotkey"))} class="field-hint">{hotkeyValidationMessage || fieldError("hotkey") || t("hotkeyRecordHint")}</small>
        </label>
      </div>
      <AdvancedSettings
        rootId="settings-backup-triggers"
        title={t("backupTriggers")}
        description={backupTriggersActive ? t("backupTriggersActiveHint") : t("backupTriggersDescription")}
        expanded={backupTriggersExpanded}
        onToggle={() => (showBackupTriggers = !showBackupTriggers)}
      >
        <div class="toggle-grid">
          <label class="check"><input type="checkbox" bind:checked={config.triggers.middle_mouse_enabled} onchange={(event) => onOptionEnabledNotice("middle_mouse_enabled", event.currentTarget.checked)} /><span class="check-copy"><span>{t("middleMouse")}</span><small>{t("tagConflictRisk")}</small></span></label>
          <label class="check"><input type="checkbox" bind:checked={config.triggers.right_alt_enabled} onchange={(event) => onOptionEnabledNotice("right_alt_enabled", event.currentTarget.checked)} /><span class="check-copy"><span>{t("rightAlt")}</span><small>{t("tagConflictRisk")}</small></span></label>
          <label class="check"><input type="checkbox" bind:checked={config.triggers.right_ctrl_enabled} onchange={(event) => onOptionEnabledNotice("right_ctrl_enabled", event.currentTarget.checked)} /><span class="check-copy"><span>{t("rightCtrl")}</span><small>{t("tagConflictRisk")}</small></span></label>
        </div>
        <p class="field-hint">{t("triggerConflictHint")}</p>
      </AdvancedSettings>
    </div>
    <div id="settings-audio" class="form-panel">
      <div class="section-heading"><h3>{t("microphoneTitle")}</h3><p>{t("microphoneDescription")}</p></div>
      <div class="audio-device-controls">
        <label class:field-invalid={Boolean(fieldError("audio.input_device"))}>
          <span>{t("inputDevice")}</span>
          <select id="setting-audio-input-device" data-config-field="audio.input_device" aria-invalid={Boolean(fieldError("audio.input_device"))} aria-describedby={fieldError("audio.input_device") ? "setting-audio-input-device-error" : undefined} value={selectedInputDeviceValue} onchange={(event) => onSetInputDevice(event.currentTarget.value)}>
            <option value="">{t("defaultInputDevice")}</option>
            {#if audioDevices.length === 0}
              <option value="" disabled>{t("noAudioDevices")}</option>
            {/if}
            {#each audioDevices as device}
              <option value={String(device.index)}>{device.index}: {device.name}</option>
            {/each}
          </select>
        </label>
        <button class="test-button" type="button" disabled={refreshingAudioDevices} onclick={onRefreshAudioDevices}>
          <RefreshCw size={16} />{t(refreshingAudioDevices ? "refreshingAudioDevices" : "refreshAudioDevices")}
        </button>
        {#if fieldError("audio.input_device")}<small id="setting-audio-input-device-error" class="field-error">{fieldError("audio.input_device")}</small>{/if}
      </div>
      <label class="check">
        <input type="checkbox" data-config-field="audio.mute_system_volume_while_recording" bind:checked={config.audio.mute_system_volume_while_recording} />
        <span class="check-copy"><span>{t("muteSystemAudioWhileRecording")}</span><small>{t("muteSystemAudioHint")}</small></span>
      </label>
      <AdvancedSettings
        rootId="settings-recording-troubleshooting"
        title={t("recordingTroubleshooting")}
        description={recordingTroubleshootingActive ? t("recordingTroubleshootingActiveHint") : t("recordingTroubleshootingDescription")}
        expanded={recordingTroubleshootingExpanded}
        onToggle={() => (showRecordingTroubleshooting = !showRecordingTroubleshooting)}
      >
        <div class="form-grid">
          <label class:field-invalid={Boolean(fieldError("asr.no_feedback_auto_stop_seconds"))}>
            <span>{t("asrNoFeedbackAutoStopSeconds")}</span>
            <input type="number" min="0" max="300" step="1" data-config-field="asr.no_feedback_auto_stop_seconds" aria-invalid={Boolean(fieldError("asr.no_feedback_auto_stop_seconds"))} bind:value={config.asr.no_feedback_auto_stop_seconds} />
            {#if fieldError("asr.no_feedback_auto_stop_seconds")}<small class="field-error">{fieldError("asr.no_feedback_auto_stop_seconds")}</small>{/if}
          </label>
          <label class:field-invalid={Boolean(fieldError("audio.input_gain_db"))}>
            <span>{t("inputGainDb")}</span>
            <input type="number" min="-12" max="24" step="1" data-config-field="audio.input_gain_db" aria-invalid={Boolean(fieldError("audio.input_gain_db"))} bind:value={config.audio.input_gain_db} />
            {#if fieldError("audio.input_gain_db")}<small class="field-error">{fieldError("audio.input_gain_db")}</small>{/if}
          </label>
        </div>
        <p class="field-hint">{t("asrNoFeedbackAutoStopHint")}</p>
        <p class="field-hint">{t("inputGainHint")}</p>
      </AdvancedSettings>
    </div>
    <div id="settings-basic-output" class="form-panel">
      <div class="section-heading"><h3>{t("inputResultTitle")}</h3><p>{t("inputResultDescription")}</p></div>
      <div class="form-grid">
        <label class:field-invalid={Boolean(fieldError("typing.paste_method"))}>
          <span>{t("pasteMethod")}</span>
          <select id="setting-paste-method" data-config-field="typing.paste_method" aria-invalid={Boolean(fieldError("typing.paste_method"))} aria-describedby={fieldError("typing.paste_method") ? "setting-paste-method-error" : undefined} bind:value={config.typing.paste_method}>
            <option value="ctrl_v">Ctrl + V</option>
            <option value="shift_insert">Shift + Insert</option>
            <option value="clipboard_only">{t("clipboardOnly")}</option>
          </select>
          {#if fieldError("typing.paste_method")}<small id="setting-paste-method-error" class="field-error">{fieldError("typing.paste_method")}</small>{/if}
        </label>
      </div>
      <div class="toggle-grid">
        <label class="check"><input type="checkbox" bind:checked={config.typing.remove_trailing_period} />{t("removeTrailingPeriod")}</label>
        <label class="check"><input type="checkbox" bind:checked={config.typing.restore_clipboard_after_paste} />{t("restoreClipboardAfterPaste")}</label>
      </div>
      <p class="field-hint">{t("removeTrailingPeriodHint")}</p>
      <p class="field-hint">{t("clipboardTextRestoreHint")}</p>
    </div>
    <div class="settings-cluster-heading">
      <span>{t("optionsMaintenanceTitle")}</span>
    </div>
    <div id="settings-overlay" class="form-panel">
      <div class="section-heading"><h3>{t("floatingCaptionAppearance")}</h3><p>{t("floatingCaptionAppearanceDescription")}</p></div>
      <div class="caption-theme-panel">
        <div class="caption-theme-head">
          <div>
            <strong>{t("captionColors")}</strong>
            <span>{t("captionColorsDescription")}</span>
          </div>
          <div class="caption-preview" style={`--preview-bg-rgb: ${overlayBackgroundRgb()}; --preview-opacity: ${overlayOpacity()}; --preview-text: ${overlayTextColor()};`}>
            {t("captionPreviewText")}
          </div>
        </div>
        <div class="preset-row">
          {#each overlayColorPresets as preset}
            <button
              type="button"
              class:active={overlayPresetActive(preset.background, preset.text)}
              aria-pressed={overlayPresetActive(preset.background, preset.text)}
              onclick={() => onApplyOverlayPreset(preset.background, preset.text)}
            >
              <span class="preset-swatch" style={`--preset-bg: ${preset.background}; --preset-text: ${preset.text};`}>Aa</span>
              <span>{t(preset.label)}</span>
            </button>
          {/each}
        </div>
        <div class="caption-opacity-row" class:field-invalid={Boolean(fieldError("ui.opacity"))} role="group" aria-describedby={fieldError("ui.opacity") ? "setting-caption-opacity-error" : undefined}>
          <div>
            <strong>{t("captionOpacity")}</strong>
            <span>{t("captionOpacityDescription")}</span>
          </div>
          <div class="preset-row opacity-preset-row">
            {#each overlayOpacityPresets as opacity}
              <button
                type="button"
                data-config-field="ui.opacity"
                aria-describedby={fieldError("ui.opacity") ? "setting-caption-opacity-error" : undefined}
                class:active={overlayOpacityPresetActive(opacity)}
                aria-pressed={overlayOpacityPresetActive(opacity)}
                onclick={() => onApplyOverlayOpacity(opacity)}
              >
                {overlayOpacityLabel(opacity)}
              </button>
            {/each}
          </div>
          {#if fieldError("ui.opacity")}<small id="setting-caption-opacity-error" class="field-error">{fieldError("ui.opacity")}</small>{/if}
        </div>
      </div>
    </div>
    <div id="settings-window" class="form-panel">
      <div class="section-heading"><h3>{t("windowBehaviorTitle")}</h3><p>{t("windowBehaviorDescription")}</p></div>
      <div class="form-grid">
        <label class:field-invalid={Boolean(fieldError("tray.close_behavior"))}>
          <span>{t("closeBehavior")}</span>
          <select bind:value={config.tray.close_behavior}>
            <option value="close_to_tray">{t("closeBehaviorCloseToTray")}</option>
            <option value="direct_exit">{t("closeBehaviorDirectExit")}</option>
            <option value="ask_every_time">{t("closeBehaviorAskEveryTime")}</option>
          </select>
          {#if fieldError("tray.close_behavior")}<small class="field-error">{fieldError("tray.close_behavior")}</small>{/if}
        </label>
        <label>
          <span>{t("language")}</span>
          <select value={language} onchange={(event) => onSetLanguage(event.currentTarget.value)}>
            <option value="zh-CN">简体中文</option>
            <option value="zh-TW">繁體中文</option>
            <option value="en">English</option>
          </select>
        </label>
      </div>
      <p class="field-hint">{t("closeBehaviorHint")}</p>
      <div class="toggle-grid">
        <label class="check"><input type="checkbox" bind:checked={config.startup.launch_on_startup} />{t("launchOnStartup")}</label>
      </div>
      <p class="field-hint">{t("launchOnStartupHint")}</p>
    </div>
    <div id="settings-update" class="form-panel update-panel">
      <div class="section-heading"><h3>{t("updatesAndDiagnostics")}</h3><p>{t("updatesAndDiagnosticsDescription")}</p></div>
      <ActionPanel
        title={updatePanelTitle()}
        description={updatePanelDescription()}
        meta={updateMetaText()}
        available={Boolean(updateStatus?.update_available)}
      >
        {#snippet actions()}
          <button type="button" onclick={() => onCheckUpdate(true)} disabled={checkingUpdate}>
            <ShieldCheck size={16} />{checkingUpdate ? t("checkingUpdates") : t("checkUpdates")}
          </button>
          {#if updateStatus?.update_available && updateStatus.asset_name}
            <button type="button" class="primary" onclick={onDownloadLatestUpdate} disabled={installingUpdate}>
              <Download size={16} />{installingUpdate ? t("downloadingInstall") : t("updateNow")}
            </button>
          {/if}
        {/snippet}
      </ActionPanel>
      <div class="toggle-grid">
        <label class="check"><input type="checkbox" bind:checked={config.update.auto_check_on_startup} />{t("autoCheckUpdates")}</label>
      </div>
      <div id="settings-diagnostics" class="subsection-panel">
        <div class="subsection-heading">
          <strong>{t("diagnosticsAndLogs")}</strong>
          <span>{t("diagnosticsDescription")}</span>
        </div>
        <ActionPanel title={t("logStatusTitle")} description={t("logStatusDescription")}>
          {#snippet actions()}
            <button type="button" onclick={onOpenLog} disabled={openingLog}>
              <FileText size={16} />{openingLog ? t("openingLog") : t("openLog")}
            </button>
            <button type="button" onclick={onCopyDiagnosticReport} disabled={copyingDiagnosticReport}>
              <ClipboardCopy size={16} />{copyingDiagnosticReport ? t("copyingReport") : t("copyDiagnosticReport")}
            </button>
          {/snippet}
        </ActionPanel>
      </div>
    </div>
  </section>
</section>

<style>
  .audio-device-controls {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: end;
    gap: 8px 12px;
  }

  @media (max-width: 520px) {
    .audio-device-controls {
      grid-template-columns: minmax(0, 1fr);
    }
  }

  .settings-cluster-heading {
    display: grid;
    gap: 4px;
    padding: 4px 4px 0;
  }

  .settings-cluster-heading span {
    color: var(--primary);
    font-size: 13px;
    font-weight: 900;
  }

  .subsection-heading {
    display: grid;
    gap: 3px;
    padding-top: 2px;
  }

  .subsection-heading strong {
    color: var(--text-main);
    font-size: 14px;
    font-weight: 800;
  }

  .subsection-heading span {
    color: var(--text-secondary);
    font-size: 13px;
    line-height: 1.45;
  }

  .subsection-panel {
    display: grid;
    gap: 12px;
    min-width: 0;
    padding-top: 14px;
    border-top: 1px solid var(--border);
  }

  .hotkey-recorder {
    display: inline-flex;
    align-items: center;
    justify-content: flex-start;
    gap: 9px;
    width: 100%;
    min-height: 38px;
    padding: 0 12px;
    color: var(--text-main);
    background: #ffffff;
    border: 1px solid var(--border);
    border-radius: 10px;
    text-align: left;
  }

  .hotkey-recorder input {
    min-width: 0;
    width: 100%;
    overflow: hidden;
    color: inherit;
    background: transparent;
    border: 0;
    outline: 0;
    font-size: 14px;
    font-weight: 800;
    text-overflow: ellipsis;
    white-space: nowrap;
    cursor: pointer;
  }

  .hotkey-recorder :global(svg) {
    flex: 0 0 auto;
    color: var(--primary);
  }

  .hotkey-recorder.recording {
    border-color: var(--primary);
    background: var(--primary-light);
    box-shadow: 0 0 0 3px rgba(47, 128, 237, 0.14);
  }

  .field-invalid .hotkey-recorder {
    border-color: var(--danger);
    background: #fff7f7;
  }

  .caption-theme-panel {
    display: grid;
    gap: 12px;
    padding: 14px;
    background: #f8fbff;
    border: 1px solid var(--border);
    border-radius: 14px;
  }

  .caption-theme-head {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 12px;
  }

  .caption-theme-head > div:first-child {
    display: grid;
    gap: 4px;
    min-width: 0;
  }

  .caption-theme-head strong,
  .caption-opacity-row strong {
    color: var(--text-main);
    font-size: 14px;
    font-weight: 800;
  }

  .caption-theme-head span,
  .caption-opacity-row span {
    color: var(--text-secondary);
    font-size: 13px;
    line-height: 1.45;
    overflow-wrap: anywhere;
  }

  .caption-preview {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 168px;
    min-height: 36px;
    padding: 0 14px;
    overflow: hidden;
    color: var(--preview-text);
    background: rgba(var(--preview-bg-rgb), var(--preview-opacity));
    border-radius: 10px;
    font-size: 14px;
    font-weight: 700;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .preset-row {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(132px, 100%), 1fr));
    gap: 10px;
  }

  .preset-row button {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    min-height: 42px;
    padding: 6px 10px;
    color: var(--text-main);
    background: #ffffff;
    border: 1px solid var(--border);
    border-radius: 10px;
    font-size: 13px;
    font-weight: 700;
  }

  .preset-row button.active {
    border-color: var(--primary);
    box-shadow: 0 0 0 2px rgba(47, 128, 237, 0.12);
  }

  .preset-row button > span:last-child {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: normal;
  }

  .preset-swatch {
    display: inline-flex;
    flex: 0 0 auto;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 26px;
    color: var(--preset-text);
    background: var(--preset-bg);
    border-radius: 8px;
    font-size: 12px;
    font-weight: 800;
  }

  .caption-opacity-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(220px, 0.8fr);
    align-items: center;
    gap: 12px;
  }

  .caption-opacity-row > div:first-child {
    display: grid;
    gap: 4px;
    min-width: 0;
  }

  .opacity-preset-row {
    grid-template-columns: repeat(auto-fit, minmax(min(112px, 100%), 1fr));
  }

  .opacity-preset-row button {
    justify-content: center;
    padding: 6px 8px;
  }

  @media (max-width: 920px) {
    .preset-row {
      grid-template-columns: repeat(auto-fit, minmax(min(132px, 100%), 1fr));
    }

    .caption-theme-head {
      grid-template-columns: 1fr;
      align-items: stretch;
    }

    .caption-preview {
      width: 100%;
      min-width: 0;
    }
  }

  @media (max-width: 720px) {
    .preset-row {
      grid-template-columns: 1fr;
    }
  }
</style>
