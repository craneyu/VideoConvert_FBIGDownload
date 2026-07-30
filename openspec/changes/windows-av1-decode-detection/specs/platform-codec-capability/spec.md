## MODIFIED Requirements

### Requirement: Three-State Codec Decode Capability

The system SHALL answer whether the running platform can decode a given video codec with one of exactly three answers: supported, unsupported, or unknown. The answer SHALL NOT be reduced to a boolean.

Only the supported answer SHALL be treated as permission to use the original stream. Both unsupported and unknown SHALL be treated the same way by callers, so that a platform which cannot be interrogated never produces a file the platform is unable to play.

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

| Platform | Mechanism                                                     | Possible answers        |
| -------- | ------------------------------------------------------------- | ----------------------- |
| macOS    | VideoToolbox hardware decode query (deliberate under-reporting) | supported / unsupported |
| Windows  | Media Foundation decoder enumeration, software decoders included | supported / unsupported |
| Linux    | none available                                                | unknown only            |

Linux has no system-level mechanism representing whether the user's player can decode a codec, so it answers unknown for every codec.

## ADDED Requirements

### Requirement: Per-Platform Detection Strictness

A platform query SHALL be permitted to answer more conservatively than the platform's true decode capability, and SHALL NOT answer less conservatively. No platform SHALL return supported for a codec the platform is unable to play.

A platform whose query mechanism reports hardware decoding only SHALL return unsupported for a codec the platform decodes in software. A platform whose query mechanism enumerates every registered decoder SHALL return supported in that same situation.

This asymmetry between platforms is a deliberate consequence of the mechanisms available on each platform, not a defect. The requirement exists so that a platform answering supported where another answers unsupported is recognised as designed behaviour.

#### Scenario: A platform whose mechanism reports hardware decoding only

- **WHEN** the platform's query mechanism reports hardware decoders only
- **AND** the queried codec is decodable on that machine in software but not in hardware
- **THEN** the query SHALL return unsupported

#### Scenario: A platform whose mechanism enumerates every registered decoder

- **WHEN** the platform's query mechanism enumerates every registered decoder
- **AND** a software decoder for the queried codec is present
- **THEN** the query SHALL return supported

#### Scenario: No platform reports support for a codec it cannot play

- **WHEN** the platform has no decoder at all for the queried codec
- **THEN** the query SHALL NOT return supported

### Requirement: Windows Decode Capability Query

On Windows the system SHALL determine decode capability by enumerating the platform's registered video decoder transforms for the queried codec.

The enumeration SHALL NOT be restricted to hardware decoders, so a codec decodable in software SHALL be reported as supported.

The system SHALL NOT instantiate a decoder in order to answer the query; the presence of a decoder SHALL be the whole basis of the answer.

Codec names SHALL be mapped to the platform's video subtype identifiers. A codec name with no such mapping SHALL return unknown rather than unsupported, because an absent mapping means the platform was never asked.

#### Scenario: A decoder for the codec is registered

- **WHEN** the enumeration finds at least one decoder for the queried codec
- **THEN** the query SHALL return supported

#### Scenario: No decoder for the codec is registered

- **WHEN** the enumeration succeeds and finds no decoder for the queried codec
- **THEN** the query SHALL return unsupported

#### Scenario: The enumeration itself fails

- **WHEN** the platform enumeration call fails, or the call panics
- **THEN** the query SHALL return unknown
- **AND** the download SHALL NOT fail because of it
- **AND** no user-facing message SHALL be produced

#### Scenario: A codec name with no subtype mapping

- **WHEN** a codec name that has no mapping to a platform video subtype is queried
- **THEN** the query SHALL return unknown

##### Example: Windows answers for a machine with a software AV1 decoder installed

| Queried codec | Registered decoder found | Answer      |
| ------------- | ------------------------ | ----------- |
| av1           | yes, software only       | supported   |
| h264          | yes                      | supported   |
| vp9           | not mapped to a subtype  | unknown     |

##### Example: Windows answers for a machine with no AV1 decoder

| Queried codec | Registered decoder found | Answer      |
| ------------- | ------------------------ | ----------- |
| av1           | no                       | unsupported |
| h264          | yes                      | supported   |
