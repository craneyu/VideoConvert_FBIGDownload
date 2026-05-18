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
The system SHALL allow users to open the containing folder of a downloaded video directly from the history list.

#### Scenario: Opening download folder
- **WHEN** the user clicks "Open Folder" on a completed download item
- **THEN** the system SHALL open the macOS Finder at the target location

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