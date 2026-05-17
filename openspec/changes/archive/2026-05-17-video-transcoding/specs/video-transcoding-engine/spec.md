## ADDED Requirements

### Requirement: Format Conversion
The system SHALL support converting videos between MP4, AVI, MOV, and MKV formats using `ffmpeg`.

#### Scenario: Successful conversion
- **WHEN** the user selects an input file and a target format
- **THEN** the system SHALL produce a new video file in the target format

### Requirement: Quality Presets
The system SHALL provide at least three quality presets: "High Quality", "Balanced", and "Small Size".

#### Scenario: Choosing a preset
- **WHEN** the user selects the "Small Size" preset
- **THEN** the system SHALL apply lower bitrate settings to the `ffmpeg` command

### Requirement: Progress Parsing
The system SHALL parse `ffmpeg` output to report real-time transcoding progress.

#### Scenario: Transcoding progress updates
- **WHEN** transcoding is in progress
- **THEN** the system SHALL emit events containing the current percentage complete