## ADDED Requirements

### Requirement: Download Management UI
The UI SHALL provide an input field for video URLs and a dedicated section for viewing download history.

#### Scenario: Starting a download via UI
- **WHEN** the user enters a URL and clicks the "Download" button
- **THEN** a new task SHALL appear in the active downloads list with a progress bar

### Requirement: Local File Access
The system SHALL allow users to open the containing folder of a downloaded video directly from the history list.

#### Scenario: Opening download folder
- **WHEN** the user clicks "Open Folder" on a completed download item
- **THEN** the system SHALL open the macOS Finder at the target location