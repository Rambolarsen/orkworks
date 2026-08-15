# Session runtime generation ownership

- Status: accepted
- Deciders: OrkWorks maintainers
- Date: 2026-08-14

## Context

Resuming a session retains its public session ID while replacing the PTY
runtime. Exit and finalization work runs asynchronously and previously used
only that ID, so a late callback from an old runtime could mutate the newly
installed runtime.

## Decision

Assign each installed PTY runtime a monotonically increasing, per-session run
generation. Pass that generation through exit, terminal-status, and
finalization paths. A path may mutate the in-memory handle or clear resume
ownership only when its generation still matches the installed runtime.

## Consequences

Late callbacks become harmless no-ops after a resume replaces their runtime.
The generation is runtime-only: it is not exposed through the HTTP API or
persisted metadata. Runtime lifecycle helpers must accept and verify the
generation whenever they act after an asynchronous boundary.
