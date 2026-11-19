---
type: Validation
title: Offline ordinary-review source components
description: Canonical registry encoding and serialized logical admission traces with explicit source and durability limitations.
tags: [orkworks, review, qualification, validation]
status: stable
---

# Offline ordinary-review source components

The owner accepted the [source contract](../superpowers/specs/2026-11-19-review-source-qualification-design.md)
and [bounded plan](../superpowers/plans/2026-11-19-review-source-components.md)
on 2026-11-19 with “keep going”. This evidence covers offline logical components
for [#1899](https://github.com/Rambolarsen/orkworks/issues/1899) and
[#68](https://github.com/Rambolarsen/orkworks/issues/68), not production admission.

## Reproduction and independent expectations

Under the pinned Node 22, run:

```sh
node --test docs/validation/ordinary-review-source-components.test.mjs
```

The implementation is [ordinary-review-source-components.mjs](ordinary-review-source-components.mjs).
The [literal vectors](ordinary-review-source-vectors.json) retain independently
hand-derived compact registry bytes and three original/retry/restart traces with
expected states, record/envelope revisions and simulated invocation counts.
The [tests](ordinary-review-source-components.test.mjs) also exercise mutations,
closed field/state domains and finite boundary constructions. No previous oracle
or fixture bytes are changed. Initial tests failed with the implementation absent;
later state/shape tests caught missing envelope increments, optimistic replay
after uncertain publication and an array masquerading as a string state.
Fresh review found sparse arrays, truthy/default validation and exhausted-revision
creator authority; four regression tests failed before fixes and then passed.
The current focused suite passes **91 tests**, with **15/15** deliberate guard
mutations detected in temporary copies.

## Checked logical boundaries

- Exact synthetic owner, epoch, branch, kind, anchor, pin and selection binding;
  zero epoch, pre-exhaustion maximum, new owner, observed A→B→A and conservative
  unrelated publication invalidation. Equal unobserved history is not invented.
- Fatal UTF-8 decoding, closed shapes, fixed-key encoding and byte equality;
  duplicate/escaped/reordered keys, whitespace/BOM, malformed identifiers,
  unsupported schema/state/reason, copied workspace, duplicate attempt/session
  identity, occupied source, noncanonical/overflow counters and raw byte limits.
- Valid maximum u64/identifier domains and 256 retained attempts, with denial
  of 257. A maximum closed attempt is **701 bytes**; the maximum 256-record
  envelope is **179,859 bytes**, including separators and maximum revision.
  These are below 4 KiB/2 MiB, so no valid byte-cap equality witness exists in
  this schema. Raw input beyond 2 MiB is rejected before decoding.
- Serialized original/repeated Start, exact tuple conflict, retained identity
  before expired/lost preview, original draft preservation, source-slot conflict
  across recommendation changes, final stale/configuration/source rejection,
  linked reply loss, restart and qualified logical settlement/evidence.
- Before/after every modeled reservation, dispatch, link, rejection, settlement,
  recovery and evidence publication, plus creator entry/exception. An uncertain
  publication preserves either selected snapshot and fences all admission and
  replay results; successful readback/restart does not clear that fence.
- Forward-only states/revisions, terminal states, exhausted counters, saturation
  retaining exact retry, and both source-change/admission orders.

Fifteen temporary-copy mutations were each detected by the existing tests: removing
exact retry tuple, retained lookup, occupied-source exclusion, dispatch-before-
entry, single entry, final validation, link-after-entry, publication fencing,
exact evidence identity, owner-loss recovery, strict preview flag, explicit
final validation, exhaustion fencing, sparse-array validation or state type. The repository module was never
mutated by that negative-control run; temporary copies were deleted afterward.

## Qualification limits

This is an immutable **serialized single-writer model**. Each operation consumes
the latest returned model. Old snapshots are not live authority and must not be
reused to invoke the simulated callback. It does not prove mutual exclusion,
concurrent HTTP behavior, callback reentrancy safety or a production one-shot
capability. The callback only counts synthetic creator entry; it does not spawn.

Synthetic owner strings are not Store reference custody; selection IDs and
prepared-input digests are fixture identities, not source production or complete
launch-input encodings. Validation/evidence inputs are simulation assumptions,
not accepted runtime API booleans. Publication faults select logical snapshots;
there is no filesystem write, flush, replacement, crash or durability witness.

Actual Store capture/fences, private prepared creation with exact reserved ID
and tool/model/workspace, original input digest, authenticated attempt ownership,
registry initialization/recovery, retention/GC/export/backup/downgrade and future
storage cutover remain unqualified. No application code imports this component.
No application metadata, native process, provider call, producer permission or
capability activation is introduced. Production admission remains Investigate.

## Repository verification

`bash scripts/verify-repo.sh` passed under Node 22 after implementation: 3,603
Rust tests plus integration suites, 31 reporter tests, native ABI check, desktop
type-check/build and 1,786 passing tests (5 skipped), and all 2,147 docs checks
with site build. Rust ignored 22 tests. Whitespace, documentation currency and
worktree checks passed. Existing compiler/build chunk warnings are outside this
diff; foreign worktree warnings were preserved for their owners. This broad
regression check does not expand the qualification limits above.
