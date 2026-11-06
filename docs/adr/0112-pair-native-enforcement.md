---
type: Architecture Decision
title: Original Windows pairs use owned VFS admission and pair-routed SQL allocation
description: Accepted public Windows pair VFS, early allocator router and conservative availability restrictions for further envelope qualification; implementation and execution remain gated.
tags: [orkworks, hierarchy, sqlite, windows, ownership, resources]
status: draft
---

# Original Windows pairs use owned VFS admission and pair-routed SQL allocation

- Status: accepted for further qualification; envelopes, implementation and execution gated
- Deciders: repository owner, “keep going” on 2026-11-06 to the explicit #1469 package question
- Date: 2026-11-06
- Tracking: [#1472](https://github.com/Rambolarsen/orkworks/issues/1472), [#1176](https://github.com/Rambolarsen/orkworks/issues/1176)
- Canonical contract: [pair native enforcement](../superpowers/specs/2026-11-06-pair-native-enforcement-design.md#scoped-architecture-acceptance--2026-11-06)

## Context

The stock-win32 handle component does not supply admission before auxiliary
creation/growth/deletion or a pair-local SQL scratch owner. The namespace/resource
audit identified those missing mechanisms. The reviewed #1469 proposal supplies
one concrete VFS/router candidate and makes justified growth/scratch envelopes
the first qualification gate, before dependent implementation planning.

## Decision

Accept the exact [#1469](https://github.com/Rambolarsen/orkworks/pull/1469) package,
head `e86d447cad43611c770b7f47dcb1ec3c75bfc78b`, merge
`50ba4dc329b9a60b51bc6adf25229b6ff0c98d1b`, **for further qualification only**:

1. One private public-interface Windows pair VFS owns rooted native I/O, original
   main associations and exact auxiliary creation/deletion.
2. One early SQLite allocator router assigns the actual pair worker's SQL
   allocations to its fixed pair arena, delegates unpaired allocations and
   retains provenance/lifetime through free/realloc/escape and unknown outcomes.
3. The stated conservative startup, no-disk-spill and retirement restrictions
   apply. An allocator conflict or unclosed bound makes the profile unavailable;
   no restart, allocator replacement or larger allowance forces availability.

This partially supersedes ADR 0111's stock-win32/no-custom-VFS candidate selection
for this qualification direction. Its original PairSlot/PairCore/same-worker,
StoppedOccupied/Unknown, source/newness and process-loss rules stay accepted.
The shared 2 GiB auxiliary total, separate 8 MiB SQL scratch and every lower
ordinary/hierarchy/source budget are unchanged.

The closed #1470 alternative is **not accepted**: no private symbol-isolated
SQLite build, whole-copy allocator selection or its extra handle/path/callback/
work defaults follows from this decision. The default SQLite instance is the
one described by #1469's early router; this is no process-global 8 MiB heap cap.

## Consequences

The no-custom-VFS exclusion is replaced only for this candidate's qualification.
Earlier stock-win32 evidence stays valid in its narrow scope and is not evidence
for the new VFS. Unpaired allocator behavior and startup integration require their
own proof. No maximum-profile fit or concrete arena capacity is established.

The first gate is a complete justified growth/scratch envelope for the exact
build and closed feature set, including rounding, native/stack/callback workspaces,
recovery inputs and allocator escapes. Its `A + S <= 8 MiB` condition must use
proved capacities; unknown `S` is not zero. The
[first source component derivation](../validation/pair-native-envelope-derivation.md)
is explicitly partial and cannot substitute for that manifest.

No dependent VFS/allocator installation, backend implementation, SQL/database,
migration, workflow dispatch, native/model probe, source registration, qualified
profile or runtime activation is authorized. All existing execution budgets and
separate source/cutoff/restart/durability/initializer/ordinary-activation gates
remain. Record unavailable and stop this candidate if the unchanged limits cannot
be justified; do not request the same architecture acceptance again.
