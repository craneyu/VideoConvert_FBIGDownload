# download-ui-integration Specification

## Purpose

TBD - created by archiving change 'video-download-history'. Update Purpose after archive.

## Requirements

### Requirement: Download Management UI
The UI SHALL provide an input field for video URLs and a dedicated section for viewing download history.

#### Scenario: Starting a download via UI
- **WHEN** the user enters a URL and clicks the "Download" button
- **THEN** a new task SHALL appear in the active downloads list with a progress bar

---
### Requirement: Local File Access

The system SHALL allow users to open the containing folder of a downloaded video directly from the history list on every platform the project publishes installers for. The reveal command SHALL NOT report success on a platform where it performs no action.

#### Scenario: Opening download folder on macOS

- **WHEN** the user clicks "Open Folder" on a completed download item on macOS
- **THEN** the system SHALL open Finder at the target location with the file selected

#### Scenario: Opening download folder on Windows

- **WHEN** the user clicks "Open Folder" on a completed download item on Windows
- **THEN** the system SHALL open File Explorer at the target location with the file selected

#### Scenario: Opening download folder on Linux

- **WHEN** the user clicks "Open Folder" on a completed download item on Linux
- **THEN** the system SHALL open the desktop file manager at the containing directory

#### Scenario: Unsupported platform reports failure

- **WHEN** the reveal action cannot be performed on the running platform
- **THEN** the system SHALL return an error instead of reporting success

---
### Requirement: Detected URL Prompt
The system SHALL display a non-intrusive prompt or banner when a valid video URL is detected in the clipboard.

#### Scenario: User accepts detected URL
- **WHEN** a URL is detected and the user clicks the "Use this link" action in the prompt
- **THEN** the system SHALL auto-fill the download input field with the detected URL

<!-- @trace
source: clipboard-auto-detection
updated: 2026-05-18
code:
  - src-tauri/Cargo.toml
  - .spectra.yaml
  - src-tauri/src/commands/utils.rs
  - CLAUDE.md
  - src/lib/stores/settings.svelte.ts
  - GEMINI.md
  - src/routes/+page.svelte
  - src-tauri/src/commands/settings.rs
  - src/routes/settings/+page.svelte
  - src-tauri/src/lib.rs
-->
