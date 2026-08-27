# Settings Coding-Tool Detection Status Design

## Goal

Make each Coding tools entry scannable by keeping its `Detected` / `Not detected` status in the entry's top row, regardless of whether the tool is enabled and its integration details are expanded.

## Design

`SettingsModal` will render one `HarnessDetectionStatus` beside each coding-tool name in the row header. The status remains in that position for loading, detected, not-detected, and unknown states. The existing detection API and component-owned refresh behavior remain unchanged.

`HarnessIntegrationSection` will no longer render its duplicate tool-detection status. Its expanded content will contain only integration-specific information and actions: integration errors, hook registration state, install/uninstall controls, diagnostics, and custom-path controls.

The resulting row structure is:

`tool icon + tool name + detection status | enable toggle`

No backend, IPC, persistence, or settings-schema changes are required.

## Data flow and error handling

Each row-level `HarnessDetectionStatus` continues to query `getHarnessIntegrationStatus(harnessId)` independently. While the request is pending it shows `Checking…`; failed status responses show `Unknown`; successful responses show `Detected` or `Not detected`. Expanded integration controls may perform their own status refresh after install, uninstall, or custom-path changes; this is existing behavior and does not change the source of truth.

## Testing

- Update the renderer source-level tests to require the detection status in the coding-tool header for enabled and disabled tools.
- Add or adjust a test ensuring the expanded integration section does not render a second `Detected` / `Not detected` status.
- Run the desktop type-check and test suite, followed by the repository documentation drift check.

## Scope

This change is limited to the Settings dialog's Coding tools presentation. It does not alter detection semantics, status wording, integration probing, or how active coding tools are saved.
