# video-transcoding-engine Specification

## Purpose

TBD - created by archiving change 'video-transcoding'. Update Purpose after archive.

## Requirements

### Requirement: Format Conversion
The system SHALL support converting videos with user-defined options (Preset, Resolution, Codec) passed from the UI.

#### Scenario: Custom conversion with options
- **WHEN** user selects a file and sets codec to H.265
- **THEN** the system SHALL include `-c:v libx265` in the `ffmpeg` command

---
### Requirement: Quality Presets
The system SHALL provide at least three quality presets: "High Quality", "Balanced", and "Small Size".

#### Scenario: Choosing a preset
- **WHEN** the user selects the "Small Size" preset
- **THEN** the system SHALL apply lower bitrate settings to the `ffmpeg` command

---
### Requirement: Progress Parsing
The system SHALL parse `ffmpeg` output to report real-time transcoding progress.

#### Scenario: Transcoding progress updates
- **WHEN** transcoding is in progress
- **THEN** the system SHALL emit events containing the current percentage complete
