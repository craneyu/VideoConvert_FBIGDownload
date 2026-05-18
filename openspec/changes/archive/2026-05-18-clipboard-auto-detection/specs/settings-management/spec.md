## ADDED Requirements

### Requirement: Clipboard Detection Toggle
The system SHALL provide a setting to enable or disable automatic clipboard detection of video URLs.

#### Scenario: Disabling clipboard detection
- **WHEN** the user toggles the "Auto-detect clipboard" setting to OFF
- **THEN** the system SHALL NOT perform clipboard scans even when the window is focused
