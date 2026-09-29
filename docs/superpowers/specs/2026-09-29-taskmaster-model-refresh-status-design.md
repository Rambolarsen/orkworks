# Taskmaster model refresh and analysis status

Date: 2026-09-29
Status: proposed for user review
Tracking issue: [#676](https://github.com/Rambolarsen/orkworks/issues/676)
Parent: [#503](https://github.com/Rambolarsen/orkworks/issues/503)

## Goal

Make it easy to select a model supported by the selected Taskmaster provider and
make analysis failures visible where recommendations are used. Taskmaster stays
independent of Peon's selected provider, inference state, and settings.

## Current behavior

Taskmaster Settings fills its model datalist from the selected provider's static
catalog. Codex advertises the `codex-app-server` model capability rather than a
static list, so the Taskmaster list is empty and users must enter a model ID.
The existing Peon Settings flow already supports an explicit **Refresh models**
action and obtains Codex models through `app-server model/list` and Ollama models
through its configured local endpoint.

Taskmaster's `analysisStatus` reports configuration/provider readiness. It does
not describe the outcome of an analysis run. The Analyze now endpoint returns
`scheduled` before the asynchronous evaluator runs. A single installation-wide
`lastError` may be shown in Settings, but it does not identify the workspace,
attempt, provider, or time. The Recommendations panel can therefore say a run
was scheduled without later exposing its failure.

## Approaches considered

1. **Keep model IDs manual and surface only the existing error string.** This
   avoids API changes, but Codex remains undiscoverable and failures stay
   detached from a specific attempt.
2. **Call the Peon discovery endpoint from Taskmaster.** This reuses its public
   sidecar route, but that route is not protected by Taskmaster's
   Electron-only authorization and can use the Peon selection/connection path.
   It also permits provider-definition model-list commands that Taskmaster does
   not intend to run.
3. **Share the provider-discovery implementation behind a Taskmaster-specific
   authorized route and persist a workspace-scoped run result.** This reuses
   established Codex/Ollama mechanics while keeping selection, draft connection
   settings, and authorization separate. It gives the UI one durable result to
   render. This is the selected approach.

## Model suggestions and refresh

The Taskmaster model control keeps provider-specific suggestions and an editable
custom model ID. Changing provider clears the draft model as it does today.
Opening Settings does not execute discovery.

Codex and Ollama expose a user-triggered **Refresh models** action:

- Codex reads the available model list using its existing app-server
  `model/list` transport.
- Ollama reads installed models from the Taskmaster selection's draft base URL.
- Claude Code displays its built-in static model catalog; it has no separate
  live listing operation in this Taskmaster flow.
- Custom providers keep static suggestions and manual entry. Refresh does not
  execute a custom model-list command.

The renderer calls a narrow preload method. Electron main sends the request to a
new Taskmaster-specific sidecar handler authenticated with the existing
Electron-only Taskmaster token. The handler validates the requested provider
against the Taskmaster catalog and uses the code-owned native profile, not an
arbitrary provider ID or the Peon selection. For Codex and Ollama, a shared
provider-discovery implementation serves both Peon and Taskmaster. The Taskmaster
call receives its own draft provider and URL and performs catalog discovery
only: no inference, Apply, settings persistence, or Peon state mutation.

The control shows request progress, refreshed suggestions, and a concise error
when discovery fails. A stale response from a provider or URL that the user has
since changed is discarded. The saved model remains available for manual entry
even when it is absent from the refreshed catalog.

Peon's current verification, refresh, cache, Apply, and Save flow remains
unchanged. The shared implementation is internal; Taskmaster does not call the
Peon HTTP endpoint.

## Analysis run status

Keep provider/configuration availability separate from runtime outcome. Model
the latest Taskmaster analysis attempt for the selected workspace with these
states:

- `queued`: an admitted request is waiting for evaluator work to begin.
- `running`: the evaluator is collecting context or invoking the selected
  provider.
- `succeeded`: the provider result passed validation and was applied.
- `failed`: a dispatched attempt failed, including context, provider, response,
  or result-application errors.
- `interrupted`: the sidecar restarted while the persisted attempt was still
  running.

An `idle` display means the workspace has no recorded analysis attempt. A
background cache hit, cooldown, disabled setting, unavailable provider, or
other decision that does not dispatch a run does not overwrite the latest
actual outcome. Existing manual request responses continue to explain admission
blocks such as an active recommendation or another in-flight analysis.

Persist a bounded latest-run record per canonical workspace key with the state,
start and completion timestamps, trigger kind (manual/background), provider,
model, and a safe error summary when applicable. Write state transitions through
Taskmaster's existing durable ledger and locking discipline. When a sidecar
starts or opens its owned workspace, recover a persisted attempt left in
`running` as `interrupted` before background scheduling begins. Status queries
are read-only and do not infer that a live attempt is interrupted. Do not
include another workspace's status in the renderer response, enumerate other
instances, or add cross-instance analysis coordination.

After a successful result, clear or supersede that workspace's previous
analysis error. Keep knowledge-update errors in their existing independent
status. Provider and error strings are bounded and rendered as text, not markup.

The Recommendations panel displays a compact inline status beside Analyze now,
with a useful failure summary and timestamp. Settings shows the full latest
attempt, provider/model, and error detail alongside provider readiness and
knowledge-update status. The UI follows its normal status/recommendation polling
and does not open a modal, show a background popup, or change focus.

## Boundaries

- No custom-provider live model discovery in this increment.
- No automatic model routing or provider fallback.
- No Peon selection changes, test inference, or automatic Peon refresh changes.
- No peer-instance status dashboard or coordination.
- No new global notification surface.

## Validation scenarios

- Provider changes update suggestions; manual model IDs remain usable.
- Settings opening causes no model-list subprocess or network request.
- Codex/Ollama Refresh uses Taskmaster draft selection, discards stale replies,
  and leaves Peon state untouched.
- Claude uses static suggestions; custom providers do not execute discovery
  commands.
- A Taskmaster-authorized request is required for live model discovery.
- Queued/running/terminal outcomes appear in both UI surfaces and survive a
  renderer refresh; a restarted in-flight attempt becomes interrupted.
- Failures identify the current workspace's attempt; a later success clears the
  old analysis failure without clearing a separate knowledge error.
- Cache/cooldown/eligibility skips do not replace the latest actual attempt.
- Workspace switches do not show the previous workspace's run details.

## Implementation gate

Before code, record the sidecar request and run-status protocol decision in an
ADR, then create an implementation plan from this reviewed design. The existing
authoritative Taskmaster spec has been updated alongside this design; issue #676
tracks the work.
