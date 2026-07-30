## MODIFIED Requirements

### Requirement: Format Conversion

The system SHALL support converting videos with user-defined options (Preset, Resolution, Codec) passed from the UI. When the selected codec is H.265 and the output container is MP4, the system SHALL set the `hvc1` codec tag on the video stream so that the output plays in macOS QuickTime. The system SHALL state the audio encoder explicitly in the conversion command rather than relying on the output container's default encoder.

#### Scenario: Custom conversion with options

- **WHEN** user selects a file and sets codec to H.265
- **THEN** the system SHALL include `-c:v libx265` in the `ffmpeg` command

#### Scenario: H.265 output carries a compatible container tag

- **WHEN** the selected codec is H.265 and the output file is an MP4
- **THEN** the system SHALL include the `hvc1` video codec tag in the `ffmpeg` command
- **AND** the resulting file SHALL play in macOS QuickTime

#### Scenario: Audio encoder is stated explicitly

- **WHEN** any transcoding job is built
- **THEN** the `ffmpeg` command SHALL name the audio encoder explicitly
