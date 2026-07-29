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

The system MUST define and use default values for all supported settings when they are missing from the database. When a stored value is present but falls outside the set of values the system recognizes for that key, the system MUST treat it as missing and use the default.

#### Scenario: First run default values

- **WHEN** `get_settings` is called and the database is empty
- **THEN** the system SHALL return a JSON object containing all default values

#### Scenario: Unrecognized stored value falls back to the default

- **WHEN** `get_settings` is called and the stored `theme` value is not one of `system`, `light`, or `dark`
- **THEN** the returned object SHALL report `theme` as `system`, and the system SHALL NOT write a corrected value back to the database

##### Example: Default Values Mapping

| Setting Key        | Default Value          | Notes                                        |
| ------------------ | ---------------------- | -------------------------------------------- |
| download_path      | (User Home)/Downloads  | Platform-specific home dir                   |
| transcoding_preset | 'Balanced'             | Mapping to CRF 23, preset medium             |
| auto_organize      | false                  | Boolean flag                                 |
| detect_clipboard   | true                   | Boolean flag                                 |
| theme              | 'system'               | One of 'system', 'light', 'dark'; preserves pre-existing behavior of following the OS color scheme |

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

---
### Requirement: Theme Setting Key

The system SHALL store the selected theme mode in the `settings` table under the key `theme`, using the existing key-value format. Because the table is key-value shaped, introducing this key SHALL NOT require a database migration.

The generic `update_setting` command SHALL remain free of per-key validation; validation of the `theme` value SHALL happen where stored settings are merged with defaults, so that the same fallback applies regardless of how the value came to be in the database.

#### Scenario: Persisting a theme selection

- **WHEN** the user selects the `dark` theme mode in the settings interface
- **THEN** the system SHALL invoke `update_setting` with the key `theme` and the value `dark`

#### Scenario: Upgrading an existing installation

- **WHEN** the application starts against a database created before the `theme` key existed
- **THEN** `get_settings` SHALL succeed and report `theme` as `system`, without any schema change being applied
