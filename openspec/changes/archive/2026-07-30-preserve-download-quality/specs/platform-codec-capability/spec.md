## ADDED Requirements

### Requirement: Three-State Codec Decode Capability

The system SHALL answer whether the running platform can decode a given video codec with one of exactly three answers: supported, unsupported, or unknown. The answer SHALL NOT be reduced to a boolean.

Only the supported answer SHALL be treated as permission to use the original stream. Both unsupported and unknown SHALL be treated the same way by callers, so that a platform which cannot be interrogated never produces a file that might not play.

#### Scenario: A platform that reports decode support

- **WHEN** the platform reports that it can decode the queried codec
- **THEN** the query SHALL return supported

#### Scenario: A platform that cannot be interrogated

- **WHEN** the platform provides no way to determine whether the codec can be decoded
- **THEN** the query SHALL return unknown, and SHALL NOT return supported

#### Scenario: Codec names are compared case-insensitively

- **WHEN** the codec name is queried as `AV1`, `av1`, or `Av1`
- **THEN** all three SHALL produce the same answer

##### Example: Answer by platform for AV1

| Platform | Mechanism                          | Possible answers            |
| -------- | ---------------------------------- | --------------------------- |
| macOS    | VideoToolbox hardware decode query | supported / unsupported     |
| Windows  | none yet — see note                | unknown only                |
| Linux    | none available                     | unknown only                |

Windows currently has no mechanism: querying Media Foundation is the intended
approach but requires a COM dependency and cannot be compiled or behaviourally
tested outside Windows, so it is deferred rather than written blind. Until then
Windows behaves exactly as a platform that cannot be interrogated.

### Requirement: Detection Failure Is Treated As Unknown

When the platform query itself fails — the system framework cannot be loaded, the call returns an error, or the lookup panics — the system SHALL return unknown rather than propagating a failure and rather than assuming support.

#### Scenario: The platform query errors

- **WHEN** the underlying platform call fails for any reason
- **THEN** the query SHALL return unknown
- **AND** the download SHALL NOT fail because of it

### Requirement: Capability Is Queried Once Per Process

The system SHALL query each platform capability at most once per process and SHALL reuse the answer for the remaining lifetime of the process.

A platform's decode capability does not change while the application runs — installing a decoder package requires restarting the application before the platform reports it — and the Windows lookup is not free enough to repeat for every download.

#### Scenario: Repeated queries do not repeat the platform lookup

- **WHEN** the same codec is queried more than once in one process
- **THEN** the platform lookup SHALL be performed only for the first query

#### Scenario: A newly installed decoder is picked up after a restart

- **WHEN** the user installs a platform decoder while the application is running
- **THEN** the application SHALL continue to report the previously determined answer until it is restarted
