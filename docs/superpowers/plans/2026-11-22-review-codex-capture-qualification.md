---
type: Qualification Plan
title: Qualify the first Codex capture inventory and custody
description: Accepted staged evidence roadmap for a retained-resource observer and shared configuration capture owner, without producer activation.
tags: [orkworks, review, codex, qualification]
status: stable
---

# Qualify the first Codex capture inventory and custody

Outcome: qualify the accepted complete-inventory/custody architecture and define the
proof obligations before any actual producer can use it. Track
[#1966](https://github.com/Rambolarsen/orkworks/issues/1966).
Read the [design](../specs/2026-11-22-review-codex-capture-design.md) and its accepted
parent contracts. This is a staged qualification roadmap, **not an executable
approval of all stages**. Owner approval of the concrete design precedes any new
observer, protocol, epoch enrollment or runtime code. Each implementation slice
needs its bounded executable plan and issue when its prerequisites are proven.

## This documentation slice

- [x] Inspect the merged fragment/isolation components and actual store, asset-root,
  Settings claim and hook-observation boundaries. Select a complete literal
  five-item inventory and separate mandatory typed joins; record proposed choices.
- [x] Resolve misleading status/default/fallback semantics, physical-vs-logical
  absence, helper context, failed publication and observed ABA in the design.
- [x] Obtain one fresh builder check and one independent contract-correctness
  check of the merged design. Settle minor implementation calls locally; bring
  only consequential owner choices to approval. No review loops of unchanged text.
- [x] Run docs metadata/index/link tests and site build, diff/doc/worktree checks;
  prepare the design-only draft PR package with evidence and remaining gates. No cargo/desktop
  installation or tests are needed for this documentation-only change.
- [x] Obtain owner approval of the proposed architecture and availability choices.

## Qualification sequence after approval

1. **Bounded resource observation proof.** Own a focused private observer slice
   adjacent to the integration source, with Unix/macOS private worker transport
   and synthetic temporary roots. Before implementation, finalize the fixed
   descriptor/result contract and lifecycle under a bounded plan. First close
   [#1981](https://github.com/Rambolarsen/orkworks/issues/1981)'s received-descriptor
   inheritance boundary against actual concurrent sidecar child launch; a serial
   receiver fcntl is insufficient. Any material architecture change needs approval.
   Record the architecture decision through the ADR lifecycle before observer code. TDD must prove
   same opened-object descriptor transfer, no-follow traversal/absence,
   hard-link/root-alias refusal, exact bytes, limits, deadline/cancellation,
   saturation, worker termination/reap and permit retention after uncertain
   cleanup. Exercise real macOS arm64 file/descriptor/stall behavior; no installed
   user hooks/native Codex invocation. Keep producer imports disabled. Synthetic
   logical receipts alone cannot pass the physical gate.
2. **Controlled mutation coverage.** Audit actual callers of HarnessStore
   mutation/startup/recovery, workspace active selection, Codex OwnedAction,
   reporter-asset writers and hook observation writes/clears, plus owner/build/
   home/native-evidence replacement. Name each first effect and existing lane,
   prove one enrollment order and no inversion, then submit a bounded enrollment
   implementation plan. Inject failure before/after publication, writer-drain,
   repeated identical bytes, observed ABA, replacement and epoch exhaustion.
   Missing caller or uncertain recovery coverage denies availability. No broad
   refactor of manual flows is included merely to simplify enrollment.
3. **Typed join and capture publication.** Only after physical observation and
   owner coverage pass, plan a dormant producer joining exact stock resolution,
   five-resource fixtures, strict projection, source/installed/build bytes,
   active selection, home/root custody, typed readiness and prequalified
   isolation/native/context evidence. Inject races during slow reads and before
   final publication; require one changed/unavailable result, zero hidden repair,
   no currentness from caller inventory, zero preview probes and unchanged manual
   behavior. Status repeats fresh validation without extending expiry. Raw leaf,
   carrier, trust metadata and credential retention/export must stay absent.
4. **Separate activation decision.** Native/model/executable custody, exact
   SessionStart contextual policy, late environment/reporting/mailbox allocation,
   physical durable admission/retention and private prepared-creator proof still
   need their accepted independent evidence. Do not fill missing joins with
   booleans or synthetic fixtures. Submit activation/compatibility and ADR
   synchronization for approval only when all owners and evidence are concrete.
   Supported production roles remain empty until then.

Each code slice runs its focused real-boundary tests, full `verify-repo.sh`,
required current-code review and normal branch/PR lifecycle. Failed physical
qualification is a truthful Investigate result, never permission for fallback.
Rollback of this documentation slice removes links only. Future dormant code can
be reverted without user-file migration; activation/downgrade recovery is gated
separately and is not claimed by that rollback statement.

## Review and investigated risk

| Dimension | Rating | Evidence/treatment |
| --- | ---: | --- |
| Dependencies | 4 | Catalog/workspace/integration/trust/native owners; stage ordering prevents a synthetic shortcut. |
| Blast radius | 3 | Proposed shared owner spans writers; no writer changes in this slice. |
| State changes | 1 | Documentation only; later enrollment explicitly gated. |
| Reversibility | 1 | Remove proposal links; no installed or persisted state changes. |
| Uncertainty | 4 | Actual descriptor transfer/stall cleanup, enrollment coverage and context/native joins remain unqualified. |

Total 13/25 for the proposal slice. Self-review checked that every design
requirement has a stage and denial oracle, while no stage treats the existing
leaf/projector tests as physical custody or native evidence. Current disposition:
**Architecture owner-approved on 2026-11-22; fresh builder/contract review passed; physical observer Investigate under #1981; later stages remain gated.**

Review routing: one owner writes this design/plan. Two read-only fresh reviewers
(maximum two agents, one round) examine the same merged artifacts independently:
a builder asks consequential missing questions; a contract reviewer checks source
claims, accepted-parent compatibility and the qualification gates. Root reconciles
the reports. Use the current lower-cost review tier first; no parallel writers.

## Documentation verification — 2026-11-22

Metadata/index validation passed. `pnpm --dir docs docs:build` passed all 2,566
regression tests and rendered the site (60.86 seconds). Whitespace/doc/worktree
checks passed; README drift needs no edit because this proposal changes neither
shipped behavior nor milestone availability. Existing circular/chunk warnings
are outside the diff. Generated pnpm executable lock metadata was inspected and
restored. Foreign edits/worktrees remain untouched. No code/native/installed-file
verification is claimed; the production disposition stays Investigate.

## Approval and first qualification disposition

Owner approved the concrete architecture with “go ahead” after #1972. The
[native temporary-file descriptor experiment](../../validation/ordinary-review-codex-descriptor-qualification.md)
executed on macOS arm64 before observer code, exposing a receive-to-close-on-exec
inheritance interval. The ordinary Rust child inherited an unset descriptor;
setting it inside the Rust parent prevented inheritance in the serial control.
This is finite native evidence, not complete worker or race qualification.
Stage 1 remains Investigate until #1981 supplies a supported concrete custody
mechanism. No bounded observer implementation plan may assume that gate passed.
No product implementation, enrollment or prepared Start is claimed here.

Fresh approval/evidence documentation verification passed: all 2,566 docs tests,
metadata/index checks and site build (112.67 seconds), plus whitespace/doc/worktree
closeout. The native experiment and serial control ran as recorded; no full
observer/runtime/native-Codex qualification is inferred. README needs no edit
because shipped behavior and availability are unchanged. Generated lock metadata
was inspected/restored; foreign worktrees and shared-checkout edits remain intact.
