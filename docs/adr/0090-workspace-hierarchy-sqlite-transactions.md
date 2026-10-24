---
type: Architecture Decision
title: Workspace hierarchy transactions use one SQLite store
description: Accepted workspace SQLite/private-worker direction for further design and qualification, with detailed limits and runtime approval still gated.
tags: [orkworks, architecture, hierarchy, sqlite]
status: draft
---

# Workspace hierarchy transactions use one SQLite store

- Status: accepted architecture direction only; detailed design and numeric limits remain proposed
- Deciders: owner, scoped acceptance on 2026-10-24
- Date: 2026-10-23
- Tracking: [#746](https://github.com/Rambolarsen/orkworks/issues/746), [#744](https://github.com/Rambolarsen/orkworks/issues/744)
- Contract: [transaction/storage proposal](../superpowers/specs/2026-10-23-hierarchy-transaction-storage-design.md)

## Context

The accepted hierarchy recovery behavior requires source heads, receipts,
dependency protection and summary lineage to recover together. Existing data-only
coordinator files and recommendation-specific graph manifests do not establish
that transaction. The sidecar already includes bundled rusqlite. Ordinary session
metadata and runtime controls retain their existing persistence and authority.

## Decision

Use one workspace-local SQLite store as the accepted direction for new hierarchy
sources, source-owned receipts, dependency indexes, summary rows/fences and
ordered pending lineage.
A private worker is the accepted ownership direction. The detailed design below
remains subject to its qualification and implementation gates: commit typed
producer commands as one transaction. Use rollback DELETE journaling with
verified EXTRA synchronization and platform-qualified
flush behavior. Publication fences hide incomplete/ambiguous work; ordinary
session/terminal controls continue. Do not dual-write JSON mirrors or migrate
existing data-only approvals into runtime grants.

Keep exact canonical records/digests and each producer's authorization, replay,
quota and retention rules. The linked design proposes finite main-file, active
journal, command, queue and checkpoint limits with explicit adapter no-go gates.
Only proof-preserving atomic checkpoint compaction may remove applied pending
transitions. No read initializes lost summary history or restores authority.

## Consequences

One database transaction can own the durable source/summary change without a
new cross-file redo protocol. It adds a storage/module integration obligation,
including ordinary-writer fencing and qualified journal/scratch enforcement.
The acceptance covers the architecture direction for further design and
qualification preparation. Repository-wide migration, shared-learning custody,
platform qualification and runtime implementation retain separate gates.
Independent stores and ordinary session history remain unchanged. Detailed
design/default acceptance, producer agreement, strict schema/fixtures, native
evidence and separate executable plans are still required. No same-user rollback
protection is claimed.

## Architecture review — 2026-10-24

The [owner-review handoff](../superpowers/specs/2026-10-23-hierarchy-transaction-storage-design.md#architecture-owner-review-handoff--2026-10-24)
clarifies caller cancellation, terminal ordinary-generation exhaustion and which
proposed limits lack stock enforcement. The two-second SQL value is a cooperative
cancellation threshold, not a hard return/rollback guarantee. Auxiliary-file,
scoped scratch/row accounting, authority/commit ordering and native durability
proof remain no-go conditions before an executable handoff. No global allocator
change or custom VFS is implied. This review does not accept this ADR, qualify
a platform or authorize runtime implementation.

## Scoped owner acceptance — 2026-10-24

After the PR #1028 review, the owner answered “go ahead” to the recommendation
to accept one selected-workspace SQLite database/private worker with rollback
DELETE/EXTRA journaling **for further design and qualification only**. Reviewed
source: `9aa0a417c19cdf449925565dfcec803537c46d37`; this record is based on
`153d7f78dbc40c3fd0c84ab243e714f887f599e2` with unchanged reviewed storage files.

This accepts that direction, not the whole detailed storage/module contract or
its numerical defaults. Main/journal/scratch/complete-command and other limits
remain proposed pending enforcement proof and separate acceptance. In particular,
payload bytes must fit with all metadata under the eventual complete-command
budget. No profile, probe, runtime plan, migration or implementation is approved.
The qualification register and #740/#744/#745/#746 gates remain binding.
