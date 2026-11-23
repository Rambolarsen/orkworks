---
type: Design Proposal
title: Research command and proof closure
description: Proposed command/proof closure over the accepted contracts — the complete effect union, captured H, coalesced versions, original proof rules and retry boundaries per command class, completing the audit's command/proof closure item.
tags: [orkworks, research, commands, proofs, closure, qualification]
status: draft
workflow_status: proposed-for-scoped-command-proof-review
---

# Research command and proof closure

Tracking: [#1689](https://github.com/Rambolarsen/orkworks/issues/1689),
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089), initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738); consumer
[#610](https://github.com/Rambolarsen/orkworks/issues/610).
Inspection base: `aa3f256fa4a5` (origin/main, including the initializer
acceptance merged in [#2020](https://github.com/Rambolarsen/orkworks/pull/2020)).

**Proposed for scoped owner review.** The
[source-coverage audit](../../validation/research-source-coverage-audit.md)
names command/proof closure as catalog-closure item 4. This proposal closes
that written boundary over the six accepted contracts: the complete per-command
effect union (every source/consumer/resource/index/header change joined to its
original proof), the captured-H prestate rule, version coalescing, and the
retry/recovery boundaries per command class. Every command's underlying
semantics remain owned by their merged contracts — this document joins them
into the complete command/proof boundary; it does not alter them. No storage
format, table/layout, migration, runtime/native/model execution, adapter or
executable plan is supplied. Gate 2 remains **Investigate**.

## Outcome and preserved decisions

Make the research command surface closed: every admitted command class names
its complete original-to-final effect union — all changed rows, all created
objects, all ledger deltas, the derived lookup maintenance, the header advance
and one new complete proof — with the captured prestate H and coalesced
versions, such that a command's proof is checkable against the full effect
union rather than a partial subset. Any effect the owning contract requires
that the union cannot express denies the command as a whole.

Preserve unchanged: every command's semantics, admission order, request/
receipt shapes and bounds from the merged contracts; the accepted
initialization boundaries (three distinct outcomes, sole-creator rule); the
discovery/invalidation ownership and bounds; the first-report/coordination/
lifecycle admission orders and exact-retry rules; the nonblocking
ordinary-input/stop/end independence.

Rejected alternatives: partial effect unions that admit a command while
leaving required effects symbolic; post-commit metadata repair; caller-supplied
affected sets; proof-only commands whose effects are asserted rather than
joined; and splitting one logical outcome across multiple commits.

## Source trace and obligations

| Source | Established constraint |
| --- | --- |
| [Audit item 4](../../validation/research-source-coverage-audit.md#catalog-indexes-initialization-and-proofs-still-required) | Join the full source/consumer/ordinary/resource/index/header effect union, captured H and one new C; coalesce sole live versions once; retain every original receipt, excluded commit link and immutable direct proof; exact retries use current authority before original lookup and return historical outcomes |
| [Effect unions (per contract)](2026-11-12-research-membership-publication-design.md#first-report-read-set-and-original-to-final-effects) | Each transition's before→final union with per-owner required changes and unchanged facts |
| [Completion machinery](2026-11-15-research-completion-design.md#source-owned-transition-and-effect-union) | Trigger, completion and coalesced effects in the same C |
| [Manifest boundaries](2026-11-23-research-initializer-manifest-design.md#manifest-completeness-and-proof-rules) | Three distinct outcomes; header advance + one new complete proof per outcome; peers preserved |
| [Shared commit rule](2026-11-19-research-execution-completion-design.md#source-owned-transition-and-effect-union) | Trigger, completion and coalesced effects commit together; neither hashes the new C |
| [Retry precedent](../../validation/first-report-producer-contract.md#exact-first-report-request-and-acceptance-order) | Authorized identical retry returns the original receipt before live CAS |
| [Bounds precedent](2026-11-12-research-membership-publication-design.md#complete-discovery-and-the-positive-first-report-slice) | 4,096 observations/16 MiB cumulative discovery, 100,000-visit ceiling, whole-command 4,096 changed-row/16 MiB limits |

## The command classes and their closed effect unions

Every research command belongs to exactly one of the following classes; each
class's effect union is the complete joined set from its owning contract.
Classes not enumerated are unavailable.

| Class | Owning contract | Effect union (original→final) | Proof rule |
| --- | --- | --- | --- |
| Workspace initialize-store | initializer contract | Resource ledgers + pools + profile registration; peers preserved | One header advance, one new C; rows' original lastCommit = C |
| Root admission | initializer + completion contracts | Run owner + selector + root subject rows + inherited six core source effects + charged ledgers | One header advance, one new C shared by all created rows |
| Attachment preparation | initializer + child-source contracts | Five child pools + summary_state + assignment source + authorization + attachment + provenance membership | One header advance, one new C |
| Proposal publication (research) | source codecs + control prefix | Source objects + input/plan/task controls + collection/expectation + pending pointer — no allocation, grant or reservation rows (those are separate later outcomes) | One C; trigger-adjacent effects coalesced once |
| Exact plan approval (research) | source codecs | Approval source/receipt + grant + active pointer versions | Receipt before live CAS on retry |
| Reservation (research) | source codecs | Reservation row + frozen version pair + task launching/version + expectation binding + run version, before native allocation | One C with the reservation |
| Allocation (research) | child source contract | Allocating intent persisted, then allocation or interrupted observations; attempt/capacity retained | Separate outcome per the accepted chain |
| Attachment + fresh preparation | child source contract | Attachment + assignment source + provenance membership + fresh subject/ledger effects | Separate outcome; only then can reporting capability delivery occur |
| Exact plan approval (research) | source codecs | Approval source/receipt + grant + active pointer versions | Receipt before live CAS on retry |
| First report publication | membership contract | Report object + head + receipt + input revision + collection/expectation versions + current-input pointer + run version | Full admission order; exact retry returns original receipt before CAS |
| Turn gate | continuation | Turn row + task needs_parent_result + task/run versions | One C with the turn row |
| Coordination outcome | continuation | Request/receipt row + task status/result + task/run versions | One C; plan-completion trigger may join the same C |
| Research plan completion (automatic) | completion contract | Completion record + research plan complete/version+1 + grant revocation + selector advance (completedResearchPlan selected) + run version | Trigger + completion + effects in same C |
| Execution plan completion (automatic terminal) | execution completion contract | Completion record + execution plan complete/version+1 + selector UNCHANGED (completedResearchPlan stays the research plan) + run terminal \`complete\` + full authority revocation + collections/expectations retired + waits closed + pointer clears | Trigger + completion + effects in same C; paused-prestate preserved through rejection-side transitions |
| Summary publication | completion contract | Summary row + selector advance + run version coalesced; consumer invalidation | Exact-key retry; changed bytes conflict |
| Lifecycle UI decisions | completion contract | Record + state transitions + effects per kind (finish/cancel-stage/continue/cancel-run) | Receipt before live CAS on authorized identical retry |
| Scope rejection | scope-rejection contract | Rejection decision + rejected plan cancelled + completion rerun when eligible | Both lifecycle decisions in one outcome |
| Review outcome | review tasks | Review row + no task version/outcome/grant change | Generation/lease/rate before lookup |
| Clarification assessment | clarification contract | Manifest + request row + pending binding + state | One outcome; request row same outcome |
| Clarification answer | clarification contract | Answer row + next input revision (copied decision, user-origin producer) + pending clear + runVersion | Copied decision byte-unchanged; fresh assessment is a separate outcome |
| Research proposal approval (execution) | execution proposal contract | Proposal object + plan/task controls proposed + planReferences append + pendingPlan + state | Resumed-eligible parent precondition; no launch closed |
| Execution proposal publication | execution proposal contract | Proposal object + plan/task controls proposed + planReferences append + pendingPlan + state | One outcome; publication input non-retained |
| Execution approval | execution proposal contract | Approval source/receipt + plan active + currentPlan + grant labels | Resumed eligible parent precondition |
| Execution rejection | execution proposal contract | Lifecycle decision + cancelled plan + pendingPlan clear + pause-retained state | Receipt planVersion null |
| Original-attempt recovery | continuation + recovery-completion contracts | Recovery decision row (four kinds, typed snapshots, native observations) + blocker/capacity effects + task/run versions, or observe-terminal settlement + automatic plan completion in one outcome | One C; exact retry returns the original receipt; no second attempt |

### Captured H and coalescing

- Every command captures its complete prestate H before mutation; the new
  manifest's predecessor is exactly H.
- Sole live versions (`runVersion`, `taskVersion`, `planVersion`,
  `collectionVersion`, `expectationVersion`, `summaryVersion`,
  `producerRevision`) coalesce once per compound outcome — a version advances
  once even when multiple owned effects touch it, and never wraps/resets.
- Immutable records never advance versions; their publication changes only
  the consumers' currentness.
- Each changed row's original→final delta is charged once per ledger;
  created rows are real effects even at zero values.

### Original proof rules

- Every retained row's `lastCommit` is the owning outcome's C; excluded
  links (`receipt.commit`) resolve their own original proof under the owning
  manifest.
- Historical direct proofs are retained; no predecessor replay, maxima
  election, source/table fallback or receipt reconstruction substitutes for
  actual currentness/coverage.
- A source hash or matching receipt never proves a complete command.
- Proven rollback keeps the complete old state; proven commit keeps the
  complete new state; unknown holds original ownership and fences dependent
  work.

### Retry and recovery boundaries

- Authorized identical retries return the original receipt/outcome before
  live CAS, with no mutation, version, event or charge — per command class.
- A changed body for the same identity conflicts.
- Post-terminal parent-level retries are denied by preserved authentication
  rules; source-level idempotence returns the original outcome.
- Unknown disposition holds original ownership/leases and fences dependent
  work; no timeout rollback assertion, replacement worker, or automatic
  relaunch.

## What stays gated

No storage format/table/layout/manifest encoding is selected — audit item 5
consumes this closure. The physical catalog/version selection, the execution
launch chain (its own gated slice), execution recovery revisions, profile
qualification and separately approved executable plans remain open. No
offline component packet is authorized — the closure enumeration is the
deliverable, validated through docs checks and hosted review.

## Owner choices and bounded next steps

Accept or revise these proposed choices for **written command/proof-closure
work only**:

1. The closed command-class table above — every admitted class names its
   complete effect union from its owning contract; classes not enumerated
   are unavailable.
2. Captured-H and coalescing rules — predecessor exactly H; sole live
   versions advance once per compound outcome; immutable records never
   advance versions.
3. Original proof rules — `lastCommit` = owning outcome's C; excluded links
   resolve their own proofs; no substitute for actual currentness.
4. Retry/recovery boundaries — identical-retry-before-CAS per class;
   changed-body conflicts; post-terminal parent retries denied; unknown
   disposition holds ownership.
5. Audit item 5 (physical catalog/version selection) remains separately
   gated; no component packet follows from this contract.

## Review and verification handoff

One writer: the owning session. One bounded round with two fresh read-only
reviewers: command-class completeness against the merged contracts and
proof/retry-boundary correctness. Reviewer cap two; no reviewer edits,
fixtures, runtime, native or model probes. Verify docs metadata/index,
build, rendered links, prior-artifact byte preservation and diff/doc/worktree
close-out. Publication/merge does not accept the five choices above.

## Remaining gates

Physical catalog/version selection (audit item 5), the execution launch
chain, execution recovery revisions, profile qualification and separately
approved executable plans remain open. #1689/#1089/#610 stay open; #740
stays closed research. Gate 2 remains **Investigate**.
