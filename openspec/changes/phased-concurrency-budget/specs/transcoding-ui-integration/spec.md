## ADDED Requirements

### Requirement: Transcoding Tasks Are Governed By A Queue

The number of transcoding tasks running at the same time SHALL NOT exceed the CPU concurrency setting. Starting a task while that limit is already reached SHALL place the task in a waiting state instead of spawning another encoder process.

The interface SHALL NOT display a concurrency limit that is not enforced.

#### Scenario: Starting more tasks than the limit allows

- **WHEN** the CPU concurrency setting is 1 and the user starts five transcoding tasks in rapid succession
- **THEN** one task SHALL run and the other four SHALL be shown as waiting

#### Scenario: The next waiting task starts when one finishes

- **WHEN** a running transcoding task finishes and at least one task is waiting
- **THEN** exactly one waiting task SHALL start

#### Scenario: The displayed transcoding limit is the enforced limit

- **WHEN** the interface displays a transcoding concurrency limit
- **THEN** that value SHALL be the CPU concurrency setting that is actually enforced

##### Example: Five tasks started with a CPU concurrency of 1

| Point in time                 | Running | Waiting    | Completed  |
| ----------------------------- | ------- | ---------- | ---------- |
| Five tasks started            | 1       | 2, 3, 4, 5 | none       |
| Task 1 finishes               | 2       | 3, 4, 5    | 1          |
| Task 2 finishes               | 3       | 4, 5       | 1, 2       |

### Requirement: Transcoding Shares Its Limit With Download Post-Processing

The transcoding limit SHALL be the same budget that bounds re-encoding in the download pipeline, not a second independent limit. A transcoding task SHALL wait when the budget is fully consumed by download post-processing, and the reverse SHALL also hold.

#### Scenario: A transcoding task waits for download post-processing

- **WHEN** the CPU concurrency setting is 1, a download is re-encoding, and the user starts a transcoding task
- **THEN** the transcoding task SHALL be shown as waiting until the download's re-encode finishes
