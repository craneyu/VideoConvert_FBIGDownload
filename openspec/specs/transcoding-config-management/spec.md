# transcoding-config-management Specification

## Purpose

TBD - created by archiving change 'transcoding-settings-optimization'. Update Purpose after archive.

## Requirements

### Requirement: Transcoding Quality Presets
The system SHALL provide quality presets (High, Balanced, Fast) that map to specific `ffmpeg` CRF and preset values, with the default preset being retrieved from the global settings system.

#### Scenario: Selecting High Quality
- **WHEN** user selects "High Quality" preset
- **THEN** the system SHALL use `-crf 18` and `-preset slow` in the `ffmpeg` command


<!-- @trace
source: implement-settings-system
updated: 2026-05-17
code:
  - src-tauri/src/lib.rs
  - src/routes/settings/+page.svelte
  - .spectra.yaml
  - src/routes/+layout.svelte
  - src-tauri/src/commands/download.rs
  - CLAUDE.md
  - src-tauri/src/commands/mod.rs
  - src/routes/+page.svelte
  - src-tauri/Cargo.toml
  - src-tauri/src/commands/settings.rs
  - src/lib/stores/settings.svelte.ts
  - GEMINI.md
-->

---
### Requirement: Advanced Resolution Control
The system SHALL allow users to specify a target height (e.g., 720, 1080) for the output video, defaulting to the value stored in global settings.

#### Scenario: Scaling to 720p
- **WHEN** user sets resolution to 720p
- **THEN** the system SHALL include `-vf scale=-2:720` in the `ffmpeg` command

<!-- @trace
source: implement-settings-system
updated: 2026-05-17
code:
  - src-tauri/src/lib.rs
  - src/routes/settings/+page.svelte
  - .spectra.yaml
  - src/routes/+layout.svelte
  - src-tauri/src/commands/download.rs
  - CLAUDE.md
  - src-tauri/src/commands/mod.rs
  - src/routes/+page.svelte
  - src-tauri/Cargo.toml
  - src-tauri/src/commands/settings.rs
  - src/lib/stores/settings.svelte.ts
  - GEMINI.md
-->