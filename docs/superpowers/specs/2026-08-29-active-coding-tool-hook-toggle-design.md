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

The existing Settings Save action remains the commit boundary. The renderer
uses a new typed Electron-main operation for active coding-tool changes rather
than calling the active-harness sidecar route and hook IPC methods separately.
Electron main owns the orchestration and returns one result containing the
active-tool persistence result plus a per-tool integration result.

The Save operation follows this order:

- Toggling a tool changes the draft state.
- Save persists the requested active-tool set through the existing sidecar
  workspace route. If that persistence fails, no hook mutation is attempted
  and the draft remains unsaved.
- After active-tool persistence succeeds, Save reconciles every
  integration-capable tool: enabled tools are installed or repaired, and
  disabled tools with an OrkWorks-owned registration are uninstalled.
- Reconciliation runs even when a tool's active state did not change, so Save
  is also the retry/repair action for an already-enabled needs-you tool.
- The requested tool-state transition remains durable even if its hook
  mutation fails; the failed integration is represented by the needs-you toggle
  state and its tooltip so the user can retry.
- The result reports partial failures per tool. If any integration mutation
  fails, Settings remains open and does not show an inline save error; the
  affected toggles show their warning state. A fully successful Save may close
  the modal as it does today.
- Existing separate inline install, reinstall, and uninstall actions are
  removed.

Hook mutations continue to use the existing Electron-main and sidecar
authority boundaries. A conceptual result is:

```ts
type ActiveHarnessSaveResult = {
  activeHarnessesPersisted: boolean;
  integrations: Record<string, {
    ok: boolean;
    state: "healthy" | "warning" | "unsupported" | "unchanged";
    message?: string;
  }>;
};
```

The exact shared type may be duplicated across the preload boundary according
to the existing Electron contract convention. The renderer never gains direct
filesystem or hook-mutation authority.

## Toggle states

The toggle communicates integration state through its visual color and its
accessible name/tooltip:

| State | Toggle appearance | Meaning |
| --- | --- | --- |
| Disabled and clean | Neutral/off | Tool is unavailable and no OrkWorks-owned hook remains. |
| Enabled without hook support | Neutral/on | Tool is enabled, but this coding tool has no OrkWorks hook capability. |
| Enabled and healthy | Green | Tool is enabled and its OrkWorks hook is installed. |
| Enabled with failed or drifted hook | Needs-you blue | Tool is enabled, but the hook failed to install or needs repair. |
| Enabled with hook trust pending | Needs-you blue | Tool is enabled, but the coding tool must approve the hook before it can activate. |
| Disabled with failed cleanup | Needs-you blue | Tool is disabled, but an OrkWorks-owned hook remains because removal failed. |
| Integration status unavailable | Needs-you blue | OrkWorks cannot verify hook health; the tooltip gives the status-query failure. |
| Hook update/repair in progress | Neutral with spinner | OrkWorks is applying the hook mutation; no user action is required yet. |

The needs-you blue state is the exact existing `--attention-needs-you` color
used by the session view for “Needs you.” It consistently means the user must
take or retry an action. It covers retryable operation failures, detected
drift, trust approval, incomplete cleanup, and unavailable verification. The
in-progress state is neutral with a spinner so it does not imply either
healthy completion or required user action. A red state is not required by
this design.

The accessible label and native tooltip include the specific condition. For
example: “Enabled, but hook installation failed: permission denied.” The
toggle uses the needs-you color plus a non-color status glyph for warning
states, and a spinner for in-progress states, so color is not the only signal.
A successful install/repair or uninstall clears the warning state.

Coding-tool detection remains a separate status signal and must not be
conflated with hook health.

## Error handling and ownership

When an enable-time hook mutation fails, the tool remains enabled because the
user explicitly enabled it. The UI keeps the failed integration visible
through the needs-you toggle state and tooltip; it does not render a separate
inline save error or integration section. Save remains open so the warning is
visible and can be retried.

When a disable-time mutation succeeds, only OrkWorks-owned entries are
removed. Foreign entries and unrelated configuration remain unchanged. If
disable-time removal fails, the tool becomes disabled as requested, but the
needs-you tooltip describes the remaining cleanup failure so the user can retry.
The next Save still attempts cleanup and removes only OrkWorks-owned entries.

The integration status response remains the source of truth for installed,
drifted, unsupported, ownership, and diagnostic conditions. The operation
failure message is retained in the Settings view until the next successful
operation or the modal is reopened. Reopening reloads durable integration
status; operation-specific text is intentionally not persisted, so a reopened
modal may show the status diagnostic rather than the original failure wording.

Unsupported or limited tools remain ordinary active-tool choices and do not
claim hook health. They use the neutral toggle state with an accessible
description that no OrkWorks hook is available. Ownership ambiguity is never
treated as safe to remove; it produces a needs-you warning with an explanation
and leaves foreign configuration untouched. Tool detection, Codex hook trust,
and hook registration are separate facts: an undetected tool or Codex
`needs_trust` state must be described in the tooltip/status model rather than
silently presented as a healthy green hook.

The custom executable-path controls are preserved. They move out of the hook
integration section into a small per-tool command-path control in the row (or
an equivalent separate settings component); removing hook actions must not
remove the ability to save or clear a custom executable path.

The blue state is local renderer state for the duration of the orchestration
promise. Each tool has an independent reconciliation state, but the Save
action is disabled while the batch is running. Stale results from a closed
modal or changed workspace are ignored, and the next Settings open reloads
status from the current workspace.

## Testing

Desktop tests should cover:

- enabling a supported tool requests hook installation as part of the save;
- disabling a tool requests ownership-aware hook removal;
- saving with no active-state change retries reconciliation for enabled
  needs-you tools and incomplete disabled cleanups;
- hook failure preserves the requested enabled/disabled transition and
  produces the needs-you warning state with the failure reason;
- active-tool persistence failure prevents hook mutation and does not claim
  that the settings were saved;
- multiple tools return independent integration outcomes, including partial
  success;
- an in-flight hook mutation presents the blue state and disables conflicting
  interaction;
- a successful repair or uninstall clears the warning state;
- the separate inline integration controls and inline save error are no
  longer rendered;
- custom executable-path save/clear behavior remains available;
- tool detection status remains rendered independently of hook state.

Existing sidecar integration tests should remain authoritative for preserving
foreign configuration and removing only OrkWorks-owned entries.
