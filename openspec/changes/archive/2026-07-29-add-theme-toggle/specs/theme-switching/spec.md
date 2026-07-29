## ADDED Requirements

### Requirement: Theme Mode Selection

The system SHALL provide three theme modes — `system`, `light`, and `dark` — and SHALL allow the user to select among them from the settings interface. The selected mode SHALL take effect immediately without requiring a reload or application restart, and SHALL persist across application restarts.

#### Scenario: Selecting an explicit theme mode

- **WHEN** the user selects the `light` mode in the settings interface
- **THEN** the interface SHALL render with light colors immediately, and SHALL still render with light colors after the application is restarted

#### Scenario: Default mode on first run

- **WHEN** the application starts and no theme value has ever been stored
- **THEN** the system SHALL behave as if `system` mode were selected, matching the behavior of releases that predate theme selection

---

### Requirement: Resolved Theme Attribute Contract

The system SHALL express the active color scheme as a `data-theme` attribute on the root `html` element. The attribute value SHALL always be either `light` or `dark`. The attribute SHALL NOT be absent, empty, or set to `system` — `system` is a settings-level mode name and MUST NOT appear as an attribute value.

The `dark:` styling variant SHALL be driven by this attribute rather than by the `prefers-color-scheme` media query, so that rendering is determined solely by the attribute.

#### Scenario: System mode resolves to a concrete value

- **WHEN** the selected mode is `system` and the operating system reports a dark color scheme preference
- **THEN** the root `html` element SHALL carry `data-theme="dark"`

##### Example: Mode to attribute value mapping

| Selected mode | OS preference | `data-theme` value |
| ------------- | ------------- | ------------------ |
| `light`       | dark          | `light`            |
| `light`       | light         | `light`            |
| `dark`        | light         | `dark`             |
| `dark`        | dark          | `dark`             |
| `system`      | dark          | `dark`             |
| `system`      | light         | `light`            |

---

### Requirement: Following the Operating System Preference

While the selected mode is `system`, the system SHALL follow operating system color scheme changes without requiring a restart. While the selected mode is `light` or `dark`, the system SHALL NOT change the rendered color scheme in response to operating system color scheme changes.

#### Scenario: OS preference changes while following the system

- **WHEN** the selected mode is `system` and the operating system color scheme changes from light to dark
- **THEN** the interface SHALL re-render with dark colors without a restart

#### Scenario: OS preference changes while a fixed mode is selected

- **WHEN** the selected mode is `light` and the operating system color scheme changes from light to dark
- **THEN** the interface SHALL continue to render with light colors

---

### Requirement: Theme Applied Before First Paint

The system SHALL apply the resolved color scheme before the first paint of the window, so that no flash of an unintended color scheme is visible during startup. Because the authoritative theme value is read over an asynchronous IPC call that cannot complete before first paint, the system SHALL maintain a synchronously readable cache of the selected mode and SHALL consult it during startup.

Once the authoritative value has loaded, the system SHALL re-apply the color scheme from the authoritative value and SHALL refresh the cache when the two differ.

#### Scenario: Restarting with a dark theme selected

- **WHEN** the selected mode resolves to `dark` and the user restarts the application
- **THEN** no light-colored frame SHALL be visible at any point during startup

#### Scenario: Cache unavailable or holding an unrecognized value

- **WHEN** the synchronous cache cannot be read, or holds a value that is not one of `system`, `light`, or `dark`
- **THEN** the system SHALL fall back to the operating system color scheme preference for the pre-paint value, and SHALL NOT raise an error that interrupts page load

---

### Requirement: Native Control Color Scheme

The system SHALL declare the active color scheme to the browser engine so that natively rendered controls — including select dropdowns and scrollbars — match the active theme rather than the operating system preference.

#### Scenario: Opening a native dropdown in light mode

- **WHEN** the resolved color scheme is `light` and the user opens one of the transcoding option dropdowns
- **THEN** the dropdown SHALL render with a light appearance

---

### Requirement: Invalid Theme Value Fallback

The system SHALL treat any stored theme value outside the set `system`, `light`, `dark` as absent and SHALL fall back to `system`. The fallback SHALL be silent: the system SHALL NOT surface an error to the user and SHALL NOT write a corrected value back to storage.

#### Scenario: Unrecognized value in storage

- **WHEN** the stored theme value is an empty string or an unrecognized string such as `sepia`
- **THEN** the system SHALL report the theme mode as `system`

##### Example: Stored value normalization

| Stored value | Reported mode | Notes                       |
| ------------ | ------------- | --------------------------- |
| `light`      | `light`       | valid                       |
| `dark`       | `dark`        | valid                       |
| `system`     | `system`      | valid                       |
| (empty)      | `system`      | treated as absent           |
| `sepia`      | `system`      | unrecognized, silent fallback |
