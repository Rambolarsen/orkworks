# Session Startup Finalization Design

## Goal

Close the remaining lifecycle hole in resumed and newly created sessions: a delete or
request cancellation during runtime startup must not leave a generation permanently
`ending` or roll its live runtime back out of the registry.

## Evidence

`start_session_runtime` waits briefly for an initial terminal resize before spawning
the PTY. A delete can move that generation to `ending` in that wait. Startup then
spawns and kills the child when its guarded `running` transition is refused, but
returns before a driver exists to invoke terminal finalization. The handler's error
path currently schedules finalization only when it can transition the handle to
`error`; an already-ending handle refuses that transition, so no finalizer runs.

The resume admission guard is also still held while startup performs the PTY setup.
Cancellation after a child is spawned but before startup returns can roll back the
new handle even though the child and its side tables are live.

## Options

1. Make startup own terminal finalization for every failed setup path. This repeats
   terminal-lifecycle policy at each failure point and is easy to drift.
2. Expose an explicit startup-ready phase and transfer admission ownership only then.
   This is the strongest separation but expands the runtime API and is more invasive.
3. Keep the existing generation-scoped lifecycle owner, commit admission at the first
   irreversible PTY side effect, and route an already-ending startup failure through
   the existing generation-guarded finalizer. This is the smallest change and keeps
   one finalization mechanism. **Chosen.**

## Design

- Split the resume admission guard's rollback boundary from the full startup call:
  rollback is allowed only before a child can exist. Once the PTY startup has crossed
  its irreversible boundary, the runtime generation remains registered and terminal
  cleanup owns it.
- When startup observes that its own generation is already `ending`, clear its
  side tables and schedule the existing generation-guarded terminal finalizer rather
  than trying to overwrite the terminal state with `error`.
- Preserve the stale-generation rule: a mismatched generation does no cleanup or
  persistence work.
- Use portable test process helpers (rather than a POSIX shell script) for the stale
  handle regression so the suite runs on Windows too.

## Tests

1. Delete a session while it waits for the initial resize; assert it reaches a
   terminal state and releases resume admission.
2. Cancel the resume handler after the runtime crosses its startup boundary; assert
   the live generation remains registered and finalizes normally.
3. Keep the fake resumed process fixture portable on supported test targets.

## Non-goals

- No persisted schema or public API changes.
- No changes to the existing runtime-generation ownership model in ADR 0041.
