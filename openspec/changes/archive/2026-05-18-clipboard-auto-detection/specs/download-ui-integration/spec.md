## ADDED Requirements

### Requirement: Detected URL Prompt
The system SHALL display a non-intrusive prompt or banner when a valid video URL is detected in the clipboard.

#### Scenario: User accepts detected URL
- **WHEN** a URL is detected and the user clicks the "Use this link" action in the prompt
- **THEN** the system SHALL auto-fill the download input field with the detected URL
