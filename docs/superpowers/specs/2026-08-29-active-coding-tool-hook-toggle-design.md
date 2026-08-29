# Active coding tool hook toggle design

## Context

Settings currently stages active coding-tool choices for the existing Save
action, while hook integration has a separate inline install, reinstall, and
uninstall control. This makes it possible for a tool to be enabled without its
OrkWorks-owned hook being installed, and gives the user two controls for one
conceptual setup action.

The authoritative MVP contract remains that integrations are workspace-only,
ownership-aware, idempotent mutations. OrkWorks must preserve unrelated coding
tool configuration and remove only entries it owns.

## Decision

The active coding-tool toggle is the single user-facing control for both tool
availability and OrkWorks hook integration.

The existing Settings Save transaction remains the commit boundary:

- Toggling a tool changes the draft state.
- Saving a transition to enabled enables the tool and installs or repairs its
  OrkWorks-owned hook.
- Saving a transition to disabled disables the tool and removes only
  OrkWorks-owned hook entries.
- The requested tool-state transition remains durable even if its hook
  mutation fails; the failed integration is represented by the orange toggle
  state and its tooltip so the user can retry.
- Existing separate inline install, reinstall, and uninstall actions are
  removed.

Hook mutations continue to use the existing Electron-main and sidecar
authority boundaries. The renderer requests the combined settings operation;
it does not gain direct filesystem or hook-mutation authority.

## Toggle states

The toggle communicates integration state through its visual color and its
accessible name/tooltip:

| State | Toggle appearance | Meaning |
| --- | --- | --- |
| Disabled and clean | Neutral/off | Tool is unavailable and no OrkWorks-owned hook remains. |
| Enabled and healthy | Green | Tool is enabled and its OrkWorks hook is installed. |
| Enabled with failed or drifted hook | Orange | Tool is enabled, but the hook failed to install or needs repair. |
| Hook update/repair in progress | Blue | OrkWorks is applying the hook mutation. |

Blue is transient and represents an operation in progress. Orange covers both
retryable operation failures and a detected drifted installation. A red state
is not required by this design.

The accessible label and tooltip include the specific condition. For example:
“Enabled, but hook installation failed: permission denied.” The warning is
shown on the tool’s existing top-row icon/toggle rather than in a separate
inline error section. A successful install/repair or uninstall clears the
warning state.

Coding-tool detection remains a separate status signal and must not be
conflated with hook health.

## Error handling and ownership

When an enable-time hook mutation fails, the tool remains enabled because the
user explicitly enabled it. The UI keeps the failed integration visible
through the orange toggle state and tooltip; it does not render a separate
inline save error or integration section.

When a disable-time mutation succeeds, only OrkWorks-owned entries are
removed. Foreign entries and unrelated configuration remain unchanged. If
disable-time removal fails, the tool becomes disabled as requested, but the
orange tooltip describes the remaining cleanup failure so the user can retry.
The next disable or repair operation must still remove only OrkWorks-owned
entries.

The integration status response remains the source of truth for installed,
drifted, unsupported, ownership, and diagnostic conditions. The operation
failure message is retained in the Settings view until the next successful
operation or the modal is reopened.

## Testing

Desktop tests should cover:

- enabling a supported tool requests hook installation as part of the save;
- disabling a tool requests ownership-aware hook removal;
- hook failure preserves the requested enabled/disabled transition and
  produces the orange warning state with the failure reason;
- an in-flight hook mutation presents the blue state and disables conflicting
  interaction;
- a successful repair or uninstall clears the warning state;
- the separate inline integration controls and inline save error are no
  longer rendered;
- tool detection status remains rendered independently of hook state.

Existing sidecar integration tests should remain authoritative for preserving
foreign configuration and removing only OrkWorks-owned entries.
