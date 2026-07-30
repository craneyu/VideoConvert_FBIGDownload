## MODIFIED Requirements

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
