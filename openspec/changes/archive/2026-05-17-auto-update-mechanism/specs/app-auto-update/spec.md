## ADDED Requirements

### Requirement: Automatic Update Check
The system SHALL check for available updates upon application startup from the configured GitHub repository.

#### Scenario: Update available
- **WHEN** the application starts and a newer version exists on GitHub
- **THEN** the system SHALL prompt the user with update details (version, release notes)

### Requirement: Secure Update Installation
The system SHALL verify the update payload using a public key before proceeding with the installation.

#### Scenario: Installing update
- **WHEN** the user confirms the update
- **THEN** the system SHALL download and install the new version, then restart the application