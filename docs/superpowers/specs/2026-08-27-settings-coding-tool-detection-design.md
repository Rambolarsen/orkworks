# Settings Coding-Tool Detection Status Design

## Goal

Make each Coding tools entry scannable by keeping its `Detected` / `Not detected` status in the entry's top row, regardless of whether the tool is enabled and its integration details are expanded.

## Design

`SettingsModal` will render one `HarnessDetectionStatus` beside each coding-tool name in the row header. The status remains in that position for loading, detected, not-detected, and unknown states. The row header is a single flex row: the tool identity and status stay together on the left, while the enable toggle stays aligned to the right; the status must not move into the expanded content when the row wraps or narrows.

`HarnessIntegrationSection` will no longer render its duplicate tool-detection status. Its expanded content will contain only integration-specific information and actions: integration errors, hook registration state, install/uninstall controls, diagnostics, and custom-path controls.

`SettingsModal` will own a per-tool detection refresh generation and pass it to both components. After an integration mutation that can change executable detection (install, uninstall, save custom path, or clear custom path), `HarnessIntegrationSection` will notify the parent; the parent will advance that tool's generation, causing the row-level status to re-fetch. The row-level indicator remains the sole rendered detection status.

The resulting row structure is:

`tool icon + tool name + detection status | enable toggle`

No backend, IPC, persistence, or settings-schema changes are required.

## Data flow and error handling

Each row-level `HarnessDetectionStatus` queries `getHarnessIntegrationStatus(harnessId)` on mount and when its parent-provided refresh generation changes. While the request is pending it shows `Checking…`; failed status responses show `Unknown`; successful responses show `Detected` or `Not detected`. The component ignores a response after unmount or after a newer request has started. Integration controls may retain their local status response for rendering integration-specific details, but the parent refresh signal keeps the visible row indicator current after mutations.

The status exposes an accessible label such as `Coding tool detection status: Detected` and uses a non-disruptive status announcement; it is not a control. “Unknown” is reserved for a failed or unusable status response, while an unsupported integration remains an integration-specific message in the expanded section.

## Testing

- Update renderer tests to require one `HarnessDetectionStatus` in every coding-tool row header, for both enabled and disabled tools, and no detection-status instance in the expanded integration section.
- Test that integration mutations notify the parent and cause the row-level status to refresh for the affected harness.
- Test the loading, detected, not-detected, unknown, unmount, and stale-response states of the row-level status component.
- Add a layout assertion for the header's single-row flex structure and right-aligned toggle; keep the status readable at narrow widths without allowing it to collide with the toggle.
- Run the desktop type-check and test suite, followed by the repository documentation drift check.

## Scope

This change is limited to the Settings dialog's Coding tools presentation. It does not alter detection semantics, status wording, integration probing, or how active coding tools are saved.
