## MODIFIED Requirements

### Requirement: Post-Download Container Optimization

The system SHALL inspect each downloaded file with `ffprobe` and SHALL decide between container remux and full re-encoding. The decision SHALL be determined by three inputs together: the probe result, the configured video handling policy, and the platform's decode capability for the probed video codec.

When the decision is remux, the system SHALL copy the existing streams into a fast-start MP4 container without re-encoding. When the decision is re-encode, the system SHALL re-encode the file.

A file SHALL be eligible for remux only when all of the following hold: the video codec is one the system can remux into MP4, the file carries an AAC audio stream or no audio stream at all, and both pixel width and pixel height are even. The video codecs the system SHALL be able to remux are H.264 and AV1; every other codec SHALL be re-encoded regardless of policy, because its playability inside an MP4 container is not predictable.

Given an eligible file, the policy SHALL decide as follows. Under `compat` the system SHALL remux only H.264 and SHALL re-encode AV1. Under `original` the system SHALL remux both. Under `auto` the system SHALL remux H.264, and SHALL remux AV1 only when the platform reports decode support for AV1 — an unsupported or unknown answer SHALL result in re-encoding.

When `ffprobe` output cannot be parsed, the system SHALL treat the file as ineligible and SHALL re-encode. The system SHALL report which path was taken through the status text of the download progress event.

#### Scenario: Already compatible source is remuxed

- **WHEN** a downloaded file has an H.264 video stream, an AAC audio stream, and even dimensions
- **THEN** the system SHALL copy the streams into a fast-start MP4 container without re-encoding
- **AND** the download progress status text SHALL indicate container optimization

#### Scenario: Incompatible source is re-encoded

- **WHEN** a downloaded file fails any eligibility condition
- **THEN** the system SHALL re-encode the file
- **AND** the download progress status text SHALL indicate re-encoding

#### Scenario: Unparseable probe output falls back to re-encoding

- **WHEN** `ffprobe` output for a downloaded file cannot be parsed
- **THEN** the system SHALL re-encode the file rather than failing the download

#### Scenario: AV1 is remuxed when the platform can decode it

- **WHEN** the policy is `auto`, the video codec is AV1 with even dimensions and AAC audio, and the platform reports decode support for AV1
- **THEN** the system SHALL copy the streams into a fast-start MP4 container
- **AND** the output video stream SHALL remain AV1

#### Scenario: AV1 is re-encoded when platform support is not established

- **WHEN** the policy is `auto`, the video codec is AV1, and the platform reports either no support or an unknown answer for AV1
- **THEN** the system SHALL re-encode the file to H.264

#### Scenario: A codec outside the remuxable set is re-encoded even under `original`

- **WHEN** the policy is `original` and the video codec is VP9
- **THEN** the system SHALL re-encode the file

##### Example: decision table

| Video codec | Audio codec | Dimensions | Policy   | Platform answer for the codec | Decision   |
| ----------- | ----------- | ---------- | -------- | ----------------------------- | ---------- |
| h264        | aac         | 1920x1080  | auto     | any                           | remux      |
| h264        | none        | 1080x1080  | compat   | any                           | remux      |
| h264        | opus        | 1920x1080  | original | any                           | re-encode  |
| h264        | aac         | 1919x1080  | original | any                           | re-encode  |
| av1         | aac         | 1080x1920  | auto     | supported                     | remux      |
| av1         | aac         | 1080x1920  | auto     | unsupported                   | re-encode  |
| av1         | aac         | 1080x1920  | auto     | unknown                       | re-encode  |
| av1         | aac         | 1080x1920  | original | unknown                       | remux      |
| av1         | aac         | 1080x1920  | compat   | supported                     | re-encode  |
| av1         | aac         | 1081x1920  | original | supported                     | re-encode  |
| vp9         | aac         | 1920x1080  | original | supported                     | re-encode  |
| unparseable | unparseable | unknown    | original | any                           | re-encode  |

## ADDED Requirements

### Requirement: Audio Is Copied When Already AAC

When the system re-encodes a downloaded file, it SHALL copy the audio stream unchanged if the probed audio codec is already AAC, and SHALL encode the audio to AAC only when the probed audio codec is something else. When the file carries no audio stream, the system SHALL NOT emit any audio encoding option.

Re-encoding audio that is already AAC produces a larger file with worse audio than the source. Measured on a Facebook Reel: a 59959 bps AAC source became a 128385 bps AAC output — more bytes for a second generation of lossy compression.

#### Scenario: AAC source audio is preserved

- **WHEN** a file being re-encoded has an AAC audio stream
- **THEN** the output audio stream SHALL be bit-identical to the source audio stream

#### Scenario: Non-AAC source audio is converted

- **WHEN** a file being re-encoded has an Opus audio stream
- **THEN** the system SHALL encode the audio to AAC

#### Scenario: A file without audio produces no audio options

- **WHEN** a file being re-encoded carries no audio stream
- **THEN** the system SHALL NOT pass any audio codec or bitrate option to the encoder

##### Example: audio decision by probed codec

| Probed audio codec | Action        |
| ------------------ | ------------- |
| aac                | copy          |
| none               | no audio args |
| opus               | encode to AAC |
| mp3                | encode to AAC |
