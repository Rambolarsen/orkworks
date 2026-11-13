---
type: Implementation Plan
title: Inert improvement text and fixed handoffs implementation
description: Coordinated implementation of approved text policy 2 with compatible history and canonical handoff admission.
tags: [orkworks, taskmaster, implementation, security]
status: draft
workflow_status: active
---

# Inert improvement text implementation

Outcome: useful bounded explanations survive decoding/restart while generated
text cannot alter the version-2 handoff. Implements the owner-approved
[contract](../specs/2026-11-13-improvement-text-contract.md) and
[ADR 0115](../../adr/0115-inert-improvement-text-and-fixed-handoffs.md), under
[#1694](https://github.com/Rambolarsen/orkworks/issues/1694) / PR #1753.

Root owns the coordinated implementation inline using `executing-plans`. No
parallel writers. One fresh whole-code security review follows implementation.
Use existing interfaces; add only a small internal text-policy module. No live
calls, activation, new dependencies or unrelated refactors.

## Delivery and checks

- [x] **Text and decoder.** Add policy-2 bounds/control/bidi/claim validation,
  with legacy adapters unchanged; route all final-result and readiness-row text
  through the chosen policy. RED/GREEN fixtures include harmless inventories,
  command-looking text as data, false claims, bounds and unknown citations.
- [x] **Producer, publication and history.** Bump the captured evaluation
  discriminator; bind manifest purpose, history wrapper/versions/evaluation key
  and recommendation version. Validate identities before text-policy selection.
  Preserve legacy reads; reject contradictions/unknown profiles. Exercise real
  persistence/restart, currentness, privacy reduction and publication joins.
- [x] **Handoff admission.** Generate fixed ID-only prompts in Rust for both
  v2 intents. Reject overrides and packet-reporting/packet-admission paths before
  mutation/delivery. Test hostile generated fields, bad IDs, both intents and
  unchanged v1 behavior through real application admission.
- [x] **Desktop.** Recognize v2 provenance; preview a read-only fixed prompt,
  submit only session identity, and bypass packet handling only by refusing it.
  Keep unknown references blocked. Test payloads and a real rendered hostile
  result/handoff with no automatic submission.
- [x] **Verification and documentation.** Replay retained public fixture finals
  offline; never rewrite trial records. Run affected suites, full
  `bash scripts/verify-repo.sh`, doc/worktree checks, current-code manual medium
  review and CI. Update canonical contracts/PR to final scope. Keep production
  qualification false and the known nine-request failure unchanged.

## Investigated risks and review

The history validator currently reuses strict legacy prose checks; the shared
policy must cover its adapter, not only opportunity strings. Canonical Rust
admission currently ignores overrides only for investigation; v2 must reject
all supplied overrides for either intent before execution state changes. The
renderer currently prefers completion packets; its v2 branch and the server
must refuse those routes. Report reload must validate text before exposure,
with current capture binding required before publication/admission. These are
covered by the owning delivery steps above.

Ratings: dependencies 2 (one owner, known Rust/desktop interfaces); blast radius
3 (new producer plus shared adapters, legacy behavior pinned); state changes 3
(additive versioned records, no rewrite); reversibility 3 (coordinated revert
makes unsupported new records unavailable safely); uncertainty 2 (source-traced
seams, behavioral checks required). Total 13/25; no unknown rollback path.

Plan review: **Ready** for authorized implementation. All contract sections map
to a delivery/check above. Distinct model-result/range-envelope schema versions
remain unchanged; producer/text/history versions select the new behavior.
No renewed design approval is required within this already approved scope.

## Evidence ledger

- 2026-11-13: owner answered “keep going” to the explicit PR #1753 contract
  approval request. Existing clean owned worktree reported before edits; base
  rebased onto verified `origin/main` with no conflicts. Implementation pending.

- Text, desktop and history/handoff regressions were observed RED before their
  implementation; focused checks passed afterward. Corrupt-citation and missing
  nullable-field cases additionally drove persistence validation. The real
  Electron scenario passed for both intents; disabling v2 recognition made it
  fail. Removing the packet-reporting admission guard made its real application
  test fail; both mutations were restored. Exact retained corpus-2 entry/CI and
  corpus-3 restricted finals decode offline under the new policy; temporary
  replay code was removed. Historical ledgers and usefulness verdicts are unchanged.
- Compatibility adjustment: public advice string limits count Unicode scalars
  as the Rust codec does; a 1,000-scalar non-BMP description is accepted and
  1,001 is rejected. Existing UTF-8 byte caps remain independently enforced.
- Active owner-authorized continuation began 2026-11-13 14:00 UTC, bounded to
  16:00 UTC for this adopted PR. Current-head manual review, full verification
  and CI remain required before merge. No live inference was run.

- The complete final-source `bash scripts/verify-repo.sh` passed: 3,555 Rust
  tests (21 ignored), integration/native checks, 31 reporter tests, 1,751
  desktop tests (five skipped), type checks/build, and 1,662 docs tests and site
  build. Full log: `/private/tmp/orkworks-inert-full-verify.log`.
- Fresh independent `/code-review medium` approved the coordinated current
  diff without material findings; all 25 code/test hashes match the reviewed
  source manifest `/private/tmp/orkworks-inert-review-source.json`. Medium is
  justified by the security/schema scope and coupled cross-component diff.
- Documentation-currency suggestions for README and public onboarding were
  reviewed against the actual diff. The gated producer adds no setup steps or
  current installed behavior; canonical source/domain/ADR descriptions are
  updated. Public activation/release claims remain unchanged and accurate.
- Required CI and current-head automated review remain pre-merge gates. No
  further source change occurred after the manual review or full verification.

Reviewed code-source map SHA256: `aa6e2181e365e99635850f60509da91c821bf88e8b8d0379879b50bc17935f44`.
