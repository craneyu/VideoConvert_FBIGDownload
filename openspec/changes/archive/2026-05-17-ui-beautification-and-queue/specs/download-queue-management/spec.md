## ADDED Requirements

### Requirement: Concurrent Download Limit
The system SHALL limit the number of active downloads to a maximum of 2 tasks simultaneously.

#### Scenario: Queuing multiple links
- **WHEN** user adds 5 video links in rapid succession
- **THEN** only the first 2 SHALL start downloading immediately, while the others remain in "Pending" status

### Requirement: Automatic Queue Progression
The system SHALL automatically start the next "Pending" task when an active download completes or fails.

#### Scenario: Finishing a task
- **WHEN** an active download completes
- **THEN** the system SHALL immediately trigger the next available task in the queue