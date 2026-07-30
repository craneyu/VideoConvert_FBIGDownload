## MODIFIED Requirements

### Requirement: Concurrent Download Limit
The system SHALL limit the number of downloads that are actively in their network phase to the network concurrency setting. The limit SHALL NOT be a fixed value compiled into the interface, and every place that displays the limit SHALL read the setting rather than a separately maintained copy.

Only tasks in a network-active state SHALL count towards this limit. A task that has finished its network phase and is waiting to be encoded SHALL NOT count towards it.

#### Scenario: Queuing multiple links
- **WHEN** the network concurrency setting is 2 and the user adds 5 video links in rapid succession
- **THEN** only the first 2 SHALL start downloading immediately, while the others remain in "Pending" status

#### Scenario: The limit comes from the setting rather than a fixed value
- **WHEN** the network concurrency setting is 4 and the user adds 5 video links in rapid succession
- **THEN** the first 4 SHALL start downloading immediately, and the queue SHALL NOT stop at any other number

#### Scenario: A raised limit is picked up without restarting the application
- **WHEN** the network concurrency setting is raised and the download queue next decides whether to start a task
- **THEN** that decision SHALL use the new value

Note: the settings interface is a separate route, and navigating to it discards the
in-memory download list. A limit change therefore cannot be observed against
downloads that were already queued before the change. Making the queue outlive
navigation is out of scope for this capability as specified.

#### Scenario: The displayed limit matches the setting
- **WHEN** the network concurrency setting is 4 and the CPU concurrency setting is 1
- **THEN** the interface SHALL display those two values as the concurrency limits

### Requirement: Automatic Queue Progression
The system SHALL automatically start the next "Pending" task when an active download completes, fails, or enters the waiting-for-encode state.

Entering the waiting-for-encode state SHALL trigger queue progression, because a task waiting for a CPU permit no longer occupies a network slot and holding the queue until it finishes encoding would leave that slot idle.

#### Scenario: Finishing a task
- **WHEN** an active download completes
- **THEN** the system SHALL immediately trigger the next available task in the queue

#### Scenario: Releasing the network slot before encoding starts
- **WHEN** an active download finishes its network phase and enters the waiting-for-encode state
- **THEN** the system SHALL immediately trigger the next available task in the queue, without waiting for that task's encoding to finish

## ADDED Requirements

### Requirement: Waiting-For-Encode Task State

A download that has finished its network phase and is waiting for a CPU permit SHALL be displayed in a distinct waiting-for-encode state, rather than being left showing an unchanging progress value.

Without this state, several tasks that have all finished downloading rest on the same unchanging progress value, which is indistinguishable from the application having stopped responding.

#### Scenario: A task waiting for a CPU permit is labelled as waiting
- **WHEN** a download finishes its network phase and no CPU permit is available
- **THEN** the task SHALL be shown as waiting to be encoded

#### Scenario: A waiting task starts reporting encode progress once it is granted a permit
- **WHEN** a task that is waiting to be encoded is granted a CPU permit
- **THEN** the task SHALL leave the waiting state and report post-processing progress

#### Scenario: A remuxed task never enters the waiting state
- **WHEN** a download's post-processing has been planned as a remux
- **THEN** the task SHALL proceed directly to post-processing without being shown as waiting to be encoded
