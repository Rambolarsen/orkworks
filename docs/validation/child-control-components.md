---
type: Validation Evidence
title: Selected child control component bytes
description: Independently generated synthetic control/input/runtime/coordination/recovery components and conditional logical oracles; no complete store, producer authority or native qualification.
tags: [orkworks, child-sources, control, recovery, canonical-bytes, qualification]
status: stable
workflow_status: component-bytes-only
---

# Selected child control component bytes

Tracking: [#1183](https://github.com/Rambolarsen/orkworks/issues/1183),
[#1176](https://github.com/Rambolarsen/orkworks/issues/1176).
Contract: [accepted selected control codecs](../superpowers/specs/2026-11-02-child-launch-control-design.md#scoped-owner-acceptance-and-component-handoff--2026-11-02).
Base: `5ba723e3475a63b644e764c19b10ff39d2d02405` (#1335).
Every native object, assessment, lifecycle outcome, receipt and commit reference
is synthetic. These components are alternatives, not rows of one coherent current
store. Different run/task/status variants must not be combined as a transaction.

## Delivered components

The independent standard-library [Python generator](https://github.com/Rambolarsen/orkworks/blob/main/docs/validation/generate-child-control-vectors.py)
produces [45 pinned vectors](child-control-vectors.json). The separate
[JavaScript checker](child-control-components.mjs) reconstructs all nine accepted
table keys, exact value field sets, canonical bytes, raw hashes, source domains
and format-4 row projections. The [tests](child-control-components.test.mjs)
regenerate the entire packet and compare its bytes.

- Prospective worker and reviewer direct inputs consume exactly the selected
  configurations, review input, target result and report sources, plus their
  original prior-plan reference. Their preparation/input revisions are two.
  They are independently typed prospective replacement components; the older
  [source packet](child-source-components.md) retains its original revision-one
  plan binding. No replacement plan or actual producer is fabricated.
- Parent/child runtime bodies, every run/plan/task/launch/session top-level state,
  all three coordination outcomes and all four recovery decision kinds are pinned.
  Keys and repeated identities resolve the supplied original source packet.
  Admission retains assignmentTaskVersion=2 even when live controls advance.
- Recovery retains full typed run/task/launch/session snapshots with original
  CommitRefs. Internal historical links stay committed; only the new row's outer
  lastCommit and receipt.commit are excluded. The synthetic successor sequence
  advances the captured header once. No direct origin/manifest proof is supplied.
- Conditional list oracles count an interrupted unattached reservation once,
  prepared/live children once, and terminal/proven-settled dispositions zero.
  They bind both the retained run ceiling and lower exact-plan ceiling. A resumed
  ordinary child may use a new retained runtime while the original spawn remains
  frozen. Reported coordination completion cannot itself release capacity.
- Resume classification preserves the explicit stage gate before blockers or
  unsettled attempts, then pending/current-plan classification. All accepted pause
  reasons/targets and interruption stage/reason tags are exercised. No resume,
  reapproval or launch operation is performed.

## Checks and denial cases

Run from the repository root with Node 22:

```bash
rtk proxy python3 docs/validation/generate-child-control-vectors.py --output /tmp/child-control-vectors.json
rtk proxy cmp docs/validation/child-control-vectors.json /tmp/child-control-vectors.json
rtk proxy node --test docs/validation/child-control-components.test.mjs
rtk proxy pnpm --dir docs docs:build
```

Tests reject altered original source/key/runtime identities even after all outer
hashes are recomputed; missing consumed inputs; unsupported clarification; stale
root/plan bindings; recaptured assignment versions; missing allocation/attachment/
runtime facts; foreign native descriptors; malformed u64 TEXT or overflowing JSON
versions; changed original coordination bytes; wrong recovery snapshots, blocker
bodies, origin epochs/sequences and receipt joins. They exercise inclusive UTF-8
summary boundaries and keep complete key/value wrapper ceilings distinct from
source-body ceilings.

The inventory helper counts every full canonical key and committed value
projection occurrence, with no content-alias discount. Duplicate current keys,
33rd recovery decisions and 65th per-session runtime generations are denied.
It measures the supplied selected run only: workspaceContributionBytes is that
run's contribution, not a complete workspace recount. Exact mutable before/after
deltas, other runs, parent_attachments preparation charges, metadata/global/command/
physical caps and supported maximum packing still need complete owner states.

The initial test run failed 115 of 116 checks against the empty checker; after
implementation all passed. A second red/green cycle caught a missing retained-run
digest check and recovery successor-sequence check, plus the new inventory/retry
oracles. No production module, model call or native harness was used.

## Qualification limits

The checker returns only unverified-control-components-only. Its capacity,
inventory, resume and retry helpers are conditional component oracles. Supplied
lists do not prove complete membership or absence; synthetic terminal/settled tags
do not prove native disposition. An exact-retained-retry-candidate is only a byte
comparison: current authentication must precede lookup in the actual owner.

No complete current store, read-set freshness proof, consumer index, control quota
initialization, old/new manifest, exact transactional CAS, historical origin proof,
source-owned lifecycle bridge, native allocation/spawn, atomic cross-store outcome
or eligible adapter is generated. The parent wrapper extension is a codec choice;
this packet does not publish it alongside actual runtime history/current controls.
Prepared replacement-plan joins remain unavailable rather than blessing the older
plan with new input bytes. Checksums cannot fill those gaps.

The [qualification handoff](../superpowers/plans/2026-10-31-child-review-source-qualification.md)
keeps complete owner/consumer/full-state qualification open. #1183/#1176 stay open;
#740 stays closed. No database, migration, runtime activation, native/model probe,
new grant, artifact read-work permit or broader clarification/research variant is
included.

## Verification and bounded review

Review routing: root is the sole writer; one read-only Luna reviewer checks the
new checker/generator against the accepted codecs, and a second checks test
coverage and claims against the actual packet. One round, at most three active
agents including root. Unchanged source contracts are reused. No model/native
probe, writer delegation or session metadata reporting is authorized for reviewers.

The focused 156 checks pass. The full repository verification command was invoked
but stopped at prerequisites because this docs-only worktree has no desktop
packages installed; it did not run the runtime suites. Documentation build/CI and
review dispositions are recorded with the PR rather than implying full runtime
verification from a synthetic packet.
