---
type: spec
status: authoritative
title: "Review Queue — Spec"
---

# Review Queue — Spec

Status: proposed
Date: 2026-06-18

## Motivation

Agentic sessions produce plans and specs faster than a human can review them. Without a centralized surface, plan/spec artifacts accumulate across sessions, repos, and feature branches, and the user ends up context-switching into each session to read what was written. The Review Queue gives the user one inbox-style surface for plan and spec artifacts, with a short pre-digest so deciding whether to read the full document is cheap.

This is an observability feature, not a workflow controller. It does not pause agents, gate merges, or rewrite artifacts. It surfaces them.

## In scope (v1)

- Detect new and modified plan/spec artifacts in the open repo.
- Show them in a docked Dockview panel in the desktop app.
- Generate a 3-bullet TL;DR per artifact using the repo Peon.
- Allow the user to mark entries as read or dismissed; entries persist across restarts.

## Out of scope (v1)

- Cross-repo aggregation. The queue is per-repo, scoped to the currently open OrkWorks workspace.
- GitHub PR or remote artifact sources. File-system only.
- Reviewer subagents producing critique passes (deferred to v2).
- Plan diffing between revisions (deferred to v2).
- Cross-session clustering of overlapping specs (deferred to v2).
- Pausing agents on plan creation (separate proposal).

## Sources

The repo Peon watches the following paths inside the open workspace:

- `docs/superpowers/plans/**/*.md`
- `specs/**/*.md`
- `.orkworks/sessions/*.json` — only when the session JSON contains `plan_ready: true` (new optional field added to the metadata protocol for cooperating agents).

Any newly-created file under those paths is enqueued. Any modification re-enqueues with a fresh digest, subject to a 30-second per-artifact debounce so active editing does not thrash inference.

## Digest

The digest is a 3-bullet TL;DR produced by the repo Peon. The Peon is given the full artifact text and a digest-specific prompt template distinct from its terminal-output prompt. If the digest call fails or the Peon is unhealthy, the queue entry is still shown with status `digest_pending` so the inbox is never blocked by inference health.

## Queue model

Each entry has:

- `id` — stable hash of `{path, first_seen_at}`.
- `path` — relative to the workspace root.
- `kind` — `plan`, `spec`, or `session_signal`.
- `first_seen_at`, `updated_at` — timestamps.
- `digest` — array of strings (3 bullets) or `null` if pending.
- `digest_status` — `pending`, `ready`, or `failed`.
- `status` — `pending`, `read`, or `dismissed`.

State is persisted to `.orkworks/review-queue.json` and loaded on app start. Dismissed entries remain in the file (filtered out of the panel by default) so re-modifying a dismissed artifact does not re-surface it unless the user opts in.

## API

- `GET /review` — list queue entries.
- `POST /review/:id/dismiss` — mark dismissed.
- `POST /review/:id/read` — mark read.
- WebSocket event `review.updated` — emitted on enqueue, digest-ready, or status change.

The frontend polls `GET /review` on the existing 2-second cadence and also subscribes to `review.updated` so instant updates do not wait for the next poll tick.

## Frontend panel

A new Dockview panel `review-queue`, registered in the existing panel registry and persisted through the panel persist/restore design already in flight. The panel renders entries as cards:

- Source path + kind + timestamps
- 3-bullet TL;DR (or a "Digesting..." placeholder)
- "Open" button — opens the raw artifact in a panel or external editor
- "Dismiss" button

The panel respects the same activity-based ordering as other inbox surfaces: most recent first, with `pending` entries above `read`.

## Repo Peon

A new Peon scope is introduced: per-repo, alongside the existing per-session Peons. See [ADR 0012](../docs/adr/0012-peon-repo-scope.md) for the scope decision.

- Lifecycle: spawned automatically when an OrkWorks workspace is opened, killed when the last window for that workspace closes.
- Harness/model: same auto-detection logic as session Peon.
- Identity in metadata: `scope: "repo"` so existing API consumers can distinguish without a special case.
- Inference budget: rolls up into the existing `.orkworks/capacity/` accounting.
- Coordination: no shared lock with session Peons; capacity tracking is the only shared state.

## Acceptance criteria

- [ ] Opening a workspace spawns a repo Peon; closing the last window for that workspace kills it.
- [ ] Creating a new plan or spec file produces a queue entry within one watcher tick.
- [ ] The 3-bullet TL;DR appears within one Peon inference round-trip.
- [ ] Editing an artifact twice within 30 seconds produces only one digest call.
- [ ] Dismissed entries remain dismissed across app restart.
- [ ] If the Peon harness call fails, the entry shows `digest_pending` and the panel remains usable.
- [ ] WS `review.updated` fires on enqueue and on digest completion.

## Non-goals reaffirmed

The Review Queue does not control git workflow, does not gate any agent action, does not modify artifacts, and does not capture or proxy any audio or terminal input. It is a read surface over file-system signal, consistent with the OrkWorks product boundary of "observe and recommend before it controls."
