## ADDED Requirements

### Requirement: SQLite Persistence
The system SHALL use an SQLite database to store download history, including URL, title, status, and local file path.

#### Scenario: Recording a new download
- **WHEN** a download is initiated
- **THEN** a new entry SHALL be created in the `download_history` table with "downloading" status

### Requirement: History Retrieval
The system SHALL allow the user to view all past downloads recorded in the database.

#### Scenario: Loading history on startup
- **WHEN** the application opens
- **THEN** it SHALL retrieve and display all entries from the `download_history` table