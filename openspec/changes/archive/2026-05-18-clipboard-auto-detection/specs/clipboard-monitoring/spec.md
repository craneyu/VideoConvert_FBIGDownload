## ADDED Requirements

### Requirement: Clipboard Content Identification
The system SHALL identify if the current clipboard content is a valid Facebook or Instagram video URL.

#### Scenario: Valid URL detected
- **WHEN** the clipboard contains a URL matching Facebook or Instagram video patterns
- **THEN** the system SHALL mark the content as a "detectable video link"

##### Example: URL patterns
| Input | Result |
|-------|--------|
| https://www.facebook.com/watch?v=123 | detectable |
| https://www.instagram.com/reels/ABC/ | detectable |
| https://google.com | ignored |

### Requirement: Active Window Focus Detection
The system SHALL check the clipboard content when the application window gains focus to ensure timely detection.

#### Scenario: App window becomes active
- **WHEN** the user switches focus back to the VidBridge window
- **THEN** the system SHALL perform a clipboard scan if the feature is enabled

##### Example: Focus trigger
- **GIVEN** setting "Auto-detect clipboard" is ON
- **WHEN** user copies a Facebook link and brings VidBridge to the foreground
- **THEN** the system SHALL invoke the clipboard check command
