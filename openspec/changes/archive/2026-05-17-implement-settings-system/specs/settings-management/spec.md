## ADDED Requirements

### Requirement: Persistent Settings Storage
The system MUST store application settings in a persistent SQLite database table named `settings` using a key-value format.

#### Scenario: Initial database migration
- **WHEN** the application starts for the first time
- **THEN** the system SHALL create the `settings` table if it does not exist

### Requirement: Global Settings Access
The system SHALL provide an IPC command `get_settings` that returns all stored settings as a JSON object.

#### Scenario: Fetching settings on startup
- **WHEN** the frontend initializes
- **THEN** it SHALL invoke `get_settings` to populate the settings store

### Requirement: Updating Settings
The system SHALL provide an IPC command `update_setting` that accepts a key and a value to update the persistent store.

#### Scenario: Changing download path
- **WHEN** the user selects a new download directory in the UI
- **THEN** the system SHALL invoke `update_setting` with the key 'download_path' and the new directory path

### Requirement: Default Settings Values
The system MUST define and use default values for all supported settings when they are missing from the database.

#### Scenario: First run default values
- **WHEN** `get_settings` is called and the database is empty
- **THEN** the system SHALL return a JSON object containing all default values

##### Example: Default Values Mapping
| Setting Key | Default Value | Notes |
|-------------|---------------|-------|
| download_path | (User Home)/Downloads | Platform-specific home dir |
| transcoding_preset | 'Balanced' | Mapping to CRF 23, preset medium |
| auto_organize | false | Boolean flag |