# Session State Injection Design

- Date: 2026-07-06
- Status: proposed

## Summary

Add a debug-only `State injection` control to the selected session's Details panel so a user can temporarily perturb one session into a known bad or transitional state and then watch the real system converge back to a correct state.

This is a testability feature, not a general metadata editor. The control is only visible when `Show debug metadata` is enabled, offers a dropdown of curated injection scenarios, and applies a one-shot mutation to both the in-memory session handle and the persisted session metadata record for the selected session. After the write, OrkWorks resumes normal behavior immediately; no timer, lock, or sticky override remains active.

## Goals

- Make it possible to manually test whether the app eventually converges to the correct session state after a deliberate perturbation.
- Exercise the real read model, metadata normalization, runtime finalization, and UI state selection logic together.
- Keep the feature narrow, explicit, and hard to trigger accidentally.
- Reuse the existing `Show debug metadata` gate instead of adding a second persistent debug setting.

## Non-Goals

- No free-form JSON or arbitrary field editor.
- No persistent override mode that stays active until manually cleared.
- No terminal/process control side effects such as kill, spawn, attach, or input.
- No provider-wide capacity mutation for the `capped` scenario.
- No mutation of sessions other than the currently selected session.

## UX

### Visibility

When `Show debug metadata` is off:

- do not render any injection control

When `Show debug metadata` is on:

- render a compact `State injection` block in the session Details panel

### Controls

The block should contain:

- a dropdown listing curated injection scenarios
- an `Apply injection` button
- a short explanation such as `Temporarily writes a debug state, then lets normal runtime and metadata logic overwrite it naturally.`

Applying an injection should show toast feedback naming the chosen scenario.

## Injection Semantics

An injection is a one-shot write, not a mode.

Rules:

- the backend applies the chosen scenario to both live in-memory session state and persisted metadata for the selected session
- the write happens once and returns immediately
- the injection does not pause Peon, background runtime tasks, lifecycle finalization, or metadata normalization
- the injection does not reapply itself
- once written, the normal system is expected to overwrite the debug state when later authoritative signals arrive

This feature intentionally allows some contradictory or impossible intermediate combinations because the purpose is to test convergence, not to simulate only domain-valid transitions.

## Provenance And Precedence

Plain `user` provenance is the wrong fit for this feature because the spec gives `user/manual override` the highest priority, which would make the injection sticky and block the natural convergence path being tested.

To preserve the ability to converge, introduce a dedicated metadata source for this feature:

- `debug`

Rules:

- injection writes set `metadataSource = "debug"`
- injection writes set `metadataConfidence = null`
- `debug` is lower priority than every normal runtime source: `user`, `agent`, `peon`, `backend_inference`, `process`, and `unknown`
- a later non-debug write may overwrite a debug-injected value immediately, without waiting for staleness windows
- UI source presentation should make the temporary nature clear by labeling the state as debug-derived

This source exists only so the system can visibly accept a temporary wrong state and then naturally escape it.

## Injection Catalog

The feature should ship with a fixed backend-owned catalog of scenario ids and labels. The renderer may display those labels, but the backend remains the authority on which injections exist and what fields they mutate.

Initial scenarios:

### `active_fake_ending`

Purpose:

- let the user watch whether finalization, recovery, and ended-state presentation correct a session that has been shoved into an ending-like shape

Mutation shape:

- set live and persisted session `status` to `running`
- set `lifecyclePhase` to `ending`
- clear live `observedStatus`
- set persisted `pendingTerminalStatus` to `ended`
- clear projected live `terminalOutcome`

### `ended_stale_live_attention`

Purpose:

- verify that ended sessions do not keep regaining live attention semantics from stale `observedStatus`

Mutation shape:

- set `status` to `ended`
- set `lifecyclePhase` to `ended`
- set `observedStatus` to a live-only value such as `waiting_for_input` or `blocked`
- preserve `finalObservedStatus` / final snapshot if present

### `ended_missing_final_snapshot`

Purpose:

- verify that normalization/recovery restores missing frozen historical state for terminal sessions

Mutation shape:

- set `status` to `ended`
- set `lifecyclePhase` to `ended`
- clear persisted `finalObservedStatusSnapshot`
- clear projected `finalObservedStatus`

### `running_blocked`

Purpose:

- verify that later real agent/peon/runtime signals can replace a temporary wrong live attention state

Mutation shape:

- set `status` to `running`
- set `lifecyclePhase` to `active`
- set `observedStatus` to `blocked`

### `running_idle_too_early`

Purpose:

- simulate an early false-idle classification and observe whether later output/runtime activity clears it

Mutation shape:

- set `status` to `running`
- set `lifecyclePhase` to `active`
- set `observedStatus` to `idle`

### `running_capped`

Purpose:

- verify capped presentation and subsequent clearing behavior without mutating provider-wide capacity state

Mutation shape:

- set `status` to `running`
- set `lifecyclePhase` to `active`
- set `atUsageLimit = true`
- set `usageLimitResetHint` to a clearly synthetic string such as `resets in 1h (debug)`

The `capped` scenario is session-scoped only. It must not mutate provider runtime state or shared capacity files.

## Backend Shape

Add a narrow debug endpoint for applying one injection to one session.

Properties:

- accepts `sessionId` and `injectionId`
- rejects unknown injection ids
- rejects requests when the session does not exist
- applies the backend-owned mutation recipe to the selected session only

Do not add a generic metadata write API. This endpoint stays deliberately narrow so the feature remains a convergence-testing tool rather than a maintenance backdoor.

### Catalog ownership

The backend should own the authoritative catalog of injection ids and labels.

Recommended shape:

- add a small read endpoint for listing supported injections
- add a write endpoint for applying one injection

This avoids drift between UI labels and backend behavior.

## Frontend Shape

Add the debug injection block to `SessionDetailPanel.tsx`.

Behavior:

- when debug metadata is hidden, render nothing
- when shown, fetch or receive the injection catalog and populate the dropdown
- send the active session id plus selected injection id to the backend
- rely on the normal session refresh/read path to reflect the injected state instead of keeping a separate client-only shadow copy

The feature should behave like all other real session changes: write through the backend, then read the resulting session model back through the existing app flow.

## Safety Rules

- no arbitrary field editing
- no background reapplication loop
- no mutation of provider-wide capacity state
- no automatic cleanup step after injection
- no override of multiple sessions at once
- no use outside the existing debug-metadata gate

If a proposed scenario would require terminal/process side effects or broad cross-session changes, it should be excluded from this feature and handled by a separate test harness design instead.

## Testing

### Rust

- catalog tests for known injection ids and labels
- endpoint test rejecting unknown injection ids
- endpoint test rejecting missing session ids
- per-scenario tests proving both live session state and persisted metadata are mutated as intended
- precedence tests proving later non-debug writes overwrite debug-injected state immediately
- `running_capped` test proving only session-scoped fields change, not provider/global capacity state

### Frontend

- Details panel test proving the block is hidden unless `Show debug metadata` is enabled
- test proving the dropdown renders the backend-supported scenarios
- test proving apply sends the selected injection id for the active session
- success and failure toast coverage

### Integration

- at least one end-to-end-ish regression around convergence: inject a temporary wrong state, then trigger the existing runtime/metadata path that should correct it, and assert the final visible session state matches the real system result rather than the injected one

## Docs And Board Follow-Up

- create a GitHub issue before implementation because this is new tracked work not yet represented on the board
- update the authoritative specs if implementation introduces the new `debug` metadata source into the supported metadata-source vocabulary
- update AGENTS/docs if the debug-source vocabulary becomes load-bearing beyond this feature

## Acceptance

This design is complete when:

- a selected session can be perturbed from the Details panel using a curated debug injection dropdown
- the injection is one-shot and affects both live and persisted session state
- normal runtime and metadata logic may overwrite the injected state immediately afterward
- the injected state is visibly marked as debug-derived
- the feature remains narrow and cannot be used as an arbitrary metadata editor
