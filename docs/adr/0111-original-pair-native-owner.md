---
type: Architecture Decision
title: Original pairs retain one native owner and exact handle binding
description: Accepted PairSlot/PairCore lifetime and Windows NTFS native-handle candidate for further qualification, with explicit stopped failure and restart limits.
tags: [orkworks, hierarchy, storage, native, ownership]
status: draft
---

# Original pairs retain one native owner and exact handle binding

- Status: accepted for further qualification; stock/no-custom-VFS candidate selection partially superseded by [ADR 0112](0112-pair-native-enforcement.md); native execution/implementation gated
- Deciders: owner, “keep going” on 2026-11-06 after #1451
- Date: 2026-11-06
- Tracking: [#1453](https://github.com/Rambolarsen/orkworks/issues/1453), [#1176](https://github.com/Rambolarsen/orkworks/issues/1176)
- Canonical contract: [original pair native owner](../superpowers/specs/2026-11-06-original-pair-owner-design.md#scoped-owner-protocol-acceptance--2026-11-06)

## Context

The ordinary single-file adapter and workspace lease do not establish original
pair-file identity across SQLite pathname opens or paired old/new/unknown
initialization. A request-owned lifetime or unqualified failed-path maintenance
handoff could release the unique position or admit a new attempt too early.

## Decision

Reuse the exact #1451 proposal at head
`fe42ba2337843404c8712fa6854659910d19ce9d` / merge
`12ddf37d243f2e5a690ec8525d286123c074a62d` for further qualification.
PairSlot is the stable coordinator lifetime root and owns the unique position.
One non-Clone PairCore moves to the retained ordinary worker before native
creation; the same core/handles/connection/lease/resources becomes Serving in
place, without a request-owned or second-worker transfer.

Select the Windows/local-NTFS + bundled win32 VFS documented handle query and
full native identity comparison as a qualification candidate. Keep original
creation/namespace guards; no private SQLite layout cast, custom VFS, path-only
fallback or new persistent journal. Unix/macOS binding remains unselected.

Creation side effects after proven logical rollback enter StoppedOccupied and
retain the original slot/core/resources. Unknown retains the original owner too.
No failed-reservation handoff or startup/restart-to-fresh/recovery-to-live-owner
route is supplied. A separate qualified durable-newness/maintenance/restart
protocol is still required; matching empty paths or new epochs cannot supply it.

## Consequences

Conservative stopped availability and retained resource costs are accepted for
qualification, not proven feasible. Actual native API/namespace/auxiliary/
durability, strict source cutoffs, resource bounds and restart evidence remain
gates. This complements ADRs 0106–0110 and changes no source codec, logical quota,
seed schedule, preparation permission, index or RegistryRoot rule.

No code, SQL/database/schema/migration, native/model probe, source registration,
new bearer/allowance or runtime activation is authorized. A separately approved
component plan may gather limited evidence; its toy files/transaction cannot
construct an admitted source, fresh initializer or production owner.

## Scoped backend replacement — 2026-11-06

[ADR 0112](0112-pair-native-enforcement.md) records owner acceptance of #1469's
public Windows pair VFS, early allocator router/pair arena and conservative
availability restrictions for further qualification. Only this record's stock
VFS/no-custom-VFS candidate selection is replaced. Its same-worker original
owner, PairSlot/PairCore, native identity/source authority, stopped/unknown
lifetime and process-loss rules remain accepted. No implementation, new execution
budget, qualified envelope or activation follows from that scoped replacement.
