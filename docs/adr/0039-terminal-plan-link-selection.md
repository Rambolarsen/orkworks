# Terminal plan link selection

- Status: accepted
- Deciders: OrkWorks maintainers
- Date: 2026-08-09

## Context

Plan discovery through terminal heuristics is unreliable, especially when a session writes in a linked worktree.

## Decision

Terminal plan links may submit only a clicked `sessionId` and `printedPath` through a dedicated preload IPC method. Electron validates primitive inputs and authenticates the sidecar request. The sidecar owns parsing, Git-worktree-family validation, persistence, content reads, and the fixed review prompt. This supersedes ADR 0025 only for this narrowly scoped selection method; no generic file-open or terminal-write API is introduced.

## Consequences

Users have an explicit reliable path to review visible plan links. The persisted reference must retain its worktree anchor and source and be revalidated before every read or PTY handoff.
