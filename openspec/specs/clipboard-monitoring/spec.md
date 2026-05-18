# clipboard-monitoring Specification

## Purpose

TBD - created by archiving change 'clipboard-auto-detection'. Update Purpose after archive.

## Requirements

### Requirement: Clipboard Content Identification
The system SHALL identify if the current clipboard content is a valid Facebook or Instagram video URL.

#### Scenario: Valid URL detected
- **WHEN** the clipboard contains a URL matching Facebook or Instagram video patterns
- **THEN** the system SHALL mark the content as a "detectable video link"

##### Example: URL patterns
| Input | Result |
|-------|--------|
| https://www.facebook.com/watch?v=123 | detectable |
| https://www.instagram.com/reels/ABC/ | detectable |
| https://google.com | ignored |


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

---
### Requirement: Active Window Focus Detection
The system SHALL check the clipboard content when the application window gains focus to ensure timely detection.

#### Scenario: App window becomes active
- **WHEN** the user switches focus back to the VidBridge window
- **THEN** the system SHALL perform a clipboard scan if the feature is enabled

##### Example: Focus trigger
- **GIVEN** setting "Auto-detect clipboard" is ON
- **WHEN** user copies a Facebook link and brings VidBridge to the foreground
- **THEN** the system SHALL invoke the clipboard check command

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