---
type: Architecture Decision Record
title: Peon admission shares the session deletion fence
description: Serialize synchronous Peon scheduling writes with forgetting while keeping provider execution outside the fence.
tags: [peon, sessions, concurrency]
status: stable
---

# Peon admission shares the session deletion fence

- Status: accepted
- Deciders: repository owner
- Date: 2026-11-01

## Context

[#403](https://github.com/Rambolarsen/orkworks/issues/403) identified scheduling
IDs that can outlive session deletion and recreate Peon diagnostics, pending
labels or timer origins. Shared forget cleanup alone cannot remove work held
locally by the scheduler. The owner approved the
[concrete design](../superpowers/specs/2026-11-01-peon-scheduling-forget-design.md)
in this session with “go ahead” on 2026-11-01.

## Decision

Use the existing projection fence for private synchronous Peon selection and
attempt admission, and separately for idle-timer selection/origin repair.
Return owned attempts rather than unadmitted IDs. Release the fence before
hard-wrap reassembly, provider work/waits and idle-transition application.
Deletion uses that same fence and invalidates admitted attempts through the
existing runtime/attempt identity owner. No new lock, tombstone registry or
lifecycle authority is introduced.

Capture Peon map snapshots before acquiring sessions, then release sessions
before lease/in-flight mutations. Preserve ordinary end, input/title/output
generation guards, provider admission outcomes and existing stale settlement.
The fence covers synchronous scheduler writes, not atomic external process
creation or cancellation of an already-admitted provider.

## Consequences

Either admission finishes before deletion and its state is cleared, or deletion
finishes first and admission finds no session. Timer repair and label requeue
cannot reinsert forgotten tracking outside that boundary. Synchronous bounded
snapshot/admission work now briefly competes for the existing input/projection
fence; provider and pure reassembly work must remain outside it. No storage
migration or data repair is required to revert this decision. See the linked
design and [plan](../superpowers/plans/2026-11-01-peon-scheduling-forget.md) for
the required interleaving, replacement and liveness checks.
