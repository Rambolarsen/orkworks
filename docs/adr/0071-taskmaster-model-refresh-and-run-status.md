# Taskmaster owns model refresh and workspace-scoped analysis status

- Status: accepted
- Deciders: repository owner
- Date: 2026-09-30

## Context

Taskmaster Settings currently presents provider model catalogs but has no live
Codex catalog, and its existing analysis readiness status does not report what
happened after an asynchronous analysis request was scheduled. Peon already has
provider discovery, but its public route and mutable provider definitions do
not provide Taskmaster's separate authorization and command-selection
contract. The installation-wide evaluation ledger also has a legacy global
`lastError`, which cannot identify a workspace or analysis attempt.

The owner-requested behavior and its detailed acceptance cases are recorded in
the [Taskmaster model refresh and analysis status design](../superpowers/specs/2026-09-29-taskmaster-model-refresh-status-design.md)
and issue [#676](https://github.com/Rambolarsen/orkworks/issues/676).

## Decision

Taskmaster live model discovery uses its own Electron-authorized sidecar
request. The sidecar validates a code-owned native profile and selects a typed
operation: Codex always starts the fixed `codex app-server --stdio` model-list
operation, and Ollama queries the Taskmaster request's draft URL. Neither path
resolves a mutable provider definition to select a model-list command. Claude
Code stays on its static catalog; custom providers keep static suggestions and
manual IDs. Discovery never invokes inference, applies settings, or changes
Peon state.

Persist Taskmaster analysis status in the existing installation-wide
`evaluations.json` ledger as one record per canonical workspace key. A record
contains an optional `activeAttempt` and the `latestOutcome`; a readable
selected workspace with neither is idle, while no selected workspace or an
unreadable ledger is unavailable. Persist queue,
running, failure, success, and interruption transitions under the existing
Taskmaster process/file locking discipline. Mark the attempt running before
context collection or prompt construction; failures after that point become
failed outcomes. Pre-evaluation skips clear the active attempt while retaining
the previous outcome. Clear an analysis failure only when the new result has
passed validation, been applied, and its successful cache update is persisted.

Expose a narrow `GET /taskmaster/run-status` sidecar route protected by the
existing Electron-only Taskmaster capability. It returns only the current
sidecar's selected workspace projection. Status reads are read-only. Recovery
may mark a persisted running attempt interrupted or clear a queued attempt only
after acquiring the same installation-wide analysis lease used by evaluators.
If another instance holds that lease, leave the record unchanged and retry on
the next workspace open or evaluation admission. Do not enumerate or coordinate
other OrkWorks instances.

Keep the legacy ledger `lastError` field readable for old files but stop
serializing new values or displaying it as analysis status. Keep knowledge
update errors independent. Bound persisted run error summaries to 512 UTF-8
bytes and remove control characters. Keep one active/latest record per
workspace; total record count grows with workspaces used, and aggregate
retention/cleanup is outside this decision.

## Consequences

Taskmaster model discovery reuses the existing Codex app-server and Ollama
transport mechanics without reusing Peon's HTTP endpoint or executing mutable
provider commands. The desktop gains a dedicated preload method for explicit
refresh and a narrow status method for Recommendations and Settings.

Analysis state survives renderer refreshes and sidecar restarts, distinguishes
provider readiness from execution outcome, and prevents a live evaluator in
another instance from being reported as interrupted. Workspace switching does
not expose the previous workspace's run record. The global ledger grows as new
workspaces are used; retention policy remains a separate future decision.
