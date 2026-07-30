# download-history-storage Specification

## Purpose

TBD - created by archiving change 'video-download-history'. Update Purpose after archive.

## Requirements

### Requirement: SQLite Persistence

The system SHALL use an SQLite database to store download history, including URL, title, status, and local file path. Creation timestamps SHALL be stored in local time so that no conversion is required when the value is displayed. When a download fails after its history record has been created, the system SHALL update that record's status to a failed state. When a download fails before its history record has been created, the system SHALL NOT attempt a record update, and SHALL still report the failure to the user.

#### Scenario: Recording a new download

- **WHEN** a download is initiated
- **THEN** a new entry SHALL be created in the `download_history` table with "downloading" status
- **AND** its creation timestamp SHALL be stored in local time

#### Scenario: Recording a failed download

- **WHEN** a download fails after its history record was created
- **THEN** the status of that record SHALL be updated to a failed state
- **AND** the record SHALL NOT remain in "downloading" status

#### Scenario: Failure before the record exists

- **WHEN** a download fails while fetching metadata, before any history record was created
- **THEN** the system SHALL report the failure to the user without attempting a record update

---
### Requirement: History Retrieval
The system SHALL allow the user to view all past downloads recorded in the database.

#### Scenario: Loading history on startup
- **WHEN** the application opens
- **THEN** it SHALL retrieve and display all entries from the `download_history` table

---
### Requirement: Local Time Timestamp Migration

The system SHALL apply a new schema migration that changes the download history creation timestamp default to local time and converts existing stored timestamps from UTC to local time. Previously released migrations SHALL NOT be modified, because installations that already ran them SHALL NOT re-run them. The migration SHALL copy existing records into the replacement table before removing the previous table. The conversion SHALL derive the local offset from the host system rather than hard-coding a fixed offset.

#### Scenario: Existing installation is migrated

- **WHEN** an installation whose history was recorded with UTC timestamps starts after the upgrade
- **THEN** the migration SHALL convert every existing timestamp to local time
- **AND** every existing record SHALL still be present afterwards

#### Scenario: Migration runs exactly once

- **WHEN** the application starts again after the migration has already been applied
- **THEN** the migration SHALL NOT run a second time
- **AND** timestamps SHALL NOT be shifted again

##### Example: UTC to local conversion on a UTC+8 host

- **GIVEN** an existing record whose stored timestamp is "2026-07-28 07:22:53" recorded as UTC
- **WHEN** the migration runs on a host set to UTC+8
- **THEN** the stored timestamp SHALL become "2026-07-28 15:22:53"
