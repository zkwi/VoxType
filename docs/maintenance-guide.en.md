# VoxType maintenance guide

This document is for day-to-day maintenance and small release iterations. The goal is to help maintainers quickly decide which module a change belongs in, and which boundaries must not be casually crossed.

## Locate the path before you change code

VoxType's core path is:

```text
trigger recording -> microphone capture -> ASR provider -> optional LLM polish -> clipboard output -> stats and local history
```

Before editing, decide whether the change touches any of these areas:

- ASR provider, audio chunking, final-packet selection, empty-recognition handling.
- LLM trigger conditions, prompt assembly, recent context, or screen OCR reference text.
- Clipboard write, auto-paste, original-clipboard restore.
- Logs, diagnostic reports, statistics, local transcript history.
- Hotkeys, tray, window-close behavior.

If you touch these areas, read [ASR quality and latency guardrails](asr-quality-latency-guardrails.md) first, then decide the test scope.

## ASR provider boundaries

`src-tauri/src/asr_provider.rs` is the unified entry point. It is responsible for only three things:

- Resolve the current provider.
- Run start-up configuration gates.
- Hand recording-session parameters to the concrete provider.

Doubao protocol details live under `src-tauri/src/asr_ws/`. Alibaba Cloud FunASR protocol details live in `aliyun_asr.rs`. Do not push provider-specific WebSocket payloads, event parsing, or final-text selection logic back into `asr_provider.rs`.

The Doubao ASR directory is split by responsibility: `worker.rs` only orchestrates ASR, LLM, and output; `session.rs` handles the Doubao WebSocket session loop; `audio_stream.rs` handles the audio queue and send pacing; `connection.rs` handles connection tests and handshake; `final_text.rs` handles final-text selection; `partial_text.rs` handles live-caption throttling; `output.rs` handles final output events and side effects; `errors.rs` handles error classification. When adding Doubao behavior, put it in the matching module first — do not pile logic back into `mod.rs`.

When adding a provider, prefer the current lightweight dispatch style. Only introduce a trait or heavier abstraction once provider count and shared behavior are clearly duplicated.

## Final-text gates

Live captions and final text must be handled separately:

- Doubao: intermediate packets only update captions; final output waits for the final packet and second-pass sentence selection.
- Alibaba Cloud: `result-generated` only updates captions; final output must wait for `task-finished`.

No intermediate text may trigger LLM polish, paste, success stats, recent context, or automatic hotword history. An empty final text must enter the failure path.

## Settings page maintenance

The settings page should stay approachable for ordinary users, with advanced parameters available for repair:

- High-frequency, required fields are shown directly.
- Low-frequency, protocol, compatibility, or troubleshooting fields prefer a collapsed section.
- Advanced sections that are enabled, non-default, or have validation errors must auto-expand.
- Field-validation navigation is owned by `src/lib/utils/settingsFields.ts`; panel ids must match the component `id` values.
- Reuse `src/lib/components/common/AdvancedSettings.svelte` for collapsible panels — do not reimplement the same DOM/CSS on each page.
- Reuse `src/lib/components/common/ActionPanel.svelte` for "description + metadata + action buttons" cards; pages keep only business buttons and state checks.

When adding a settings field, follow the configuration-sync checklist in `AGENTS.md`. Do not change only Rust or only the frontend.

## Privacy and diagnostics

By default, never write any of the following into logs, diagnostic reports, release audits, or screenshots:

- Real credentials / API keys.
- Recognition transcript body.
- Hotwords, prompts, or recent-context body text.
- Screen OCR body text.
- Windows username paths.

Statistics store non-body metrics only. Even when recent context and automatic hotword history are enabled, they may only enter their own local data files — never write them back into `config.toml`.

## Clipboard and background cost

Clipboard code lives only in `text_output.rs`. Keep two constraints intact:

- The clipboard must be opened with an owner window that belongs to the calling thread; do not fall back to `OpenClipboard(NULL)`. A NULL open can be taken over by any other program that also passes NULL, which surfaces as "Thread does not have a clipboard open" on the next read.
- Before reporting "some formats were not backed up", exclude formats that Windows re-synthesizes from a captured one. Otherwise every dictation reports a false warning whenever a screenshot is on the clipboard.

Each has a manual regression test that rewrites the real system clipboard and restores it afterwards. They are ignored by default; run them after touching clipboard logic:

```powershell
cargo test --lib real_clipboard -- --ignored --nocapture --test-threads=1
```

VoxType lives in the tray, and a hidden WebView keeps running. When adding a timer, a poll, or a looping animation, make sure it stops while the window is hidden. To check background cost, look at the CPU-time delta of the app and its WebView processes; it should be close to zero when idle.

Check the main process as well: look at per-thread CPU time and context switches per second; an idle main thread should show single digits. The `tray-icon` library's "mouse left the icon" timer has an upstream defect and may never stop, which shows up as the main thread sitting at over two hundred switches per second and about 0.75% of one core. `tray.rs` stops that timer once the tray icon has been quiet for 2 seconds and writes an info log line. The workaround depends on the library's window class name `tray_icon_app`. The timer id has changed between versions (`6008` in 0.24.1, `6007` in 0.25.1), so the cleanup covers the whole `6000-6031` range. After a Tauri upgrade changes the `tray-icon` version, check the new source to confirm the class name is unchanged and the timer id is still inside that range. Upstream tracks the defect in [tauri-apps/tray-icon#292](https://github.com/tauri-apps/tray-icon/issues/292) with a fix in [#371](https://github.com/tauri-apps/tray-icon/pull/371); remove the workaround once the fix arrives through a Tauri upgrade.

Every log line carries a millisecond timestamp, so the time between two stages (for example "stop requested" to "paste shortcut sent") is the difference between two lines.

## Pre-release checks

Day-to-day changes:

```powershell
npm run test:unit
npm run ai:check
```

Before a release:

```powershell
npm run ai:release-check
npx tauri build
```

`ai:release-check` first confirms the debug EXE is not locked by a running VoxType instance, then covers the day-to-day checks, npm audit, Rust audit, clippy, and a Tauri debug build. It cannot cover real installation, the WebView2 bootstrapper, first launch, or uninstall; run the [installer smoke-test checklist](installer-smoke-test.md) on a clean virtual machine for those. If the preflight reports a file lock, close the debug app from this session and retry — do not wait until the final Tauri build to debug it. GitHub Actions CI reuses the same entry point; if local release checks fail, do not push a release branch.

Treat test evidence as three separate layers — do not mix them:

- Unit/governance tests use synthetic or local temporary data only and must not call providers.
- API-settings ASR connection tests use real credentials and send a program-generated short silence packet to the selected service, but do not open the microphone.
- Real recording regressions capture and send live microphone audio; only record them as completed when the change truly touches capture or the full ASR main path and a maintainer explicitly ran them.

Release version numbers should reflect impact:

- patch: pure maintenance, docs, small fixes, narrow copy changes.
- minor: user-visible features, clear UX adjustments, default-policy changes.
- major: breaking compatibility or requiring users to relearn the core workflow.

On release, keep `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src-tauri/tauri.conf.json`, `CHANGELOG.md`, `docs/audits/`, and the current release-audit entry in `docs/README.md` in sync.

The online GitHub Wiki is a separate git repository and is not updated by merges into the main repository, yet the setup guide the app opens on first launch is an online wiki page. After a release is merged, publish `docs/wiki/`; append `-- -DryRun` to preview the difference first. Wiki drafts must use absolute URLs when they reference other files in the repository: relative links stop working once published, and the governance check rejects them:

```powershell
npm run wiki:publish
```

## Configuration sync, secrets scan, and rollback

Before merging maintainer changes that touch settings or release metadata:

```powershell
npm run check:governance
npm run scan:secrets
```

`check:governance` validates docs/governance consistency used by CI. `scan:secrets` must stay clean — never commit transcripts, credentials, hotwords, prompts, OCR text, recent context, logs, statistics dumps, or Windows username paths.

If a release build or publish is bad:

1. Stop distributing the bad artifact (GitHub Release / installer channel).
2. Prefer a forward fix release over rewriting published tags.
3. Record the incident under `docs/audits/` and link it from `docs/README.md`.
4. Re-run `npm run ai:release-check` on the fix branch before tagging again.

Chinese original: [maintenance-guide.md](maintenance-guide.md).