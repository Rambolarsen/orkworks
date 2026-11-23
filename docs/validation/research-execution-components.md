---
type: Validation Evidence
title: Offline research execution-proposal components
description: Accepted execution-proposal source examples with exact summary binding, publication/approval/rejection records and label-assertion grants; supplied authority and qualification facts stay unqualified.
tags: [orkworks, research, execution, proposal, approval, qualification]
status: draft
workflow_status: execution-components-only
---

# Offline research execution-proposal components

Tracking: [#1689](https://github.com/Rambolarsen/orkworks/issues/1689),
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089), initiative #738.
Inspection base: the merged execution-proposal contract plus the acceptance
record (`b9f2524d8acf`).

The owner [accepted the five execution-proposal choices](../superpowers/specs/2026-11-19-research-execution-proposal-design.md#scoped-owner-acceptance-—-2026-11-19)
for offline components only on 2026-11-19. These are separate decoded examples
and supplied-fact oracles, not an event sequence, complete store, publisher or
authority implementation. Gate 2 remains **Investigate**.

## Artifacts and source boundary

- [Independent Python generator](https://github.com/Rambolarsen/orkworks/blob/main/docs/validation/generate-research-execution.py)
  reads the unchanged prefix and completion packets; it never calls JavaScript.
- [JSON oracle](research-execution-components.json) retains the
  `single-task-execution-source` plan with the exact `researchBinding`
  (current summary/completed plan/input), the offline publication input, the
  publication effect rows (plan/task controls `proposed` v1, `planReferences`
  append, `pendingPlan` binding, `awaiting_execution_approval` state), the
  exact approval request/source/receipt with closed family members and the
  label-assertion grant boundary, the `reject-execution-proposal` lifecycle
  decision with its captured read set and cancelled-plan effects, the denial
  cases and effect labels.
- [JavaScript checker](research-execution-components.mjs) independently
  rebuilds finite source bodies, validates exact keys/values/domains/joins and
  digest exclusions, and supplies the publication/approval helper oracles.
- [Tests](research-execution-components.test.mjs) regenerate exact UTF-8/LF
  bytes and exercise fourteen semantic-mutation denials.

## What the packet establishes

- The no-new-input-revision boundary: the proposal binds the exact current
  summary/completed-plan/input refs; no decision-union sibling and no fresh
  assessment manifest are published by this transition.
- Publication effects: plan/task controls at version 1 with status
  `proposed`; the execution `PlanRef` appended once under the 16/64 ceiling;
  `pendingPlan` bound; `currentPlan` stays cleared (the accepted completion
  effect is unchanged); one `runVersion` advance; `awaiting_execution_approval`
  selected only when eligible, with gated/blocked retained-state alternatives.
- Approval: exact shown bytes/request digest/approval digest domains; closed
  family members; receipt ≤4 KiB; `planVersion` advance; one fresh
  server-held `PlanExecutionGrant` represented as label assertions only —
  never serialized, never restored, never carried from the research grant;
  no launch is closed and no execution profile is qualified.
- Rejection: the `reject-execution-proposal` lifecycle kind with the captured
  proposed-plan read; the rejected plan `cancelled`/planVersion+1 as a
  non-executable rejected proposal; `pendingPlan` cleared; an interrupted
  parent stays paused with its reason and retained
  `resumeTarget=awaiting_execution_approval`; the rejection receipt's
  `planVersion` stays null; replacement is a new publication bound to the
  then-current context.
- Denial cases: multi-task plans, reused worktree groups, bearer-as-approver,
  research-grant carryover, unqualified-profile approval, stale completed
  plans and wrapper overflow.

## Supplied and unqualified facts

Current Main/parent authority, epoch, owner, lease, health, rate and shown
context; grants as volatile label assertions; capability/profile
qualification; launch-chain admission; complete source/consumer/index
membership; original H/C/proofs; resource ledgers and actual effects.
Matching digests and repeated source bytes establish none of those facts.
Source fingerprint hashes cover documentation bytes; CommitRef changeDigest
hashes explicitly unqualified labels. No wire decoder, header, manifest,
row/change hash, runtime reducer, native/model probe, database, migration or
executable product plan is supplied. The terminal-completion packet is a
later separate bounded task under its own acceptance.

## Verification and measurements

Node 22 `node --test` runs all prior documentation packets unchanged (the
packet's own 19 regressions pass, including independent regeneration
equality, fourteen semantic-mutation denials, and helper oracles separating
publication guards from approval authority). The generator-derived
measurements record the exact key plus committed occurrence projection for
the retained rows; they are not storage charges or a legal transaction
witness.
