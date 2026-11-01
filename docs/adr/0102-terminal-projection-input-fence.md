---
type: Architecture Decision
title: Slow terminal projections compute outside the input fence
description: Accepted capacity-analysis separation that preserves input ownership and rejects stale writes.
tags: [orkworks, architecture, terminal, concurrency]
status: draft
---

# Slow terminal projections compute outside the input fence

- Status: accepted
- Deciders: Rambolarsen
- Date: 2026-11-01

## Context

[#1222](https://github.com/Rambolarsen/orkworks/issues/1222) identifies a conflict
between ADR 0075's input fencing and session listing: capacity analysis runs
under the same global mutex required to admit each keystroke. The observed
request took 2.157 seconds, and the user reports burst/freeze typing.

## Decision

Implement the owner-approved [responsive input design](../superpowers/specs/2026-10-31-terminal-input-lag-design.md).
Keep ADR 0075's input preparation/enqueue and bookkeeping fence. Bound pure capacity analysis independently, capture immutable inputs under a
short fence, scan outside it, then validate runtime/control/capability identity
and a capacity control revision before applying results within the existing
projection. Ordinary output can advance counters during analysis: commit only
the scanned prefix and preserve newer counters. Other state changes reject the
session observation and retain its current committed capacity. Workspace changes
produce unavailable, never an authoritative empty list or stale provider writes.
Metadata and organization projection retain their existing fenced ordering. Reuse exact-key pure analysis and fail closed for
oversized/racing password observations before input admission.

This narrows the session projection locking implementation documented in the
architecture reference; it does not supersede ADR 0075, ADR 0041 or ADR 0092.
ADR 0065's observation-window policy remains unchanged. Synchronize
the canonical architecture/runtime instructions with the implementation; the implementation retains their input/lifecycle authority.

## Consequences

Typing no longer waits for capacity computation, including a cold scan of
large retained output. Remaining metadata/Git work inside the fence is unchanged
and must be measured after repair. A stale observation temporarily preserves
current committed capacity; continuous output alone does not starve list updates. Large or racing output may
conservatively suppress input-derived metadata while still delivering terminal
bytes. Capacity-control revision advancement must cover each state-machine mutator
and accepted input, including no-op rechecks and ABA state changes.
No schema migration, terminal restart, automatic retry or attention-authority
expansion is introduced. Rollback is a code revert and owner-controlled restart.

Owner acceptance: 2026-11-01, “go ahead, implement”, followed by “keep going”.
