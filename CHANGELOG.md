# Changelog

All notable changes to VoxType are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- Added a Windows GitHub Actions CI workflow for frontend checks, secret scanning, Rust formatting, clippy, and tests.

### Changed

- Moved shared frontend application types out of `src/routes/+page.svelte` into `src/lib/types/app.ts` as a first low-risk step toward page/component splitting.
- Added focused Rust regression tests for empty ASR result handling, processing-phase session toggles, and invalid audio sample-rate validation.
- Added doc comments to high-risk configuration, session, and clipboard output boundaries.

## [0.1.21] - 2026-04-26

### Changed

- Automatic hotword status no longer exposes the local history file path to the frontend.
- Automatic hotword generation failures no longer carry raw service response bodies through the error handling path.
- Hotword history tests now clean up their temporary directories.

