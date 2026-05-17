## MODIFIED Requirements

### Requirement: Transcoding Quality Presets
The system SHALL provide quality presets (High, Balanced, Fast) that map to specific `ffmpeg` CRF and preset values, with the default preset being retrieved from the global settings system.

#### Scenario: Selecting High Quality
- **WHEN** user selects "High Quality" preset
- **THEN** the system SHALL use `-crf 18` and `-preset slow` in the `ffmpeg` command

### Requirement: Advanced Resolution Control
The system SHALL allow users to specify a target height (e.g., 720, 1080) for the output video, defaulting to the value stored in global settings.

#### Scenario: Scaling to 720p
- **WHEN** user sets resolution to 720p
- **THEN** the system SHALL include `-vf scale=-2:720` in the `ffmpeg` command