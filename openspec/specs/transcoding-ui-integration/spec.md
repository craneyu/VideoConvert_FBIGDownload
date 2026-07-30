# transcoding-ui-integration Specification

## Purpose

TBD - created by archiving change 'video-transcoding'. Update Purpose after archive.

## Requirements

### Requirement: Batch File Selection

The UI SHALL allow users to select multiple video files for transcoding via a file picker or drag-and-drop. Event listeners for file drops and task progress SHALL be registered exactly once per mounted view, and SHALL be removed when that view is destroyed. Navigating away from a view and back SHALL NOT increase the number of registered listeners, and a single file drop SHALL therefore create exactly one task per dropped file regardless of how many times the user has navigated.

#### Scenario: Drag and drop files

- **WHEN** the user drops three MP4 files into the application
- **THEN** three new transcoding tasks SHALL appear in the list

#### Scenario: No duplicate tasks after navigating away and back

- **WHEN** the user navigates from the main view to settings and back, then drops one MP4 file
- **THEN** exactly one new transcoding task SHALL appear in the list

##### Example: repeated navigation before a single drop

| Navigation round trips before drop | Files dropped | Tasks created |
| ---------------------------------- | ------------- | ------------- |
| 0                                  | 1             | 1             |
| 1                                  | 1             | 1             |
| 2                                  | 1             | 1             |
| 2                                  | 3             | 3             |

#### Scenario: Listeners removed when the view is destroyed

- **WHEN** the main view is destroyed
- **THEN** the file drop and progress listeners it registered SHALL be removed

---
### Requirement: Task Dashboard
The UI SHALL display a list of all active and completed transcoding tasks with progress bars and status indicators.

#### Scenario: Viewing task status
- **WHEN** a task completes
- **THEN** its status SHALL change to "Success" and a notification SHALL be triggered

---
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

---
### Requirement: Transcoding Shares Its Limit With Download Post-Processing

The transcoding limit SHALL be the same budget that bounds re-encoding in the download pipeline, not a second independent limit. A transcoding task SHALL wait when the budget is fully consumed by download post-processing, and the reverse SHALL also hold.

#### Scenario: A download's re-encode waits for a running transcode

- **WHEN** the CPU concurrency setting is 1, a transcoding task is running, and a download finishes its network phase and needs re-encoding
- **THEN** the download SHALL wait, and no second encoder process SHALL be started while the transcode holds the permit

#### Scenario: A transcoding task waits for download post-processing

- **WHEN** the CPU concurrency setting is 1, a download is re-encoding, and the user starts a transcoding task
- **THEN** the transcoding task SHALL be shown as waiting until the download's re-encode finishes
