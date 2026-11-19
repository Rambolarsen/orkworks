---
type: Implementation Plan
title: Unverified improvement advice implementation
description: Deliver producer 3 with code-owned status, compatible projections and fixed warned handoffs while activation remains gated.
tags: [orkworks, taskmaster, implementation, compatibility]
status: draft
workflow_status: in-progress
---

# Unverified improvement advice implementation

Implement the [owner-approved design](../specs/2026-11-17-unverified-improvement-advice-design.md)
and [ADR 0116](../../adr/0116-unverified-improvement-advice.md) for
[#1694](https://github.com/Rambolarsen/orkworks/issues/1694). Pass means producer-3
advice survives decode/store/history/publication as explicitly unverified data,
and no older consumer can see its prose or mutate its records. Production
qualification stays false; no paid calls or historical output recrediting.

## Ownership and order

One parent owns all implementation in `unverified-improvement-advice`, based on
`fcf901205c0f1a73b6906f3b47a9f198b3965b1c`. Execute inline with TDD, recording
red/green commands and material rulings in the plan workspace. At most four
read-only agents, one round: independent backend/desktop surface inventories
before implementation, then current-code correctness and compatibility reviews
of the merged implementation. No concurrent writers or other coding harness.

## Delivery steps

- [x] **Identity and inert data.** Add producer-selected text policy 3 without
  changing policy 1/2. Update new capture, engine, manifest, history and
  recommendation publication identities exactly as specified. Bind status only
  after validation; preserve old readers and stored bytes. Tests cover the saved
  CI validation sentence, false completion claims only under policy 3,
  unknown/duplicate model fields, bounds/controls/citations and save/reload joins.
- [x] **Backend views and actions.** Introduce a closed protocol declaration
  parser and common public projection boundary. Wire all producer-bearing HTTP
  responses and record-bound mutation admission. Absent header stays legacy;
  unsupported values reject. Protocol 3 responses carry matching status only for
  validated producer-3 data. Legacy lists/history omit it, latest freshness is
  unknown if omitted data determines it, and detail/mutations refuse before any
  effect. Keep unfiltered records in active/dedupe/lease/currentness accounting.
  Exclude v3 prose from legacy rollup/audit/packet synthesis. Exercise real HTTP
  handlers, storage and side-effect refusal with mixed-version fixtures.
- [x] **Desktop presentation and handoff.** Add distinct narrow protocol-3 IPC
  methods in main/preload and independently owned renderer declarations. Keep
  old channels legacy and renderer headers/URLs unconfigurable. Use the new
  methods for improvement/readiness/recommendations and actions; fail closed
  against an old main/server. Validate protocol/status before exposing new
  prose. Render the exact accessible warning before report, history, card and
  preview text. Fixed ID-only server/preview handoffs carry the warning and
  protocol declaration, with no generated prose interpolation. Tests cover
  transport, parsing, rendered warnings, old/new combinations and refusal paths.
- [ ] **Integrate and verify.** Reconcile canonical specs/concept documentation
  and indexes. Run focused red/green tests, then `bash scripts/verify-repo.sh`,
  doc-check/worktree-check and current-code review. Inventory every surface
  below against implementation/tests. Deliver one PR and babysit through its
  terminal state; qualification still needs separate finite authorization.

## Surface inventory and checks

| Boundary | Owned paths | Required evidence |
| --- | --- | --- |
| Improvement status/start/cancel | `http/improvement_handlers.rs`, `improvement/{runtime,store,access,publication,history}.rs` | Version binding, legacy refusal, no inference on reads, same budgets/recovery |
| Readiness/history and combined/standalone admission | `http/readiness_handlers.rs`, `taskmaster/{readiness,readiness_store}.rs` | Omission preserves bytes; omitted latest gives unknown freshness; standalone remains legacy |
| Recommendation list/detail/active-analysis/run status | `http/{taskmaster,taskmaster_settings}_handlers.rs`, recommendation query projections | No unlabelled nested or detail prose; same active IDs and currentness |
| Dismiss/accept/complete/packet | taskmaster handlers and `session_application` action admission | Old view cannot mutate v3; auth and packet/override refusal precede effects |
| Rollup/audit and follow-up synthesis | `taskmaster/{evaluator,rollup,audit,mod}.rs` | No new prose enters a legacy producer; IDs still gate other analyses |
| Electron bridge | `electron/{main,preload}.ts`, transport helpers, `src/orkworksWindow.d.ts` | Fixed header only on new methods, generation/auth retained, legacy methods unchanged |
| Renderer/card/history/actions/preview | `src/{api,improvement,taskmaster,recommendationSource}.ts`, Readiness/Recommendations/Evidence components and Actions overview | Require supported protocol/status; warning before any v3 prose, including collapsed cards and previews |

## Review focus and recovery

Test new renderer/old server, old renderer/new main/server, malformed/duplicate
headers, contradictory stored identities and a new latest snapshot hidden from
an old view. A supplied status field is never authority; a completed
recommendation never verifies checks. Privacy redaction and downgrade preserve
safe bytes without relabeling new records as legacy. Rollback disables new
admission and keeps compatible readers; do not delete or rewrite history.

Plan review: **Ready** within the existing owner approval. Dependencies 3
(coordinated sidecar/main/renderer interfaces), blast radius 3 (shared views),
state changes 3 (additive versioned records), reversibility 3 (stop admission,
retain readers), uncertainty 2 (bounded inspected seams): 14/25. Compatibility
and recovery tests address these risks; approval does not waive qualification.

## Delivery evidence

The complete repository verifier passed: 3,571 Rust unit tests (21 ignored),
the Rust integrations, 1,772 desktop tests, 2,056 documentation checks, both
application/documentation builds and close-out checks. Ignored live qualification
was not run. Native Electron fixtures cover versions 2/3 and both intents,
including warning order, active-advice banners and ID-only receipts.

Fresh `/code-review medium` approved the current product diff, code-diff SHA256
`95ff16b6ef4947b9b7731b8a234a9195149e2a64aeb009d3ee5565659a0fc4c0`.
Both reported findings were reproduced and fixed: negotiate protocol support
before mutations, and warn before active-recommendation banner titles. A separate
static compatibility/privacy pass found no additional missed surface. No deferred
minor findings remain. The PR lifecycle is tracked in #1694; qualification and
production activation remain separately gated.

Implementation rulings retain a snapshot's version-3 origin after privacy
redaction, return lifecycle-only mutation receipts, and carry fixed HTTP status
through IPC for stale-detail recovery. These avoid origin loss, stale prose and
lost Electron Error properties. Review used an immutable staged tree whose code
hash is checked against the eventual commit; one parent owned all writes, with
four bounded read-only agents. Canonical source docs changed; installed/onboarding
claims need no change while production remains disabled. Foreign stale worktrees
reported by the close-out check remain untouched.
