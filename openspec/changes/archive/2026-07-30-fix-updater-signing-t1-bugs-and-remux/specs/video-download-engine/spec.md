## ADDED Requirements

### Requirement: Post-Download Container Optimization

The system SHALL inspect each downloaded file with `ffprobe` and SHALL decide between container remux and full re-encoding. When the file satisfies the compatibility whitelist, the system SHALL copy the existing streams into a fast-start MP4 container without re-encoding. When the file does not satisfy the whitelist, the system SHALL re-encode the file. The whitelist SHALL require all of: an H.264 video stream, an AAC audio stream or no audio stream at all, and even pixel width and even pixel height. When `ffprobe` output cannot be parsed, the system SHALL treat the file as failing the whitelist and SHALL re-encode. The system SHALL report which path was taken through the status text of the download progress event.

#### Scenario: Already compatible source is remuxed

- **WHEN** a downloaded file has an H.264 video stream, an AAC audio stream, and even dimensions
- **THEN** the system SHALL copy the streams into a fast-start MP4 container without re-encoding
- **AND** the download progress status text SHALL indicate container optimization

#### Scenario: Incompatible source is re-encoded

- **WHEN** a downloaded file fails any whitelist condition
- **THEN** the system SHALL re-encode the file
- **AND** the download progress status text SHALL indicate re-encoding

#### Scenario: Unparseable probe output falls back to re-encoding

- **WHEN** `ffprobe` output for a downloaded file cannot be parsed
- **THEN** the system SHALL re-encode the file rather than failing the download

##### Example: whitelist decision table

| Video codec | Audio codec | Dimensions | Decision   | Notes                        |
| ----------- | ----------- | ---------- | ---------- | ---------------------------- |
| h264        | aac         | 1920x1080  | remux      | typical Facebook source      |
| h264        | none        | 1080x1080  | remux      | audio stream absent          |
| h264        | opus        | 1920x1080  | re-encode  | audio codec not AAC          |
| vp9         | aac         | 1920x1080  | re-encode  | video codec not H.264        |
| h264        | aac         | 1919x1080  | re-encode  | odd width                    |
| h264        | aac         | 1920x1079  | re-encode  | odd height                   |
| unparseable | unparseable | unknown    | re-encode  | conservative fallback        |

## MODIFIED Requirements

### Requirement: Progressive Downloading

The system SHALL execute `yt-dlp` to download videos and report real-time progress. The system SHALL continuously drain every output stream it pipes from the download process, so that a filled pipe buffer SHALL NOT block the child process. When a download fails, the returned error SHALL include the download tool's captured error output instead of a fixed message.

#### Scenario: Downloading with progress updates

- **WHEN** a download starts
- **THEN** the system SHALL emit progress events containing percentage and speed

##### Example: Progress update event

- **GIVEN** yt-dlp outputs "[download]  25.0% of 10.00MiB at  1.23MiB/s"
- **WHEN** the Rust backend parses this line
- **THEN** it SHALL emit a `download-progress` event with payload `{ "id": "task1", "progress": 25.0, "speed": "1.23MiB/s" }`

#### Scenario: Download failure reports the underlying reason

- **WHEN** the download tool exits unsuccessfully because the video is private
- **THEN** the returned error SHALL contain the tool's own error output describing the cause

#### Scenario: Large error output does not stall the download

- **WHEN** the download tool writes more error output than one pipe buffer holds
- **THEN** the download process SHALL continue to completion without blocking
