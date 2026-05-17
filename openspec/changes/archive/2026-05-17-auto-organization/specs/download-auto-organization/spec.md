## ADDED Requirements

### Requirement: Source-Based Directory Creation
The system SHALL automatically create a sub-directory named after the video source (e.g., "Facebook", "Instagram") within the `VidBridge` folder in the downloads directory.

#### Scenario: Creating Facebook directory
- **WHEN** user downloads a Facebook video
- **THEN** the system SHALL ensure `~/Downloads/VidBridge/Facebook/` exists before saving

### Requirement: Categorized File Saving
The system SHALL save the downloaded video into the source-specific sub-directory.

#### Scenario: Saving to specific path
- **WHEN** a download completes for Instagram
- **THEN** the resulting file SHALL be located inside `~/Downloads/VidBridge/Instagram/`