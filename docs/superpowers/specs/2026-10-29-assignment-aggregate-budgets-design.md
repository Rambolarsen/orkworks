---
type: Design Proposal
title: Assignment record and result payload aggregate budgets
description: Proposed finite assignment, owner-run and workspace limits for retained evaluation records and accepted-result payload occurrences, with explicit capacity-denial and retention behavior.
tags: [orkworks, hierarchy, evaluation, payloads, budgets, retention]
status: draft
---

# Assignment record and result payload aggregate budgets

Tracking: [#744](https://github.com/Rambolarsen/orkworks/issues/744),
[#746](https://github.com/Rambolarsen/orkworks/issues/746); initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738).
Inspection base: `1b973062` after PR #1156.
**Proposed numerical and admission-policy choices for owner review. No acceptance,
new fixture/checker, database, native probe, migration, executable plan or runtime
implementation is delivered here.**

## Outcome and short plan

The [accepted candidate ledger meanings](2026-10-25-hierarchy-candidate-command-manifests-design.md#closed-resource-ledger-subset)
already charge canonical record bodies and retained result payload occurrences at
assignment, owner-run and workspace scope. Their production numerical ceilings
remain unresolved. Propose those two resource limits and their denial behavior,
obtain scoped owner acceptance, then add independent logical boundary vectors and
source-owned transaction witnesses only within their existing authority/custody
gates. Do not edit old fixture caps or recast small synthetic witnesses as
production capacity proof.

This follows the [initial preparation budget components](../../validation/root-preparation-budgets.md).
It does not extend that no-reference/no-output profile into an eligible output or
review producer. The numerical choices here constrain the existing #744 record
and accepted-payload resources only. They supply neither staged/artifact custody
codecs nor a full source-profile implementation/qualification plan.

## Named proposed limits

All ceilings are inclusive; MiB means 1,048,576 bytes. These are author-proposed
starting defaults, not measured workload requirements or proven native capacities.

| Existing resource | Assignment | Owner run | Workspace across retained runs/epochs |
| --- | ---: | ---: | ---: |
| assignment-record-bytes | 64 MiB (67,108,864) | 128 MiB (134,217,728) | 256 MiB (268,435,456) |
| result-payload-bytes | 64 MiB (67,108,864) | 128 MiB (134,217,728) | 256 MiB (268,435,456) |

Apply each row independently at all three scopes. Unused payload space cannot
fund records, and unused record space cannot fund payloads. The root assignment
is one ordinary charged owner within its run; it is not a free pool or counted
again per child plan. A child/evaluated target uses its exact retained
AssignmentIdentity and its owning run, never the reporting runtime or currently
selected UI run. Run/workspace ceilings are shared, not an allowance per subject,
plan, generation, active epoch or producer.

Use the existing quota_usage key `{owner,resource}` and exact
`{used:Counter,reserved:Counter,lastCommit:CommitRef}` value unchanged. Counters
retain the storage contract's checked canonical u64 TEXT representation, without
numeric coercion, overflow or wrap. reserved stays zero for these two retained
resource pools. This proposal introduces no new tag, row family, source field,
reservation credit, digest domain or trusted caller-supplied usage total.

Keep the 512 KiB per-record bounds, 32-candidate/result and evaluation/revision
counts, structured output/declaration/snapshot limits, and conservative per-candidate
rehash policy unchanged. The accepted 17/64/256 MiB receipt pools are separate.
Their reservations cannot be consumed or released to satisfy these byte pools.
All independent command, summary/lineage, affected-row, proof/history, complete
logical store and native physical/resource limits still apply with their own
scope and acceptance status. These two quotas do not accept or expand a storage
profile, and do not raise an existing store limit.

## Exact charged inventory and ownership

| Resource | Included occurrence | Excluded from this resource only |
| --- | --- | --- |
| assignment-record-bytes | Complete canonical source body of every retained AssignmentResultCandidate, AssignmentResult, AssignmentEvaluation and EvaluationDisposition object owned by the target/producing subject as specified below | Storage keys/envelopes/commit links, heads, receipts/lookup/claim/work metadata, dependency and summary/lineage rows; all still count in their own command/whole-store budgets |
| result-payload-bytes | Raw byte length of every retained present-output payload row, keyed by exact AssignmentIdentity, accepted result revision and declaration ID | Metadata wrappers, known-missing outputs with no payload row, and independent staged/producer artifacts; none becomes exempt from its owning budget |

For candidates and results, charge their producing AssignmentIdentity, that
identity's workspace/run and the workspace. For evaluations and dispositions,
charge the evaluated target AssignmentIdentity and its owner run/workspace,
even if a reviewer in another run supplied the source artifact. Validate the
full object key, source body, exact target result and retained immutable joins
before deriving that owner. A reviewer's current runtime, own assignment or
artifact source location cannot redirect a target-record charge.

Reviewer artifact production remains additionally charged to its producing
assignment under its own still-required producer custody/budget contract. If the
same exact bytes are later retained as an accepted result payload, that result's
payload occurrence also counts fully. This is consumer accounting, not permission
to create a second artifact identity or charge the target's pool instead of the
producer's. An ordinary retained input/evidence reference does not itself copy
payload bytes, but still needs its own finite read/evidence/dependency contract.

Include every retained revision, rejected/superseded/abandoned candidate body,
old result, corrected/replaced evaluation and invalidation disposition. Advancing
a head, invalidating a stream or rejecting candidate finalization removes no
immutable object-body charge. Only the exact deleted owned occurrence in a
separately eligible complete purge can subtract usage. Keep old epochs and closed
subjects in the same run/workspace sums.

Present outputs each own a result-keyed payload occurrence, including zero bytes.
Two equal-content declarations, two accepted revisions or two result owners count
separately, without digest deduplication or physical-sharing discounts. A
zero-byte payload adds zero to this raw-byte resource but still requires its full
row, key, digest and manifest/physical accounting. Known missingness cannot be
invented to make a declared or actual oversize source fit.

Staged copies, pending candidate payload, immutable server/reviewer artifacts,
report/plan/configuration bytes, external files and scratch are not accepted-result
payload rows. They cannot borrow these retained-result pools as their only budget.
The [existing source contract](2026-10-04-assignment-evaluation-design.md#bounds-retention-and-deletion)
requires finite producing-owner staging/artifact and broader quotas before code.
Those owners/codecs and finite custody effects remain separate no-go gates; this
proposal does not claim to bound all producing-assignment storage.

## Atomic deltas, admission and retry

Derive usage from the complete authoritative retained inventory, compare it with
every required existing owner ledger, and validate the whole immutable intent.
Missing/unknown owners, ambiguous source kinds, orphan payloads, missing ledgers,
noncanonical bodies or mismatched sums make the affected store unavailable.
Do not initialize a missing historical row at zero or choose a head from the
largest revision. Fresh owner initialization retains its existing producer gate.

For each command, sum exact per-owner/resource deltas first; replace each changed
quota row once in the same complete transaction as the source, receipt, lookup,
claim/outcome, dependency protection, summary/checkpoint/lineage and controls.
No change means no ledger/link rewrite. The existing logical delta recipes remain:

| Command | Record used delta | Payload used delta |
| --- | ---: | ---: |
| New candidate admission | Full new canonical candidate body | 0; admission copies/hashes no output content |
| Accepted finalization | Full new canonical result body | Sum of every present retained payload occurrence |
| Rejected finalization / supersession / abandonment | 0; old candidate body remains charged | 0; no accepted result/payload created |
| Evaluation creation/correction/replacement | Full new canonical evaluation body at target owner | 0 unless a separate accepted-result publication actually creates a payload occurrence |
| Eligible evaluation invalidation | Full new canonical disposition body at target owner | 0; invalidating a consumer does not delete target result payloads |
| Work reservation / exact terminal retry or receipt read | 0 | 0 |

An independently scoped reviewer artifact publication still needs its own exact
complete producer effects; the table does not invent a generic artifact writer.
SubjectRevision/CAS, source eligibility, current authentication and the operation's
accepted claim or distinct finalization cutoff apply unchanged. A claim protects
only its exact allowed attempt, not stale quotas or a larger effect set. Revalidate
captured headers, owners, inventory and all independent capacities under the
separately qualified native ordering before commit. Stage/copy work outside core
locks retains its existing durable positive rehash reservations; these new totals
are neither I/O deadlines nor a source of free content passes.

An identical currently authorized retry resolves the original exact receipt before
new admission/CAS checks and adds no bytes, proofs, revisions or rows. A full but
valid resource does not prevent that read. Authentication/review-context rules,
source-byte equality and corruption/recovery checks still apply; a changed retry
payload conflicts and a missing receipt is not reconstructed from a high-water mark.

## Capacity denial and protected receipt reservations

These are new proposed logical admission limits, not reserved future record or
payload capacity. A candidate that fits at admission has no guarantee that its
later result body/payload fits. A reserved user receipt credit likewise does not
reserve a maximum-size evaluation or disposition body in these zero-reserved
byte ledgers. Other assignments may fill shared run/workspace capacity first.
The owner must explicitly accept or revise this usability cost before fixtures
or an executable profile adopts the limits.

If any of the six final totals exceeds its ceiling, deny the whole new publication
with a proposed bounded 409 capacity response: `assignment-record-capacity-exhausted`
or `result-payload-capacity-exhausted` for the failed resource. These are proposed
reason-code choices in the existing error envelope, not new source/wire fields.
Do not expose peer subjects' usage to a producer. Where both resources fail,
report the record reason first deterministically. A missing/corrupt/unverifiable
ledger or storage outcome retains the existing unavailable/recovery behavior,
rather than a fabricated capacity total. No safe-message bound is raised.

A capacity denial adds no mutation receipt, source object, result/revision/head,
lookup, partial ledger change or summary transition. It cannot evict evidence,
truncate output, overwrite a prior evaluation, consume a protected receipt credit,
refund earlier work reservations or silently change the configured output contract.
An open candidate whose attempted acceptance cannot fit remains in its prior
state; capacity denial does not manufacture a durable rejected finalization,
source failure or Needs rework judgment. Existing independent source/work-budget
terminal-rejection rules remain unchanged. A later explicitly authorized action
must still pass all its original checks; no automatic retry/resume is granted.

An eligible internal rejected-finalization command adds no record/payload usage
and can use its reserved outcome receipt without a new receipt-pool reservation.
It must still fit its complete metadata/proof/summary/physical effects; there is
no guarantee of writability. Existing user correction/invalidation credits remain
owned, protected and unspendable by other purposes when a byte-cap denial occurs.
If a correctness-critical source-health/dependency change cannot durably publish
its required protection, keep the existing health/unavailable fence; do not leave
stale passing material current merely because a resource is full. No new health
mutation, authority exception or cleanup bypass is supplied here.

## Retention and arithmetic consequences

No TTL, head-only recount, automatic pruning, namespace rotation, resume, token
renewal or run/plan completion releases retained record or payload usage. Explicit
purge still needs closed-subject authority, exact subjectRevision, incoming-consumer
and shared-custody proof and complete atomic effects; this proposal supplies no
purge implementation or permanent saturation recovery route. Ordinary stop/PTY
controls remain independent of hierarchy SQL.

The 64 MiB record pool is deliberately smaller than all independent source maxima
combined: 32 candidates plus 32 results at 512 KiB each consume 32 MiB; the
remaining 32 MiB holds only 64 more 512 KiB record bodies, before any other retained
evaluation/disposition. The 512-evaluation allowance is a revision upper bound,
not a guarantee that all maximum-size revisions fit. Existing smaller canonical
records may permit more history. No maximum workload or eligible profile is
inferred from this source-only arithmetic.

Eight 8 MiB accepted-payload occurrences would exhaust one assignment's 64 MiB
pool; two such full assignment pools exhaust the run's 128 MiB, and two full run
pools exhaust the workspace's 256 MiB. These are independent retained-byte examples,
not executed publications or reserved allocations. A 16 MiB new-payload command
already exceeds the inherited proposed 16 MiB complete-command cap once mandatory
metadata is included; larger aggregate headroom cannot override that denial.

The two 256 MiB workspace pools total 512 MiB of source bodies/raw payload alone,
before receipts, preparation, metadata, summaries or proofs. They cannot both be
maximal inside a store capped at 512 MiB complete logical bytes. The initial
preparation profile's accepted limits do not make a mixed producer store eligible;
any future mixed profile must retain all applicable stricter store/command bounds
and obtain its own closed registry/effects/custody/native qualification. Independent
ceilings are not a promise of simultaneous fit or a reason to raise a store limit.

Rejected alternatives: unlimited historical revisions; treating fixture-only
1 MiB/zero-payload caps as production policy; pooling receipt/source/payload credits;
using content hashes or current heads to erase occurrence charges; reserving every
possible future maximum body/payload through a new ledger protocol; and granting
capacity by deleting protected evidence. Future reserved-source capacity would
require its own owner-reviewed protocol, not an implicit change to reserved=0.

## Boundary handoff after scoped acceptance

| Case | Required oracle |
| --- | --- |
| Each resource/owner equals its cap; one new byte exceeds it | Exact fit may proceed only if every other check fits; overflow denies whole command |
| Assignment fits but a peer filled run/workspace usage | Deny; no per-subject clone or borrowing from another resource |
| Corrected evaluation adds a new revision | Charge full new body, keep all prior bodies; no head-replacement discount |
| User has a protected write/invalidation credit but record space is full | Byte-cap denial; credit and old source/history remain unchanged, with no fabricated success |
| Candidate fits but final result or payload cannot fit | Preserve open candidate/old head and work charge; no partial result or automatic quality rejection |
| Capacity-full result owner receives a target evaluation from another run | Target owner still pays full record bytes; reviewer artifact custody is additionally enforced by its own producer |
| Same raw bytes appear in two declarations/revisions/owners | Charge each accepted payload occurrence separately, including source and target producer obligations |
| Present zero-byte versus known-missing output | Zero-byte row is retained and fully metadata-accounted; missing output has no payload row and retains source-defined semantics |
| Terminal rejection at full record/payload pools | Zero source/payload delta, existing outcome reservation; independent full-command/storage failure still fails closed |
| Exact currently authorized retry at full valid pools | Return original receipt without new charge; changed bytes or revoked scope still fail |
| Unknown/orphan/missing historical owner/ledger or over-limit reload | Unavailable; no partial sum, inferred zero or automatic new epoch |
| Explicit purge or health protection cannot commit | Preserve evidence/fences under the owning contract; no quota-driven destructive fallback |

These are written oracles, not executed tests. After owner acceptance, independently
pin component boundary totals and source-traced complete deltas with cases for
root/child owners, cross-run reviewers, historical epochs and duplicate payloads.
Maximum-length source/payload/key/wrapper/proof witnesses must establish fit or
explicit denial under all composing limits. Neither a small old witness nor an
isolated arithmetic comparison qualifies a source-valid maximum transaction.

## Owner decisions and remaining gates

Review the two independent 64/128/256 MiB resource pools, existing zero-reserved
semantics, full-history/occurrence charging, proposed deterministic 409 reasons,
and lack of future source-capacity guarantees for admitted candidates or protected
user receipt credits. This is a draft policy; merging stores it without accepting
those choices or authorizing dependent fixtures.

Staged candidate/immutable artifact/report/plan/evidence/read/worker/scratch quotas,
full producer/affected-set codecs and cross-owner custody, protected purge/recovery,
production maximum-profile transactions and bounded native enumeration/resource/
physical/durability/finalization authority remain separate gates. Existing no-pruning
preparation exhaustion is unchanged. #740 stays deferred/no-go until November;
broader component review and separately approved executable plans remain required.
Keep #744/#746 and the initiative open.


## Owner acceptance and component handoff — 2026-10-30

The owner said “go ahead” after the two separate 64/128/256 MiB pools,
no future-capacity reservation, retained history and explicit capacity-error
behavior were presented as the next review decision. This accepts the complete
named policy in this proposal for documentation fixtures and further logical
qualification: inclusive independent ceilings at assignment/owner-run/workspace;
unchanged zero-reserved and full-history/per-payload-occurrence accounting;
record-before-payload deterministic bounded 409 reasons; no automatic eviction;
and no guarantee that candidate admission or protected receipt credits reserve
future record/payload capacity. A denied publication preserves existing evidence,
credits and prior candidate/work state without a fabricated quality outcome.
No numerical value, owner mapping, schema, authority or codec is changed.

The [boundary component packet](../../validation/assignment-aggregate-budgets.md)
pins isolated inclusive totals, shared-scope saturation, independent pools and
counter/error precedence. It supplies arithmetic evidence only. Existing contract
bodies and earlier fixture/checker catalogs remain unchanged. In particular,
zero-delta arithmetic at full pools is not authenticated retry or publication
proof; fixtures do not establish actual inventory ownership, source validity,
receipt preservation or native atomicity.

Next: source-traced retained-history/occurrence and complete delta witnesses for
root/child targets, cross-run reviewers, old epochs, corrections, invalidations,
finalization denial and exact retries, under the owning source/authority/custody
contracts. Reconcile complete manifests and independent logical/physical limits
before a separately approved executable plan. Staging/artifact/report/plan/evidence/
read/worker/scratch quotas, custody, purge/recovery, native qualification, #740
and broader component/runtime gates remain independent. Keep #744/#746 open.


## Source-accounting component evidence — 2026-10-30

The [source-accounting packet](../../validation/assignment-accounting-witnesses.md)
recounts the existing complete root empty-output witness under its original
fixture caps and binds sixteen archived root/child source charge components.
The retained candidate plus accepted result charge is 1,362 bytes at each owner;
complete admission/acceptance commands remain 23 rows / 28,371 and 30,151 bytes.
No earlier catalog or checker bytes change. Cross-run evaluation source charges
belong to the target owner, with reviewer artifact custody still separate.
The archived correction's placeholder predecessor explicitly prevents a claim
of source-valid retained evaluation history. Payloads, coherent reviewer/source
histories, denial/retry/old-epoch complete effects, custody, production maxima,
native and executable-plan gates remain open; this is accounting evidence only.
