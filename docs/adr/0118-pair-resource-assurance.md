---
type: Architecture Decision
title: Pair resource limits cover owned file lengths and fixed arena storage
description: Accepted scoped reduction to logical file-length and fixed SQL arena capacities, with native host costs outside the numeric guarantees and execution still gated.
tags: [orkworks, hierarchy, windows, sqlite, resources, assurance]
status: draft
---

# Pair resource limits cover owned file lengths and fixed arena storage

- Status: accepted for further qualification; implementation and execution gated
- Deciders: repository owner, “keep going” on 2026-11-24 to the explicit #2034 guarantee question
- Date: 2026-11-24
- Tracking: [#1176](https://github.com/Rambolarsen/orkworks/issues/1176)
- Supersedes: [ADR 0112](0112-pair-native-enforcement.md), resource meanings and complete numeric E/S gate only
- Canonical contract: [accepted resource assurance](../superpowers/specs/2026-11-24-pair-resource-assurance-design.md#scoped-resource-assurance-acceptance--2026-11-24)

## Context

The retained callback/compiler/rounding evidence does not establish a complete
native-growth peak or complete outside-arena scratch bound. No impossibility
finding follows. The owner accepted an explicit reduction of the guarantee,
not a claim that the earlier resource proof was completed or always meant EOF.

## Decision

Keep the accepted same ordinary worker, attached rollback transaction, private
Windows/local-NTFS public VFS, early allocator router and original PairSlot/
PairCore lifetime. Replace only the resource contract with:

1. Hierarchy 4,096-byte pages and at most 1 GiB owned logical file-length
   high-water; no total native allocation ceiling.
2. One shared 2 GiB sum of logical file-length high-water charges across both
   participants' disk auxiliaries, including pending and uncertain extensions.
   Checked end/aggregate admission precedes I/O; shrinking does not refund.
   Reuse requires original-owner proof of complete auxiliary reclamation.
3. One fixed 8 MiB backing region for pair-routed SQL heap and application-owned
   variable SQL callback buffers, including alignment, bookkeeping, fragmentation
   and simultaneous copies. No default allocator fallback or migration.

Total native disk/memory costs, including NTFS rounding/internal allocation,
metadata/cache, native/OS workspaces and stacks, are outside these numeric caps
and may exceed them. The ordinary main file stays under its existing owner;
all lower queue/intent/source/work/retained-copy/materializer/handle/position
budgets remain separate and binding. No maximum valid command fit is promised.

The first resource qualification obligation is now complete coverage of the
actual controlled EOF-extension and allocation routes for the exact build,
with unknown controlled routing denied and outside native paths disclosed.
A complete numeric native E/S envelope is no longer required for this profile.
This does not qualify an implementation or permit relabeling an uncontrolled
application allocation as OS overhead. Use the canonical design's exact checked
charges, failure, recovery, arena, escape and startup restrictions.

## Consequences

Host disk exhaustion, memory pressure, process failure and collateral effects
on unpaired work remain possible. This profile is not host resource isolation.
Eligibility/build explanations must name the weaker assurance; host observations
are not guaranteed remaining capacity. Old evidence stays valid only in its
recorded scope and is not rewritten as proof of the new mechanism.

ADR 0112's other architectural/ownership decisions remain accepted, including
StoppedOccupied/Unknown, no restart-to-fresh, no spill, no allocator conflict
recovery by reset/replacement and unpaired allocator preservation. No source/
format/authority/cutoff/initializer/recovery/durability/activation gate changes.

Acceptance authorizes further qualification and its written planning, not backend
implementation, SQLite startup changes, SQL/database/native/model/compiler probes,
a renewed execution budget, source registration, migration or runtime activation.
Follow the [gated qualification plan](../superpowers/plans/2026-11-24-pair-resource-assurance-qualification.md).
#1176 remains open and the production profile remains unavailable.
