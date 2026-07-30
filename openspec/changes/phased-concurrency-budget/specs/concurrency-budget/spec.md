## ADDED Requirements

### Requirement: Shared CPU Permit Pool

The system SHALL bound the number of concurrently running re-encode operations with a single permit pool that is shared by both the download post-processing pipeline and the transcoding pipeline. The number of permits SHALL come from the CPU concurrency setting, read once when the application starts.

Neither pipeline SHALL maintain its own independent re-encode limit, because a per-pipeline limit cannot observe work running in the other pipeline.

#### Scenario: Re-encode work in both pipelines competes for the same permits

- **WHEN** the CPU concurrency setting is 1, a download has entered its re-encode phase, and the user starts a transcoding task
- **THEN** the transcoding task SHALL wait until the download's re-encode phase releases its permit

#### Scenario: A released permit is granted to the longest-waiting task

- **WHEN** a re-encode operation finishes and one or more operations are waiting for a permit
- **THEN** the system SHALL grant the released permit to a waiting operation rather than leaving it idle

##### Example: Five queued downloads with network concurrency 3 and CPU concurrency 1

| Point in time                          | Downloading | Re-encoding | Waiting for encode | Not yet started |
| -------------------------------------- | ----------- | ----------- | ------------------ | --------------- |
| Five downloads queued                  | 1, 2, 3     | none        | none               | 4, 5            |
| Task 1 finishes its network phase      | 2, 3, 4     | 1           | none               | 5               |
| Task 2 finishes its network phase      | 3, 4, 5     | 1           | 2                  | none            |
| Tasks 3, 4, 5 finish their network phase | none      | 1           | 2, 3, 4, 5         | none            |
| Task 1 finishes re-encoding            | none        | 2           | 3, 4, 5            | none            |

### Requirement: Phase-Scoped Permit Acquisition

A download consists of a network phase followed by a post-processing phase. The system SHALL treat the two phases as separately budgeted resources: a task that has finished its network phase SHALL NOT continue to occupy a network slot while it waits for a CPU permit.

Before waiting for a CPU permit, the download SHALL report that it has entered the waiting-for-encode state, so that the download queue can release the network slot and start the next pending download.

#### Scenario: A task waiting for a CPU permit releases its network slot

- **WHEN** the network concurrency setting is 3, three downloads are active, and the first finishes its network phase and begins waiting for a CPU permit
- **THEN** the fourth pending download SHALL start its network phase immediately, without waiting for the first task's re-encode to finish

#### Scenario: Waiting for a CPU permit does not occupy an async runtime worker

- **WHEN** a download is waiting for a CPU permit
- **THEN** other IPC commands SHALL continue to be served

### Requirement: Remux Is Exempt From The CPU Budget

Container remuxing copies streams without decoding or encoding. The system SHALL NOT require a CPU permit for a post-processing operation that has been planned as a remux, and SHALL require a CPU permit only for an operation planned as a re-encode.

#### Scenario: A remux proceeds while the CPU permits are exhausted

- **WHEN** the CPU concurrency setting is 1, a re-encode holds the only permit, and a second download's post-processing has been planned as a remux
- **THEN** the remux SHALL start immediately rather than waiting

#### Scenario: A re-encode waits when the CPU permits are exhausted

- **WHEN** the CPU concurrency setting is 1, a re-encode holds the only permit, and a second download's post-processing has been planned as a re-encode
- **THEN** the second operation SHALL wait for a permit before starting

##### Example: Permit requirement by post-processing plan

| Post-processing plan | Requires a CPU permit | Reason                                        |
| -------------------- | --------------------- | --------------------------------------------- |
| Remux                | no                    | stream copy, no decode and no encode          |
| Re-encode            | yes                   | saturates all available cores                 |

### Requirement: Long-Running Commands Do Not Block The Async Runtime

Commands that read a child process's output line by line and wait for it to exit SHALL perform that blocking work on a thread dedicated to blocking operations, so that the work does not occupy an async runtime worker for its whole duration.

Declaring such a command without the async keyword SHALL NOT be used to satisfy this requirement, because a command declared that way runs synchronously on the IPC thread.

#### Scenario: Settings remain readable during a long re-encode

- **WHEN** a re-encode that takes several minutes is in progress
- **THEN** a request to read or update settings SHALL be served without waiting for the re-encode to finish

#### Scenario: A panic in the blocking work is reported as a command error

- **WHEN** the blocking portion of a download or transcoding command panics
- **THEN** the command SHALL return an error to the caller, and the application SHALL remain usable

### Requirement: Permit Acquisition Failure Is Surfaced

The system SHALL NOT start a re-encode when a required CPU permit cannot be acquired. A failure to acquire a permit SHALL be returned to the caller as an error.

#### Scenario: The permit pool is unavailable

- **WHEN** a re-encode operation requests a CPU permit and the permit pool cannot grant one because it has been closed
- **THEN** the operation SHALL return an error, and SHALL NOT start the re-encode without a permit
