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

#### Scenario: Unrecognized video handling policy falls back to the default

- **WHEN** `get_settings` is called and the stored `download_video_handling` value is not one of `auto`, `original`, or `compat`
- **THEN** the returned object SHALL report `download_video_handling` as `auto`, and the system SHALL NOT write a corrected value back to the database

##### Example: Default Values Mapping

| Setting Key               | Default Value          | Notes                                        |
| ------------------------- | ---------------------- | -------------------------------------------- |
| download_path             | (User Home)/Downloads  | Platform-specific home dir                   |
| transcoding_preset        | 'Balanced'             | Mapping to CRF 23, preset medium             |
| auto_organize             | false                  | Boolean flag                                 |
| detect_clipboard          | true                   | Boolean flag                                 |
| theme                     | 'system'               | One of 'system', 'light', 'dark'; preserves pre-existing behavior of following the OS color scheme |
| max_network_concurrency   | 3                      | Accepted range 1 to 8; how many downloads run their network phase at once |
| max_cpu_concurrency       | 1                      | Accepted range 1 to 2; shared budget for re-encoding across downloads and transcoding |
| download_video_handling   | 'auto'                 | One of 'auto', 'original', 'compat'; decides whether a downloaded video is remuxed or re-encoded |

## ADDED Requirements

### Requirement: Download Video Handling Setting Key

The system SHALL store the video handling policy in the `settings` table under the key `download_video_handling`, using the existing key-value format. Because the table is key-value shaped, introducing this key SHALL NOT require a database migration.

The accepted values SHALL be exactly `auto`, `original`, and `compat`. The generic `update_setting` command SHALL remain free of per-key validation; validation SHALL happen where stored settings are merged with defaults, so that the same fallback applies regardless of how the value came to be in the database.

The stored value SHALL NOT be a cached platform detection result. `auto` SHALL remain `auto` in storage and SHALL be resolved against the platform each time a post-processing decision is made, so that changing machine, upgrading the operating system, or installing a decoder does not leave a stale answer behind that the user has no reason to revisit.

#### Scenario: Persisting a policy selection

- **WHEN** the user selects the compatibility-first policy in the settings interface
- **THEN** the system SHALL invoke `update_setting` with the key `download_video_handling` and the value `compat`

#### Scenario: Upgrading an existing installation

- **WHEN** the application starts against a database created before this key existed
- **THEN** `get_settings` SHALL succeed and report `download_video_handling` as `auto`, without any schema change being applied

#### Scenario: Auto is stored as auto

- **WHEN** the policy is `auto` and the platform reports decode support for AV1
- **THEN** the stored value SHALL remain `auto` rather than being rewritten to `original`

### Requirement: Resolved Auto Policy Is Shown

When the policy is `auto`, the settings interface SHALL show which of the two outcomes it currently resolves to. Without this, `auto` gives the user no way to know whether their downloads are being remuxed or re-encoded.

The interface SHALL also state the trade-off of keeping the original stream: better quality, smaller files, and near-instant post-processing, against the file possibly not playing on devices that lack a decoder for that codec.

#### Scenario: Auto resolving to keeping the original

- **WHEN** the policy is `auto` and the platform reports decode support for AV1
- **THEN** the settings interface SHALL indicate that downloads will keep their original video stream

#### Scenario: Auto resolving to re-encoding

- **WHEN** the policy is `auto` and the platform does not report decode support for AV1
- **THEN** the settings interface SHALL indicate that downloads will be re-encoded for compatibility

#### Scenario: The portability trade-off is stated

- **WHEN** the user views the video handling setting
- **THEN** the interface SHALL state that keeping the original stream may produce a file that does not play on other devices
