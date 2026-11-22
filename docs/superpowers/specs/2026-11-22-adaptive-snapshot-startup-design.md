---
type: Design
title: Publish adaptive workspace status before Git enrichment
description: Give new adaptive subscribers a coherent session list while repository inspection is pending.
tags: [orkworks, sessions, performance, workspace-updates]
status: stable
---

# Publish adaptive workspace status before Git enrichment

The owner approved this behavior and contract amendment. [#1965](https://github.com/Rambolarsen/orkworks/issues/1965)

## Evidence and outcome

In the populated 793-session workspace, the legacy stream delivered its first
snapshot in 4.976 seconds. The adaptive stream took 14.710 seconds in a sequential
probe and delivered four heartbeats without a snapshot in a separate 50-second
probe. Session activity varied, so these timings identify a startup failure
mode and do not quantify a rollout speedup. [Live qualification #1925](https://github.com/Rambolarsen/orkworks/issues/1925)

Each adaptive subscription currently advances the shared enrichment revision.
The socket waits for a full inspection at or beyond that revision. An
inspection that loses its publication fence is discarded, so a reconnect can
invalidate work for established readers and wait through multiple inspections.
[Desktop recovery #1954](https://github.com/Rambolarsen/orkworks/issues/1954)
bounds the wait for each connection attempt. It cannot produce the missing data.

A new adaptive connection should establish current session identity, membership,
order, and status while Git or document inspection is blocked. Git and document
actions continue to resolve against current server-owned state.

## Proposed contract

- An adaptive snapshot is authoritative for the complete session status group
  under the active workspace owner. Each row also carries its enrichment
  revision and freshness state. A pending row uses the existing empty `cwd`
  placeholder, omits optional enrichment fields, and carries
  `{state: "pending", checkedAt: null}`.
- An initial subscription or reconnect requests a fresh status capture.
  Coalesce requests through a publisher-local status request revision, so a
  subscription leaves legacy workspace invalidation revisions alone. A subscriber
  can adopt a publication only after a status capture whose admission revision
  meets its request.
  Compare the request as a lower bound on the admitted capture. Later terminal
  output does not invalidate the earlier request. Use the existing
  workspace-owner, complete-read, and status publication checks on that
  capture. A superseded or incomplete capture stays in recovery. This bounds
  the Git dependency. Continuous changes that invalidate the status capture
  itself still rely on the desktop deadline and REST fallback.
- Publish the cold status snapshot before launching its initial Git inspection.
  A warm publisher supplies the same captured display to new and established
  subscribers. Retain enrichment only when the row and location revision
  still match. Carry forward its original check time and expose its current
  freshness. The existing renderer accepts the pending representation and
  ages checked values by that timestamp. Qualify a pending reconnect snapshot
  with the previous desktop build before enabling it for the existing
  capability.
- Subscribing requests status work. The shared publisher owns one enrichment
  worker and schedules it from its existing cold-start, mutation, and cadence
  rules. An explicit refresh remains a full invalidation. A rejected or late
  enrichment result cannot replace status or attach to a changed row.
- Send the first snapshot and subsequent deltas from the same ordered display
  baseline. Carry the existing owner, generation, and cursor through recovery.
  Workspace replacement, an incomplete status read, or an unsupported
  connection uses the existing resynchronization and REST fallback paths.
  Retain the desktop's 30-second first-snapshot deadline.
- Legacy clients and full REST requests receive complete rows through their
  current projection. Selected document reads, plan review, desktop app
  destinations, and workspace mutations use their existing fresh validation.
  Scope is format-1 workspace display transport. SQLite activation and history
  pagination have their own contracts.

### Alternatives considered

Keeping the full-snapshot barrier and adding more retry time still couples
startup to the user's repository workload. Reusing a cached snapshot immediately
would avoid that work, but could republish obsolete session membership after a
reconnect. A fresh status capture gives reconnect a current baseline and uses
the existing pending state to show which repository details still need checking.

## Implementation plan

1. After approval, amend the
   [adaptive stream design](2026-11-18-session-status-enrichment-design.md#wire-compatibility-and-recovery)
   and [ADR 0092](../../adr/0092-workspace-update-stream.md) to describe
   status-authoritative startup with independent enrichment freshness. Keep
   ADR 0092's version-1 cursor and recovery decision. Synchronize the runtime
   guide with the implemented behavior.
2. In `crates/orkworksd/src/workspace_updates/split.rs`, test a cold and warm
   subscriber with Git inspection blocked. Then gate initial delivery on the
   status capture admission revision. Prove multiple subscribers share the
   status work and that connecting to a warm publisher does not trigger another
   inspection or clear enrichment for existing subscribers. Keep status
   freshness independent of enrichment acceptance.
3. Test successor messages after the pending snapshot, including row removal,
   changed binding, incomplete status, workspace replacement, late worker
   results, and an unavailable Git directory. Verify that older desktop
   reducers accept pending rows and apply the matching enrichment delta. Check
   desktop status display and action admission while enrichment is pending.
   If an older desktop cannot safely handle this shape, return for a capability
   decision before enabling it.
4. Run the repository verifier and current-diff review. Compare first-snapshot
   latency, inspection starts, and CPU on a fixed fixture approximating 800
   rows and 19 roots with idle and sustained output. Hold the source build
   profile, row/worktree layout, output and subscriber counts fixed. Include
   a blocked inspection test that delivers the status snapshot before
   releasing the inspection. Report real-app qualification separately under
   [#1925](https://github.com/Rambolarsen/orkworks/issues/1925) after an
   owner-coordinated rollout.

A source rollback restores the full inspection gate for new subscriptions.
Legacy reconnect and REST polling remain the operational recovery paths.
There is no stored-state transition.

## Plan review

Ready for implementation under the approved contract. The previous desktop reducer validates the existing
pending representation, the sidecar already emits it on row changes, and the
status capture already runs without Git inspection. A direct reducer probe
accepted a pending first snapshot, rejected an older revision, applied the
matching enrichment, and aged it to stale. The remaining compatibility
risk is initial rendering and action affordances on older desktops. Include a
mixed-version test before treating that assumption as qualified. Exercise
reconnect during both a blocked inspection and a workspace replacement to
distinguish a fresh status capture from reuse of a cached display.

Dependencies 3 (sidecar publisher and desktop consumer), blast radius 3
(workspace display startup), state changes 2 (display admission and no
persistence), reversibility 2 (source rollback with existing polling),
uncertainty 3 (real-workload behavior and previous-client startup). Total 13/25.
