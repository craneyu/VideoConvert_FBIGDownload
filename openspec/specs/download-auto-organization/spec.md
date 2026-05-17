# download-auto-organization Specification

## Purpose

TBD - created by archiving change 'auto-organization'. Update Purpose after archive.

## Requirements

### Requirement: Source-Based Directory Creation
The system SHALL automatically create a sub-directory named after the video source (e.g., "Facebook", "Instagram") within the configured download directory, provided the 'auto_organize' setting is enabled.

#### Scenario: Creating Facebook directory
- **WHEN** user downloads a Facebook video AND 'auto_organize' is true
- **THEN** the system SHALL ensure the source-specific directory exists within the configured download path


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
### Requirement: Categorized File Saving
The system SHALL save the downloaded video into the source-specific sub-directory if 'auto_organize' is enabled, otherwise it SHALL save directly to the configured download path.

#### Scenario: Saving to specific path
- **WHEN** a download completes and 'auto_organize' is true
- **THEN** the resulting file SHALL be located inside the source-specific sub-directory of the configured download path

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