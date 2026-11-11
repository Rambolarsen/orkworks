---
type: Design Proposal
title: Fresh report-retention ledger initialization
description: Proposed exact fresh-store and fresh-run resource initialization bridging initial preparation to later report-history qualification without adopting old stores.
tags: [orkworks, hierarchy, reports, preparation, storage, accounting]
status: draft
workflow_status: proposed-for-scoped-owner-review
---

# Fresh report-retention ledger initialization

Tracking: [#1679](https://github.com/Rambolarsen/orkworks/issues/1679),
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089),
[#742](https://github.com/Rambolarsen/orkworks/issues/742),
[#744](https://github.com/Rambolarsen/orkworks/issues/744),
[#746](https://github.com/Rambolarsen/orkworks/issues/746); initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738).
Inspection base: `f938a9b3169e93cc07f76c5bcc4ed508a11cb8b2`.

**Proposed choices for scoped owner review.** This closes the design of one
initialization seam identified by the [Gate 2 investigation](../../validation/terminal-producer-history-consumer-contract.md#report-history-and-accounting-contract-to-close).
It does not claim producer qualification or Gate 2 completion. Merging this
proposal is not acceptance. No dependent fixture/checker, runtime/native execution,
migration, adapter, model probe or executable plan is supplied.

## Outcome and bounded choice

The [accepted initial effects](2026-10-28-root-preparation-effects-design.md#resource-keys-and-zero-initialization)
initialize preparation/reporting/summary resources but no report-retained-bytes
ledger. The [accepted terminal profile](2026-10-29-report-publication-profile-design.md#resource-and-command-budgets)
requires report ledgers already present and prohibits creating them during
correction. Choose explicit zero initialization at existing fresh-owner creation
boundaries: workspace at initialize-store; run at first subject-preparation.
Draft preparation reserves no report capacity and creates no run report ledger.

This is a proposed extension recipe for a newly initialized store and a freshly
admitted run, not a change to existing receipts or an automatic extension of old
stores. Reuse existing command tags, quota_usage codec and report resource tag.
Do not add a table, profile field, new source domain, report allowance, repair
command or standalone lazy initialization operation. Original initial-profile
witnesses remain valid only for their original profile and retain exact bytes.

Alternatives rejected: initializing on first report (allows the writer to invent
its missing accounting prerequisite); initializing at draft prepare (unneeded
ledger lifetime and discard effects before any report owner exists); backfilling
from current head/catalog (cannot prove historical acceptance or complete usage).

## Exact resource and provenance mapping

Use the [accepted owner/resource codec](2026-10-25-hierarchy-candidate-command-manifests-design.md#closed-resource-ledger-subset)
unchanged. The two exact resource keys are:

```text
{owner:{kind:"workspace",workspaceId},resource:"report-retained-bytes"}
{owner:{kind:"run",workspaceId,runId},resource:"report-retained-bytes"}
```

Initial body is exactly `{used:"0",reserved:"0",lastCommit:C}`, where C is the
creating command's CommitRef. Counters are canonical storage u64 strings, not
source JSON integers. No assignment-scoped report ledger is created. A report
stream's workspace/run derives from its exact approved source identity, never a
selected-run pointer or UI lifetime. All retained epochs in that workspace remain
charged. UI receipt-v2 lifetime and immutable report runtime lineage stay separate.

Reconcile used from canonical key plus committed-body projection of every owned
research-report objects row, compact report_heads row and report_receipts row;
omit only their already named outer lastCommit links. Charge every occurrence,
including receipt-v2 authorization. reserved stays zero. This is the accepted
report projection, not a new quota formula. Ledger/proof/header bytes are not
report-source occurrences; charge their full bytes to command and complete logical
store totals, avoiding circular self-accounting.

For zero creation, the complete relevant inventory must prove no report objects,
heads, receipts or orphan report ownership exists for that owner. Absence of a
head is insufficient. Reconcile all pre-existing workspace resources before fresh
run admission. Existing unrelated runs and epochs may hold nonzero workspace
report usage; the new run's zero row does not reset or rewrite that workspace row.

Original creation provenance is required, not just matching current values:

- Workspace: the original null-predecessor initialize-store proof commits this
  extension's complete ten-ledger initialization. A matching header, zero current
  total or later manifest containing one ledger cannot substitute for it.
- Run: the immutable original root_preparations receipt/source identifies the
  exact subject-preparation commit, whose complete effect includes the new zero
  run report ledger. Join workspace/run and original source/approval/reservation
  identities under the accepted initial producer.
- Subsequent changes: current ledger links resolve their own updating commands.
  Do not require a nonzero current ledger to retain its original creation link.
  Retain the original creation proof and validate the complete checked command
  history under the owning no-pruning profile; a sequence of hashes alone cannot
  establish legal producer effects.

No new persistent profile discriminator is selected. A future adapter must
explicitly recognize this closed initialization recipe and its original proof;
neither auto-detection of one extra row nor schemaVersion 1 alone enables it.
Stores with the original nine-ledger recipe remain governed by that original
profile and cannot perform report writes through this extension. Missing original
proof/profile recognition is unavailable, not implicit conversion. This does not
make the receipt-only authority fixture or synthetic terminal seed an eligible
initialization history.

## Complete effect deltas

Let K be the unchanged header replacement/new-proof pair; C is its checked next
CommitRef. Q is exactly final changed quota rows after summing owner deltas once.
Use unchanged format-3 key ordering, row commitments, manifest exclusions and
original-before/final-after rules. No proof pruning occurs. All new row links
equal C; unchanged rows keep original links. These are proposed logical effects,
not executed maximum-byte or source-valid fixtures.

| Existing command under this extension | Exact delta from accepted initial recipe | Complete changed-row count |
| --- | --- | ---: |
| New-store initialize-store | Add workspace report-retained-bytes zero row to the original nine zero workspace resources; no sources, drafts or runs | 12: ten resource rows plus K |
| root-draft-prepare | No new report row or report resource replacement; retain original absence/source/reservation guards | n + 6, unchanged |
| Exact root-draft-cancel | No report ledger creation/deletion; original complete draft/artifact/preparation effects only | n + 6, unchanged |
| subject-preparation, initialized summary | Insert one zero run report ledger in the same commit as original five core effects, six zero #744 resource rows, two preparation-byte replacements and three summary-resource replacements | 19 including K |
| subject-preparation, never-initialized summary | Same run report-ledger insertion with original unavailable-summary admission branch; omit only the three summary-resource replacements | 16 including K |
| Exact authorized preparation retry | Return original immutable source receipt after current authority/context and complete history checks; do not rerun fresh-owner absence tests as though admission were new | 0 |

n is the original selected capability-reference count, at most 16. Existing smaller
source/row/command limits win. Every zero row is a real inserted logical row and
manifest member. Include key/value and its proof-member overhead in byte totals;
do not count the same proof occurrence twice. The 12/19/16 counts do not establish
byte fit, physical storage, query work or native latency.

Initialization is only for a genuinely new empty store under the original
ownership/newness contract; a matching workspace header is not newness proof.
Fresh admission requires its burned reservation, exact pending draft/source/approval,
absent retained source/state/subject and absent initial reporting-resource
ownership as required by the initial recipe; preserve the existing draft-owned
run preparation ledger. In addition require the run report ledger and all report
prefixes absent. An existing zero ledger is a conflict/corruption to resolve, not
permission to overwrite it. An admitted retry is recognized from its original
source/receipt before entering these new-admission absence checks.

Drafts have no run report ledger, so exact authorized draft discard must also
verify no report ledger/history exists for that unadmitted run. If any does, deny
without deleting it or claiming that run was clean. Reservation identity remains
burned on discard or admission failure. Admission failure leaves the exact draft
and artifact ownership intact. Old authority never revives because a resource
write or discard was denied.

## Capacity, report publication and terminal history

Preserve the accepted initial [8/128 MiB preparation and retained-inventory budgets](2026-10-29-root-preparation-budgets-design.md#owner-acceptance-and-boundary-fixture-handoff--2026-10-29)
within their scope, including 32 runs, 64 MiB/4,096 proofs and 512 MiB/8,192 logical
bytes/rows. Zero report resources do not reserve more space or increase these caps.
Later producers still need their own reviewed whole-inventory/custody effects;
this initial extension does not authorize them. At every boundary count all new
ledger/proof/header bytes and rows, including never-initialized summary markers.

For later report qualification retain 256 MiB workspace report usage and the
existing 32 MiB combined per-run preparation/report aggregate, with each applicable
smaller component limit enforced independently. Terminal run-state occurrences
remain charged to preparation. No run-count or version credit releases on finish,
cancellation, retry or namespace rotation. Keep receipt/source/request caps and
the 2,149-byte v2 maximum-width rejection at 2 KiB unchanged. No #744 receipt credit
can fund reports.

An initial child report would require exact approved task/reservation/configuration,
fresh authenticated child generation and complete accepted report/request/receipt/
head/run/dependency/resource effects. Those active producer rows/effects remain
unclosed. This proposal does not name a legal first-report command by combining
empty ledgers with a report-shaped object. Once that separate producer is closed,
its first accepted descriptor/head/receipt must charge both existing report
ledgers in the same recoverable publication; never add a ledger afterward.

Terminal conversion preserves these existing report rows and usage. It must
reconcile the run-state preparation delta under its future complete finish/cancel
recipe. It cannot import synthetic history or reset report usage to zero. The
evaluation-only correction profile still needs complete source-owned consumer
absence proof; retained summary/proposal/plan inputs do not disappear at finish.
This design supplies neither that conversion nor permission to broaden consumers.

## Recovery, denial and future offline oracles

All visits, including complete inventory scans, repeated lookups, overflow
sentinels and final revalidation, consume the inherited 100,000 visited-row bound.
Stop on byte/row/counter overflow with no truncation or partial owner set. Keep
16 MiB / 4,096 changed-row command limits. Qualified physical index enumeration,
exclusive ownership and cooperative SQL deadline enforcement remain separate.

| Future logical case | Required oracle after scoped acceptance |
| --- | --- |
| New extended store | Exactly ten zero workspace resources plus header/proof; no run or report source |
| Fresh admission, either summary branch | Exactly one zero report run ledger joins original admission proof; workspace report row stays byte-identical |
| Draft prepare/discard | No report row effects; discard with orphan report ledger/history denies and preserves all bytes |
| Existing nine-ledger store / missing row | Report-capable operation unavailable; no automatic initialization, row insertion or upgrade |
| Existing nonzero workspace and fresh run | Complete old usage reconciles; fresh run gets zero without zeroing any other owner |
| Exact admission retry after later qualified usage | Original receipt, no new ledger/proof/counter change; nonzero current ledger is not mistaken for conflicting fresh admission |
| Self-consistent forged genesis/admission effect | Even with recomputed hashes, reject omitted/extra resource effects or unqualified producer history; manifest membership alone is insufficient |
| Counter/row/byte/proof/visit exhaustion | Preserve complete prestate/draft; no lazy ledger or partial successful admission |
| Interrupted initialization/admission | Qualified recovery yields complete old/new state or unavailable reconciliation; never recreate missing zero rows |

These are future offline oracles, not executed cases. Recovery must validate
recognized recipe, exact source/row/proof joins, all resource ownership and complete
history before current report writes/retries. It cannot derive original acceptance
from latest head, current totals, schema version, source catalog or volatile scope
metadata. Unknown/corrupt/legacy state stays preserved and unavailable for this
extension; no event replay, credential resurrection, counter reset, rollover,
pruning or migration is introduced.

## Scoped decision and handoff

Accept or revise these new choices together:

1. Workspace report zero ledger at explicit fresh initialize-store; run report
   zero ledger at first subject-preparation, with no report row at draft lifetime.
2. Exact additional effects/counts and original creation-proof recognition using
   existing codecs/tags, without a new header discriminator or old-store backfill.
3. Fail-closed legacy/orphan/recovery behavior and full unchanged accounting/caps.

After scoped acceptance, independent documentation witnesses may cover only these
creation/accounting deltas and denials. They must preserve old catalogs and cannot
claim a complete admitted report/terminal history. #1679 stays open until the
owner decision; #1089 Gate 2 remains Investigate. Actual finish/cancel and active
report producers, complete consumers/indexes, UI transport/lifecycle/native
ownership/durability, custody, applicable capability evidence and separately
approved executable plans remain independent gates.

This documentation can be reverted without store effects. Persistence downgrade
or migration is not defined or authorized; the explicit no-adoption cost is that
an older store cannot gain report capability through this proposal alone.

## Bounded design review

One fresh read-only builder check found no consequential unanswered question for
the narrow initialization extension. A separate read-only correctness pass checked
the accepted initial/terminal/resource contracts and found no concrete conflict
in counts, original-proof recognition, ownership, no-adoption, retry ordering or
shared limits. Both reviews explicitly excluded deferred report/terminal/native
producers. The author subsequently clarified that admission preserves the
draft-owned preparation ledger while requiring absent initial reporting resources;
no independent rereview of that wording is claimed. No fixture/runtime/native
qualification was performed by either reviewer.
