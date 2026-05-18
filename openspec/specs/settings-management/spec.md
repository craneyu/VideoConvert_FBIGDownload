# settings-management Specification

## Purpose

TBD - created by archiving change 'implement-settings-system'. Update Purpose after archive.

## Requirements

### Requirement: Persistent Settings Storage
The system MUST store application settings in a persistent SQLite database table named `settings` using a key-value format.

#### Scenario: Initial database migration
- **WHEN** the application starts for the first time
- **THEN** the system SHALL create the `settings` table if it does not exist


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
### Requirement: Global Settings Access
The system SHALL provide an IPC command `get_settings` that returns all stored settings as a JSON object.

#### Scenario: Fetching settings on startup
- **WHEN** the frontend initializes
- **THEN** it SHALL invoke `get_settings` to populate the settings store


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
### Requirement: Updating Settings
The system SHALL provide an IPC command `update_setting` that accepts a key and a value to update the persistent store.

#### Scenario: Changing download path
- **WHEN** the user selects a new download directory in the UI
- **THEN** the system SHALL invoke `update_setting` with the key 'download_path' and the new directory path


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
### Requirement: Default Settings Values
The system MUST define and use default values for all supported settings when they are missing from the database.

#### Scenario: First run default values
- **WHEN** `get_settings` is called and the database is empty
- **THEN** the system SHALL return a JSON object containing all default values

##### Example: Default Values Mapping
| Setting Key | Default Value | Notes |
|-------------|---------------|-------|
| download_path | (User Home)/Downloads | Platform-specific home dir |
| transcoding_preset | 'Balanced' | Mapping to CRF 23, preset medium |
| auto_organize | false | Boolean flag |

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
### Requirement: Enhanced Settings Interaction
The settings interface SHALL provide immediate visual feedback for all user interactions to confirm that changes are being registered.

#### Scenario: Toggling a setting
- **WHEN** the user changes a setting (e.g., toggling 'Auto-organize')
- **THEN** the control SHALL animate its state change and provide a subtle background highlight during the interaction.

<!-- @trace
source: beautify-settings-page
updated: 2026-05-18
code:
  - src/routes/+page.svelte
  - src/routes/settings/+page.svelte
  - src-tauri/src/commands/settings.rs
  - src-tauri/src/commands/utils.rs
  - .spectra.yaml
  - src/lib/stores/settings.svelte.ts
  - src-tauri/Cargo.toml
  - CLAUDE.md
  - GEMINI.md
  - src-tauri/src/lib.rs
-->

---
### Requirement: Clipboard Detection Toggle
The system SHALL provide a setting to enable or disable automatic clipboard detection of video URLs.

#### Scenario: Disabling clipboard detection
- **WHEN** the user toggles the "Auto-detect clipboard" setting to OFF
- **THEN** the system SHALL NOT perform clipboard scans even when the window is focused

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