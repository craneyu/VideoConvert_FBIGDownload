## MODIFIED Requirements

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
