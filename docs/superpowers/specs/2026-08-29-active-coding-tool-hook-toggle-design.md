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

The Tools subsection's Save action remains the commit boundary for active
coding-tool changes. The renderer uses a new typed Electron-main operation
for those changes rather than calling the active-harness sidecar route and
hook IPC methods separately.
Electron main owns the orchestration and returns one result containing the
active-tool persistence result plus a per-tool integration result.

The Tools Save operation follows this order:

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

The Settings modal has no overall footer-level Save, Cancel, or Restore
defaults section. Each subsection owns its own persistence and draft
lifecycle:

- **Coding tools** owns the active-tool draft, combined tool/hook Save, and
  tool-save warning states. Its Save is the retry action for incomplete hook
  reconciliation.
- **Model providers** keeps its existing provider-specific Apply and Save
  actions.
- **Hotkeys** owns its draft, Save, Reset, Restore defaults, and subsection
  Cancel/revert behavior.
- **Session retention** and **Debug** retain their existing immediate or
  field-level save behavior and status feedback.

Closing the modal through the title-bar close control discards unsaved drafts
in every subsection and does not perform a global commit. A subsection Cancel
or revert restores only that subsection's last committed values. A subsection
that has no staged draft does not need a Cancel control.

Hook mutations continue to use the existing Electron-main and sidecar
authority boundaries. A conceptual result is:

```ts
type ActiveHarnessSaveResult = {
  activeHarnesses: {
    outcome: "persisted" | "failed" | "stale_workspace";
    message?: string;
  };
  integrations: Record<string, {
    operation: "install" | "repair" | "uninstall" | "skipped";
    outcome: "succeeded" | "failed" | "unsupported" | "stale_workspace";
    registration: IntegrationRegistration;
    activation: IntegrationActivation;
    coverage: IntegrationCoverage;
    diagnosticCode?: string;
    message?: string;
  }>;
};
```

`skipped` is used for tools without an integration capability; it is not an
error. Mutation failure, status-query failure, ownership ambiguity, and Codex
trust requirement are distinct diagnostics even when they map to the same
user-action color. A stale-workspace result is never presented as a successful
Save. The exact shared type may be duplicated across the preload boundary
according to the existing Electron contract convention. The renderer never
gains direct filesystem or hook-mutation authority.

The operation captures the current workspace identity and backend generation
before persisting active tools. If the workspace changes before or during
reconciliation, remaining work is aborted and the result is
`stale_workspace`; the renderer ignores late results from the old workspace
and reloads the new workspace's active tools and integration statuses.

## Toggle states

The toggle communicates integration state through its visual color and its
accessible name/tooltip:

| State | Toggle appearance | Meaning |
| --- | --- | --- |
| Disabled and clean | Neutral/off | Tool is unavailable and no OrkWorks-owned hook remains. |
| Enabled without hook support | Neutral/on | Tool is enabled, but this coding tool has no OrkWorks hook capability. |
| Enabled with limited integration | Green/on | Tool is enabled and its limited OrkWorks integration is applied; the tooltip explains the limited coverage. |
| Enabled and healthy | Green/on | Tool is enabled and its OrkWorks integration is installed with no action-required diagnostic. |
| Enabled but absent | Needs-you blue/on | Tool is enabled, but its supported integration is absent and needs installation. |
| Enabled with failed or drifted hook | Needs-you blue | Tool is enabled, but the hook failed to install or needs repair. |
| Enabled with hook trust pending | Needs-you blue | Tool is enabled, but the coding tool must approve the hook before it can activate. |
| Disabled with owned registration | Needs-you blue/off | Tool is disabled, but an OrkWorks-owned integration remains and needs cleanup. |
| Disabled with failed cleanup | Needs-you blue | Tool is disabled, but an OrkWorks-owned hook remains because removal failed. |
| Integration status unavailable | Error color with offline glyph | OrkWorks cannot verify hook health; the tooltip says to retry the status check. |
| Integration operation in progress | Neutral with spinner | OrkWorks is applying an install, repair, or uninstall; no user action is required yet. |

The needs-you blue state is the exact existing `--attention-needs-you` token
used by the session view for “Needs you.” It consistently means the user must
take or retry an action. It covers retryable operation failures, detected
drift, trust approval, absent integrations, incomplete cleanup, and ownership
ambiguity. Status-query failures use the existing error token instead because
they indicate an OrkWorks/sidecar problem rather than a user-fixable hook
condition. The in-progress state is neutral with a spinner so it does not
imply either healthy completion or required user action. The token must be
used in both light and dark themes; no hard-coded color is introduced. A red
state is not required by this design.

The accessible label and native tooltip include the specific condition. For
example: “Enabled, but hook installation failed: permission denied.” The
toggle uses the needs-you color plus a non-color status glyph for warning
states, and a spinner for in-progress states, so color is not the only signal.
A successful install/repair clears the warning only when the returned status
has no action-required diagnostic. In particular, successful Codex
installation leaves the needs-you state in place while activation is
`needs_trust`; successful uninstall clears the warning only after the owned
registration is gone.

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

Unsupported tools remain ordinary active-tool choices, use `skipped`, and do
not claim hook health. Limited tools, including Aider, use their declared
limited integration capability and show green only when that capability is
successfully applied, with the limited-coverage explanation in the tooltip.
The UI derives participation from the resolved harness integration capability,
not from a second hard-coded harness-ID allowlist; retired Gemini behavior is
therefore governed by the resolved registry and existing selectable-harness
rules.

Ownership ambiguity is never treated as safe to remove; it produces a
needs-you warning explaining that OrkWorks will not modify the ambiguous
configuration. The tooltip identifies the safe recovery action: inspect the
workspace-local configuration, remove or reconcile the foreign entry outside
OrkWorks, then press Tools Save to retry. Tool detection, Codex hook trust,
and hook registration are separate facts: an undetected tool or Codex
`needs_trust` state must be described in the tooltip/status model rather than
silently presented as a healthy green hook.

The custom executable-path controls are preserved in a separate
`HarnessCommandPathControl` rather than being removed with the hook section.
They retain immediate Save/Clear behavior, keep path errors next to that
control, and remain available for command-template tools regardless of hook
support or coverage. Changing a path refreshes detection/integration status
but does not silently install a hook; if an integration operation is running,
the path control is disabled until it completes.

The in-progress state is local renderer state for the duration of the
orchestration promise and is named “integration operation,” not “hook update.”
The toggle keeps its draft on/off position while showing the neutral spinner.
Each tool has an independent final result, but the Tools Save action is
disabled while the batch is running. Stale results from a closed modal or
changed workspace are ignored, and the next Settings open reloads status from
the current workspace.

Warning precedence is: current operation failure, current status diagnostic,
then any older local warning. A successful operation for one tool clears only
that tool's warning; other tools retain their own warnings. Status-query
failures show the error presentation and a “Retry status check” tooltip/action
without changing the tool's active draft.

The stable accessible name remains the coding-tool name. The state and reason
are exposed through `aria-describedby` on a visible status description, with a
native `title` tooltip for pointer users. Warning uses a warning glyph and
text, trust-pending uses a hook/trust glyph and text, and in-progress uses a
spinner with “Integration operation in progress.” The toggle remains
`role="switch"`; it is disabled during the batch, preserves keyboard focus,
and uses the shared token's light/dark theme values with sufficient contrast.

## Testing

Desktop tests should cover:

- enabling a supported tool requests hook installation as part of the save;
- disabling a tool requests ownership-aware hook removal;
- saving with no active-state change retries reconciliation for enabled
  needs-you tools and incomplete disabled cleanups;
- enabled-but-absent, disabled-but-owned, unsupported, limited, and status
  unavailable states;
- hook failure preserves the requested enabled/disabled transition and
  produces the needs-you warning state with the failure reason;
- Codex installation success with `needs_trust` keeps the needs-you state;
- active-tool persistence failure prevents hook mutation and does not claim
  that the settings were saved;
- multiple tools return independent integration outcomes, including partial
  success;
- workspace changes during reconciliation produce stale results and do not
  report success for the old workspace;
- an in-flight hook mutation presents the blue state and disables conflicting
  interaction;
- a successful repair or uninstall clears the warning state;
- the separate inline integration controls and inline save error are no
  longer rendered;
- the modal has no overall Save, Cancel, or Restore defaults footer;
- Tools owns its Save and warning lifecycle, while Hotkeys owns Restore
  defaults and subsection revert behavior;
- custom executable-path save/clear behavior remains available;
- custom-path operations remain independent and are disabled while integration
  operations run;
- warning precedence and per-tool warning retention when another tool
  succeeds;
- accessible descriptions, native tooltips, status glyphs, keyboard focus,
  and both theme token variants;
- tool detection status remains rendered independently of hook state.

Existing sidecar integration tests should remain authoritative for preserving
foreign configuration and removing only OrkWorks-owned entries.
