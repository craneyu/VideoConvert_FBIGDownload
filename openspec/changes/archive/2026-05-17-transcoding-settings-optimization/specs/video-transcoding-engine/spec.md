## MODIFIED Requirements

### Requirement: Progressive Downloading
(Content remains same as base spec, adding support for options)

### Requirement: Format Conversion
The system SHALL support converting videos with user-defined options (Preset, Resolution, Codec) passed from the UI.

#### Scenario: Custom conversion with options
- **WHEN** user selects a file and sets codec to H.265
- **THEN** the system SHALL include `-c:v libx265` in the `ffmpeg` command