## MODIFIED Requirements

### Requirement: Default Settings Values

The system MUST define and use default values for all supported settings when they are missing from the database. When a stored value is present but falls outside the set of values the system recognizes for that key, the system MUST treat it as missing and use the default.

#### Scenario: First run default values

- **WHEN** `get_settings` is called and the database is empty
- **THEN** the system SHALL return a JSON object containing all default values

#### Scenario: Unrecognized stored value falls back to the default

- **WHEN** `get_settings` is called and the stored `theme` value is not one of `system`, `light`, or `dark`
- **THEN** the returned object SHALL report `theme` as `system`, and the system SHALL NOT write a corrected value back to the database

#### Scenario: Out-of-range concurrency value falls back to the default

- **WHEN** `get_settings` is called and the stored `max_cpu_concurrency` value is `8`, which is outside the accepted range
- **THEN** the returned object SHALL report `max_cpu_concurrency` as `1`, and the system SHALL NOT write a corrected value back to the database

##### Example: Default Values Mapping

| Setting Key             | Default Value          | Notes                                        |
| ----------------------- | ---------------------- | -------------------------------------------- |
| download_path           | (User Home)/Downloads  | Platform-specific home dir                   |
| transcoding_preset      | 'Balanced'             | Mapping to CRF 23, preset medium             |
| auto_organize           | false                  | Boolean flag                                 |
| detect_clipboard        | true                   | Boolean flag                                 |
| theme                   | 'system'               | One of 'system', 'light', 'dark'; preserves pre-existing behavior of following the OS color scheme |
| max_network_concurrency | 3                      | Accepted range 1 to 8; how many downloads run their network phase at once |
| max_cpu_concurrency     | 1                      | Accepted range 1 to 2; shared budget for re-encoding across downloads and transcoding |

## ADDED Requirements

### Requirement: Concurrency Setting Keys

The system SHALL store the two concurrency limits in the `settings` table under the keys `max_network_concurrency` and `max_cpu_concurrency`, using the existing key-value format. Because the table is key-value shaped, introducing these keys SHALL NOT require a database migration.

The generic `update_setting` command SHALL remain free of per-key validation. Parsing and range checking of both values SHALL happen where stored settings are merged with defaults, so that the same fallback applies regardless of how the value came to be in the database.

#### Scenario: Persisting a concurrency selection

- **WHEN** the user sets the network concurrency to 4 in the settings interface
- **THEN** the system SHALL invoke `update_setting` with the key `max_network_concurrency` and the value `4`

#### Scenario: Upgrading an existing installation

- **WHEN** the application starts against a database created before these keys existed
- **THEN** `get_settings` SHALL succeed and report the default values for both keys, without any schema change being applied

##### Example: Parsing stored concurrency values

| Key                     | Stored value | Reported value | Reason                        |
| ----------------------- | ------------ | -------------- | ----------------------------- |
| max_network_concurrency | "4"          | 4              | within the accepted range     |
| max_network_concurrency | "1"          | 1              | lower bound of the range      |
| max_network_concurrency | "8"          | 8              | upper bound of the range      |
| max_network_concurrency | "0"          | 3              | below the range, default used |
| max_network_concurrency | "9"          | 3              | above the range, default used |
| max_network_concurrency | "abc"        | 3              | not a number, default used    |
| max_cpu_concurrency     | "2"          | 2              | upper bound of the range      |
| max_cpu_concurrency     | "3"          | 1              | above the range, default used |

### Requirement: CPU Concurrency Change Requires A Restart

The CPU permit pool is built once per process — when a permit is first needed — and is fixed thereafter, so a change to `max_cpu_concurrency` SHALL take effect on the next launch. The settings interface SHALL state this, so that a user who changes the value does not read the unchanged behaviour as the setting having failed to save.

A change to `max_network_concurrency` SHALL take effect immediately, because the download queue reads that setting each time it decides whether to start another download.

#### Scenario: Changing the CPU limit states when it applies

- **WHEN** the user changes the CPU concurrency setting
- **THEN** the settings interface SHALL indicate that the new value applies after the application is restarted

#### Scenario: Changing the network limit applies at once

- **WHEN** the user changes the network concurrency setting
- **THEN** the new value SHALL take effect without the application being restarted, and without waiting for another task to be added or to finish
