# Peon scope expands to per-repo

- Status: accepted
- Deciders: OrkWorks team
- Date: 2026-06-18

## Context

Peon was introduced as a per-session observer: one Peon attached to each terminal session, inferring metadata from terminal output. The Review Queue feature (see `specs/review-queue.md`) needs Peon-style inference for repo-scoped signal — plan and spec artifacts that exist outside any single session. Without a dedicated owner, there is no obvious place for digest inference to live: piggybacking on an arbitrary session Peon couples review health to that session's lifecycle, and adding a second non-Peon inference path duplicates harness selection, model detection, and capacity tracking.

## Decision

Introduce a second Peon scope: per-repo. A repo Peon is spawned automatically when an OrkWorks workspace is opened and killed when the last window for that workspace closes. It uses the same harness/model auto-detection as the session Peon and the same capacity accounting in `.orkworks/capacity/`. Its identity is marked `scope: "repo"` in surfaced metadata so existing consumers can distinguish without a special case.

Session Peon and repo Peon run concurrently. There is no shared lock between them; capacity tracking is the only shared state.

This keeps Peon's identity coherent — "low-cost observer of repo-level signal" — instead of fragmenting inference into multiple competing abstractions.

## Consequences

- One new place for inference to live, but it reuses Peon's existing harness, model detection, and capacity infrastructure.
- The repo Peon is auto-started, so opening a workspace incurs an inference budget even if the user never opens the Review Queue. This is acceptable because it rolls up into the existing capacity surface and remains observer-only.
- `SessionInfo`-shaped APIs gain a `scope` field; consumers default to `scope: "session"` when absent.
- Reaffirms ADR 0006 (Peon observer-only): repo Peon also never types into terminals, never modifies source, and is read-only over the file system.
- Future Peon scopes (e.g. organization-wide, cross-repo) are possible without further architectural changes — `scope` is the seam.
