## MODIFIED Requirements

### Requirement: Default Settings Values

The system MUST define and use default values for all supported settings when they are missing from the database. When a stored value is present but falls outside the set of values the system recognizes for that key, the system MUST treat it as missing and use the default.

#### Scenario: First run default values

- **WHEN** `get_settings` is called and the database is empty
- **THEN** the system SHALL return a JSON object containing all default values

#### Scenario: Unrecognized stored value falls back to the default

- **WHEN** `get_settings` is called and the stored `theme` value is not one of `system`, `light`, or `dark`
- **THEN** the returned object SHALL report `theme` as `system`, and the system SHALL NOT write a corrected value back to the database

##### Example: Default Values Mapping

| Setting Key        | Default Value          | Notes                                        |
| ------------------ | ---------------------- | -------------------------------------------- |
| download_path      | (User Home)/Downloads  | Platform-specific home dir                   |
| transcoding_preset | 'Balanced'             | Mapping to CRF 23, preset medium             |
| auto_organize      | false                  | Boolean flag                                 |
| detect_clipboard   | true                   | Boolean flag                                 |
| theme              | 'system'               | One of 'system', 'light', 'dark'; preserves pre-existing behavior of following the OS color scheme |

## ADDED Requirements

### Requirement: Theme Setting Key

The system SHALL store the selected theme mode in the `settings` table under the key `theme`, using the existing key-value format. Because the table is key-value shaped, introducing this key SHALL NOT require a database migration.

The generic `update_setting` command SHALL remain free of per-key validation; validation of the `theme` value SHALL happen where stored settings are merged with defaults, so that the same fallback applies regardless of how the value came to be in the database.

#### Scenario: Persisting a theme selection

- **WHEN** the user selects the `dark` theme mode in the settings interface
- **THEN** the system SHALL invoke `update_setting` with the key `theme` and the value `dark`

#### Scenario: Upgrading an existing installation

- **WHEN** the application starts against a database created before the `theme` key existed
- **THEN** `get_settings` SHALL succeed and report `theme` as `system`, without any schema change being applied
