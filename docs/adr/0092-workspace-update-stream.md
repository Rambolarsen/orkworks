---
type: Architecture Decision
title: Workspace update stream with authoritative recovery
description: Use workspace-scoped changed-row WebSocket updates and shared recommendation invalidation with REST fallback.
tags: [orkworks, architecture, sessions, taskmaster]
status: stable
---

# Workspace update stream with authoritative recovery

- Status: accepted
- Deciders: repository owner
- Date: 2026-10-24

## Context

A measured 655-session workspace returns 1.32 MB and takes approximately 1.26
seconds per list fetch. A single output timestamp change currently transfers the
whole list. Shell and recommendations panel also poll recommendations separately.
See the [approved design](../superpowers/specs/2026-10-24-push-refresh-design.md).

## Decision

Use one workspace-bound versioned WebSocket with an initial complete session
snapshot, changed rows/removals, and recommendation invalidations. A global cursor
fences missing messages. Reconnection requires an authoritative snapshot. Reuse
the complete REST projection and existing client merge semantics. Share the
recommendation list across consumers; revalidate details through existing APIs.

Coalesce invalidations with a two-second projection throttle. Reconcile every
30 seconds for external/time-derived changes. Preserve current REST polling while
disconnected. The workspace watcher uses notify 8.2's native recommended watcher
(nonrecursive directory watches); error recovery rearms it and polling protects
unsupported or unavailable watchers. Linux, macOS and Windows integration tests
must verify atomic replacement and deletion behavior before release.

## Consequences

Idle workspaces avoid repeated full downloads; active output sends changed rows.
Active projection CPU remains a separate optimization. Git/cwd and other derived
fields may lag by 30 seconds. No stored-data migration or terminal ownership
change. Slow clients reconnect; incomplete projections cannot delete rows.
Existing REST/terminal authorization is unchanged; the new read-only socket rejects
non-loopback browser origins rather than relying on permissive HTTP CORS.

## Scoped supersession — 2026-11-01

[ADR 0103](0103-sqlite-session-storage.md) supersedes the complete-all-history
REST projection and initial stream snapshot after qualified format-2 activation.
Version 2 uses complete live rows plus bounded history pages/invalidation while
preserving authoritative recovery, incomplete-read protection and read security.
The accepted [storage contract](../../specs/session-storage.md) and
[delivery plan](../superpowers/plans/2026-11-01-sqlite-session-storage.md) govern
the replacement. Current format-1 source behavior remains until implementation
and activation; this does not claim a shipped SQLite migration.

## Amendment — 2026-11-19

The owner approved [independent status and enrichment refreshes](../superpowers/specs/2026-11-18-session-status-enrichment-design.md)
to address the active-projection CPU cost reserved in the consequences above.
Negotiated `status-enrichment-v1` subscribers receive separate typed status and
repository-enrichment messages under the existing cursor. One workspace-owned
publisher shares work across those subscribers. Status retains two-second
coalescing; enrichment starts every five seconds during recent output activity
and every 30 seconds when idle, with no overlap or unbounded catch-up queue.
A changed output timestamp on an existing row selects the active cadence for
ten seconds; ordinary metadata changes and open-session count do not. Explicit
refresh and known location/document mutations bypass the periodic wait. Expired display information is visibly stale, not
newly validated by a status event. Explicit refresh and document/action validation
stay fresh. Legacy clients and REST retain the complete projection and recovery.

This amendment authorizes source implementation, not a claim of runtime or
installer qualification. Idle/busy performance and mixed-version checks are
required before enabling the capability. ADR 0103's format-2/version-2 activation
and scoped supersession are unchanged; the new capability does not activate it.

## Amendment — 2026-11-22

The [approved startup design](../superpowers/specs/2026-11-22-adaptive-snapshot-startup-design.md)
makes the first adaptive snapshot authoritative for a complete, newly captured
session status list. It carries the matching enrichment state and check time.
Git and document details can arrive independently as enrichment deltas. A
subscription requests fresh status from the shared publisher. Explicit refresh
continues to request a full inspection. Retain the ordered cursor, current
action validation, legacy complete snapshots, and REST recovery.
