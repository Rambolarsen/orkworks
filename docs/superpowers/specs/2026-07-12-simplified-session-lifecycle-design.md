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

The transition from `alive` to `stopping` is idempotent. It captures the last accepted observer state and schedules at most one final scan. Once a session enters `stopping`, live attention is cleared and no later inference may restore it. `dead` is terminal for that runtime; resuming creates a new alive runtime using the applicable resume memory.

## Frontend presentation

- `creating` and `stopping` use a brief spinner and disable conflicting actions.
- An alive, working session uses an active indicator without a redundant “Running” label.
- An alive, idle session uses a quiet idle indicator.
- An alive actionable session surfaces its attention label: Needs you, Blocked, Failed, or Capped.
- A dead session is muted and offers Resume or Forget according to its resume capability. Normal UI does not show “Dead”, “Ended”, or “Killed”.
- Lifecycle, terminal outcome, and frozen observer state appear only in debug metadata or history.

The list continues to put alive sessions before dead sessions. Within alive sessions, actionable attention ranks above working and idle; dead sessions do not participate in live-attention ordering.

## API and migration

The API exposes independent `lifecycle` and `attention` fields. It stops exposing `status`, `lifecyclePhase`, and their frontend fallback logic as presentation inputs.

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

New writes use only the new state vocabulary. Existing terminal outcome, resume, and final-snapshot data remain compatible through the migration.

## Verification

Tests must cover:

- lifecycle transitions, including launch failure and idempotent stopping;
- clearing live attention on stopping and preventing it from resurfacing;
- terminal output returning every alive attention state to working;
- normalization of `stale`, `done`, and legacy lifecycle/status values;
- frontend rendering and sorting for each lifecycle and attention state;
- resume actions and muted presentation for dead sessions.
