# Simplified session lifecycle design

> **Date:** 2026-07-12  
> **Scope:** Separate process lifecycle from live-session attention so the frontend exposes only meaningful states.

## Goal

Replace overlapping frontend lifecycle vocabulary (`running`, `done`, `stale`, and the visible four-phase runtime lifecycle) with a small, explicit model. A session has a process lifecycle, and an alive session has a current attention state. These concerns must not be encoded in the same enum.

## State model

### Lifecycle

```text
creating -> alive -> stopping -> dead
```

- `creating`: launch has been requested but the harness process is not yet confirmed alive.
- `alive`: a harness process exists and can produce terminal output.
- `stopping`: the process has exited or is being killed; the sidecar is finalizing metadata and its final observer snapshot.
- `dead`: finalization is complete. The session has no live process and cannot receive live attention updates.

Lifecycle is backend-owned. `stopping` retains the current atomic finalization and recovery guarantees, but it is not a permanent user-facing state.

### Attention

An attention value exists only while a session is `alive`:

```text
working | idle | needs_you | blocked | failed | capped
```

- `working`: recent terminal output shows the process is making progress.
- `idle`: the process is alive but quiet.
- `needs_you`, `blocked`, `failed`, and `capped`: actionable alterations of idle. They replace the idle presentation until new terminal output returns the session to `working`.

Capacity polling is supporting data, not an additional attention state. `checking_capacity` is not exposed as attention. A confirmed capacity limit sets `capped`; new terminal output clears that capped attention, while the last reported capacity value remains diagnostic metadata. A later, fresh confirmed limit may set `capped` again.

`running` is removed: it duplicates `alive`. `done` is removed: session completion is represented by `dead` and its terminal outcome, while an unfinished quiet session is `idle`. `stale` normalizes to `idle`.

### Independent data

- `terminalOutcome` remains `ended | killed | error`. It is historical/debug detail, not a lifecycle or attention value.
- `memoryState` remains independent and controls resume availability only.
- Final observed-state snapshots remain backend history and recovery data. They never become live attention after the session is dead.

## Transitions

```text
creating -- spawn confirmed --> alive
creating -- launch failure ----> dead (terminal outcome: error)

alive -- terminal output -----> working
alive -- quiet ---------------> idle
idle -- attention observed ---> needs_you | blocked | failed | capped
any alive attention -- output -> working

alive -- process exit / Kill -> stopping
stopping -- final snapshot ---> dead
```

The transition from `alive` to `stopping` is idempotent. It durably records the pending terminal outcome and captures the last accepted observer snapshot before scheduling at most one bounded final scan. Scan success replaces the captured snapshot; scan failure or timeout finalizes with the captured snapshot. Once a session enters `stopping`, live attention is cleared and no later inference may restore it.

Startup recovery finalizes an orphaned `stopping` session by consuming its persisted pending terminal outcome and captured snapshot. A launch failure transitions directly from `creating` to `dead`, records the `error` outcome, and persists the canonical null final-observer snapshot. Therefore every `dead` session has a final snapshot even when no live observation occurred. `dead` is terminal for that runtime; resuming creates a new alive runtime using the applicable resume memory.

## Frontend presentation

- `creating` and `stopping` use a brief spinner and disable conflicting actions.
- An alive, working session uses an active indicator without a redundant “Running” label.
- An alive, idle session uses a quiet idle indicator.
- An alive actionable session surfaces its attention label: Needs you, Blocked, Failed, or Capped.
- A dead session is muted and offers Resume or Forget according to its resume capability. Normal UI does not show “Dead”, “Ended”, or “Killed”.
- Lifecycle, terminal outcome, and frozen observer state appear only in debug metadata or history.

The list continues to put alive sessions before dead sessions. Within alive sessions, actionable attention ranks above working and idle; dead sessions do not participate in live-attention ordering.

## API and migration

The API exposes independent `lifecycle` and `attention` fields. During a compatibility window it retains legacy `status`, `lifecyclePhase`, `observedStatus`, and `connectivity` fields for older desktop builds, but new frontend code uses only the new fields. A later protocol-removal change may delete the legacy fields after desktop/sidecar compatibility is no longer required.

During migration, persisted legacy values normalize as follows:

| Legacy value | New value |
| --- | --- |
| `creating` status / lifecycle phase | `creating` lifecycle |
| `running` status / `active` lifecycle phase | `alive` lifecycle |
| `ending` lifecycle phase | `stopping` lifecycle |
| `ended` lifecycle phase or terminal status | `dead` lifecycle |
| `working` observed status | `working` attention |
| `idle` or `stale` observed status | `idle` attention |
| `done` observed status | `idle` when alive; otherwise retained only in historical metadata |
| `waiting_for_input` observed status | `needs_you` attention |
| `blocked` or `failed` observed status | same-named attention |
| a confirmed capacity limit / `atUsageLimit` | `capped` attention |
| `checking_capacity` / `capacityCheckPending` | no attention value; retain only as capacity diagnostic metadata |

For any lifecycle other than `alive`, attention is omitted or cleared. New writes use only the new state vocabulary. Existing terminal outcome, resume, capacity, and final-snapshot data remain compatible through the migration.

## Verification

Tests must cover:

- lifecycle transitions, including launch failure and idempotent stopping;
- clearing live attention on stopping and preventing it from resurfacing;
- terminal output returning every alive attention state, including `capped`, to working while retaining capacity history as diagnostic data;
- normalization of `stale`, `done`, `blocked`, `failed`, capacity, and legacy lifecycle/status values;
- final-scan success, timeout/fallback, canonical-null launch-failure snapshots, and startup recovery of an orphaned stopping session;
- frontend rendering and sorting for each lifecycle and attention state;
- resume actions and muted presentation for dead sessions.
