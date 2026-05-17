# video-download-engine Specification

## Purpose

TBD - created by archiving change 'video-download-history'. Update Purpose after archive.

## Requirements

### Requirement: Video Metadata Fetching
The system SHALL be able to retrieve video title and other metadata using `yt-dlp` before starting a download.

#### Scenario: Fetching metadata successfully
- **WHEN** the user inputs a valid public video URL
- **THEN** the system SHALL display the video title fetched from `yt-dlp`

---
### Requirement: Progressive Downloading
The system SHALL execute `yt-dlp` to download videos and report real-time progress.

#### Scenario: Downloading with progress updates
- **WHEN** a download starts
- **THEN** the system SHALL emit progress events containing percentage and speed

##### Example: Progress update event
- **GIVEN** yt-dlp outputs "[download]  25.0% of 10.00MiB at  1.23MiB/s"
- **WHEN** the Rust backend parses this line
- **THEN** it SHALL emit a `download-progress` event with payload `{ "id": "task1", "progress": 25.0, "speed": "1.23MiB/s" }`
