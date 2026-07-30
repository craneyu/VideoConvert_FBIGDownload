## MODIFIED Requirements

### Requirement: Automatic Update Check

The system SHALL check for available updates upon application startup by retrieving an update manifest from the configured endpoint. The configured endpoint SHALL resolve to a manifest that the published release process actually produces, and the manifest filename SHALL match the filename the endpoint requests. Update check failures SHALL be written to the application log. Update check failures SHALL NOT be silently discarded, and SHALL NOT block application startup.

#### Scenario: Update available

- **WHEN** the application starts and a newer version exists at the configured endpoint
- **THEN** the system SHALL prompt the user with update details (version, release notes)

#### Scenario: Update manifest cannot be retrieved

- **WHEN** the application starts and the configured endpoint returns no usable manifest
- **THEN** the system SHALL write the failure reason to the application log
- **AND** the system SHALL continue starting up without interrupting the user

#### Scenario: Manifest filename matches the configured endpoint

- **WHEN** the release process publishes an update manifest
- **THEN** the published manifest filename SHALL equal the filename requested by the configured updater endpoint

---

### Requirement: Secure Update Installation

The system SHALL verify the update payload against a minisign public key before proceeding with the installation. The verification key stored in application configuration SHALL be a public key: its decoded content SHALL NOT contain key-derivation-function fields (salt, operations limit, memory limit) and its comment line SHALL NOT identify it as a secret key. The corresponding private key SHALL NOT be stored in version control, and SHALL be supplied to the release process exclusively through repository secrets.

#### Scenario: Installing update

- **WHEN** the user confirms the update
- **THEN** the system SHALL download and install the new version, then restart the application

#### Scenario: Configured verification key is a public key

- **WHEN** the verification key in application configuration is decoded
- **THEN** the decoded content SHALL NOT contain a secret key marker
- **AND** the decoded content SHALL NOT contain key-derivation-function fields

#### Scenario: Private signing key absent from version control

- **WHEN** the repository is searched for the active signing private key
- **THEN** no file under version control SHALL contain it
