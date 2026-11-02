---
type: Implementation Plan
title: Qualify selective Rust CodeQL scanning
description: Bounded implementation, comparison and reversible activation of the proposed CodeQL routing design.
tags: [ci, codeql, security, implementation]
status: draft
workflow_status: activated
---

# Qualify selective Rust CodeQL scanning

**Outcome:** avoid Rust scans for proven unrelated UI/prose changes while retaining
full Rust security analysis and the other four language jobs. The
[design](../specs/2026-11-02-codeql-scan-routing-design.md) is the contract;
[issue #1312](https://github.com/Rambolarsen/orkworks/issues/1312) tracks delivery.

## Gates and ownership

Owner approval of the new security design precedes dependent implementation.
The owning session implements serially; no parallel writers or new harnesses.
The owner approved the concrete design on 2026-11-02 ("yes, go ahead"); this
authorizes implementation and the qualified cutover/rollback described there.
Implement through an owned worktree and PR, with the normal review and merge gates.
Do not activate live settings until staged qualification passes.

## Deliverables and checks

### 1. Conservative routing and a non-publishing workflow

Files: add `.github/workflows/codeql.yml`, `scripts/codeql-routing.mjs` and
`docs/.vitepress/codeql-routing.test.mjs`; update canonical CI guidance only when
implemented. Keep the selector narrowly responsible for choosing whether Rust
joins the fixed four-language matrix. Consume complete changed paths plus a
forced-full indicator; emit the matrix and a bounded routing reason.

- [x] Capture current default-setup settings, per-language analysis categories,
  effective versions and exact-SHA source identities. Locate the existing Rust
  extraction error and retain the diagnostic as a baseline limitation.
- [x] Write failing routing cases for renderer/prose-only skips; Rust/Cargo,
  new crates, bundled knowledge, external includes, shared fixtures and workflow
  edits; additions/deletions/renames; unknown/empty/truncated inventories; and
  initial pushes, schedules, manual runs and immutable main-push comparisons.
- [x] Implement the conservative allowlist with Rust inputs taking precedence.
  Unknown inputs run Rust. Validate against actual external Rust includes and
  pin cumulative PR-diff behavior. Use existing change-detection tools where
  sufficient; no custom GitHub client or persistent cache service.
- [x] Implement the five-language workflow with explicit no-upload staging,
  least permissions, same-repository PRs and PR-only cancellation. Keep all
  source/query scope, categories and other-language behavior from the design.
- [x] Run routing tests, workflow linting, docs checks/build, repository checks
  appropriate to the diff, and a current-diff review. Verify no publishing or
  default-setup setting has changed.

### 2. Qualify, then activate or restore

- [x] While managed scanning remains active, run the staged workflow and compare
  full five-language and repeated-Rust results using matching source inputs,
  CLI/query-pack versions and effective configuration. Preserve diagnostics,
  SARIF, coverage metrics and timings; do not compare only green conclusions
  or aggregate finding counts. Explain every new/missing diagnostic or finding.
- [x] Demonstrate real unrelated UI/prose skips, Rust/external-input scans,
  cumulative PR changes, cancellation and a forced-full five-language dispatch.
  Confirm category names and upload permissions before cutover.
- [x] With qualification passing and owner design approval recorded, perform the
  exact reversible cutover in the design. Confirm all five main categories upload
  successfully. On failure restore managed setup and verify its new full scan;
  leave the issue open and record the failure. Do not change required checks.
- [x] Record actual runtime/skip evidence and remaining limitations in the
  qualification report and update canonical guidance.

Final evidence-PR merge and owned-worktree cleanup follow repository policy;
issue #1312 records that close-out after the checks pass.

## Risk review

| Dimension | Rating | Evidence and treatment |
| --- | --- | --- |
| Dependencies | 3 | GitHub-managed setup plus owned Actions workflow; staged no-upload run qualifies compatibility. |
| Blast radius | 3 | All security publishers switch, but languages/query/source scope remain fixed. |
| State changes | 3 | Coordinated repository settings and SARIF category continuity; retain previous analyses. |
| Reversibility | 3 | Known settings restore path; live restoration remains an activation check. |
| Uncertainty | 3 | Timing is measured; staging compatibility, baseline extraction error and comparable findings require qualification. |

Total: 15/25; judgment aid, not approval. Primary blind spots are dropped external
inputs, incomplete PR inventories, main-push cancellation, category churn and a
publishing gap. The checks above cover each. No implementation is implied by a
review-ready plan.

## Plan review

Disposition: Approved for implementation. One fresh read-only builder check
found no important unanswered planning questions. The owning review confirmed
that each security boundary has a delivery check. Design approval does not establish qualification or prove that a live cutover is safe. Review this plan
once; revisit only material changes to scope, risks or acceptance criteria.

## Execution evidence

- Owner design approval recorded in conversation: 2026-11-02, "yes, go ahead".
- Preflight: staged routing outputs feed the fixed-language analysis matrix;
  activation consumes successful staged evidence. No conflicting interfaces found.
- Routing tests failed against empty exports before implementation; five routing,
  real-Git cumulative diff/rename, embedded-input and publication checks now pass.
  Forty documentation tooling tests pass; workflow linting passes.
- Current default configuration and category analyses captured under the owning
  session's `/tmp/orkworks-codeql-qualification/`; the publication variable is absent.
- Full local repository verification stops at the existing Node 26/required Node 22
  prerequisite; hosted checks use their configured versions.
- Rollout ruling: the non-publishing workflow may merge while default setup remains
  active, after its staged PR run passes. This makes full main and manual repeated
  qualification possible. A subsequent documentation/evidence PR can prove a real
  unrelated-change skip before activation. This is staged delivery of this same
  approved task; publisher cutover still requires all qualification checks.
- Fresh implementation review found canonical database aliases (`cpp`,
  `javascript`) differ from matrix language names. Fixed evidence capture to
  consume CodeQL's `db-locations` output. The actual workflow shell step failed
  against alias fixtures before the correction and passes afterward.
- Hosted PR staging and full main/manual comparisons passed for all five
  languages at matching source/tool/query versions. Prose-only routing omitted
  Rust, and a newer PR commit cancelled the bounded rerun of the older run.
- Activated with managed setup disabled and owned upload enabled. Full publishing
  run 41479654105 passed; five categories uploaded; all 27 open alert identities
  were retained. The qualification report records remaining extraction and live
  rollback-drill limits. Final evidence PR closes the issue after its checks pass.
