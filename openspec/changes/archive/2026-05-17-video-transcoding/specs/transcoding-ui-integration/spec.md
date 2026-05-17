## ADDED Requirements

### Requirement: Batch File Selection
The UI SHALL allow users to select multiple video files for transcoding via a file picker or drag-and-drop.

#### Scenario: Drag and drop files
- **WHEN** the user drops three MP4 files into the application
- **THEN** three new transcoding tasks SHALL appear in the list

### Requirement: Task Dashboard
The UI SHALL display a list of all active and completed transcoding tasks with progress bars and status indicators.

#### Scenario: Viewing task status
- **WHEN** a task completes
- **THEN** its status SHALL change to "Success" and a notification SHALL be triggered