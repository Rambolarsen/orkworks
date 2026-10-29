---
type: Design Proposal
title: Terminal-run report correction publication profile
description: Proposed closed report receipts, terminal run variant, retained evaluation dependency mapping and exact correction effects for bounded logical transaction fixtures.
tags: [orkworks, reports, hierarchy, storage, transactions, dependencies]
status: draft
---

# Terminal-run report correction publication profile

Tracking: [#1136](https://github.com/Rambolarsen/orkworks/issues/1136),
[#742](https://github.com/Rambolarsen/orkworks/issues/742),
[#744](https://github.com/Rambolarsen/orkworks/issues/744),
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089), initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738).
Inspection base: `42e8341dd9b049049ff54616b21fc6363029d964`.
**Proposed for scoped owner/producer review. No dependent fixture/checker,
DDL/migration, native probe, executable qualification plan or runtime code.**

## Decision and boundary

The [publication audit](../../validation/r2-publication-boundary.md) identifies
missing report receipts and mutable consumers. Choose a small first publication
profile: an Electron-authorized user correction to an existing report in a
completed or cancelled current-epoch run, with retained evaluation consumers only.
This covers the two R2 source corrections while leaving terminal outcomes intact.
The author proposes the new fields, mappings and storage choices below; instruction
to draft this document is not acceptance of them.

A full active-run profile needs preparation revisions, proposal invalidation,
approved-plan launch fences and launched-assignment blockers. Do not close those
by opaque JSON bags, an asserted empty caller list or a workspace-wide event.
Reject this profile if the run is active, any selected terminal shape differs,
any report-dependent consumer is not supported here, or complete discovery cannot
be proven. No initial report, child correction, source deletion, purge, learning,
report withdrawal, plan transition or lifecycle conversion is a command in this
profile. Existing historical child versions can be retained and retried only
through their original authority contract; terminal child authority is revoked.

This is a **conditional logical storage profile**, not a complete admitted-run
producer. A fixture may supply a synthetic terminal prestate and frozen source
context, labelled as such. It cannot claim that the full admission, research-plan
or terminal-transition producer has been qualified. Creating this terminal
variant from a running/preparing run requires that producer's separately reviewed
complete effects; this document supplies no conversion, import or migration path.
Unknown existing row variants remain inadmissible, rather than being normalized
into this profile. No fixture may mutate the prior R2 packet to imply it was an
admitted terminal run; reuse its source bytes under a separate explicit context.

## Common types and source preservation

Use #741 canonical JSON and #742 wire counter/ID/digest rules unchanged. All listed
fields are required, unknown fields reject, and null is allowed only where stated.
Storage Counter/CommitRef/ObjectRef and format-3 row/manifest domains are those in
the [storage contract](2026-10-24-hierarchy-storage-contract-design.md).
Use exact source JSON integer encodings, not storage Counter strings, inside
ResearchReport, request, receipt and terminal run source fields.

S means the complete existing research-report stream key. H means the existing
ResearchReportHead. R means the unchanged complete report descriptor, including
contentDigest. Its ObjectRef uses the existing research-report kind, full #744
ReportIdentity and source digest. Reconstruct that identity from R; no shortened
source identity or new report hash domain is introduced. Retain exact old report
bytes and original author/generations. A user replacement copies predecessor
lineage, approved questions and scope under the existing source-owner checks.

## Closed terminal run variant

Extend run_states with exactly this alternative value, keyed by
`{workspaceId,runId}`:

```text
{schemaVersion:1,terminal:TerminalReportRunV1,lastCommit:CommitRef}

TerminalReportRunV1 =
 {workspaceId,runId,parentSessionId,admissionEpoch,runSequence,
  runDefinitionDigest,bootstrapConfigurationDigest,state,runVersion,
  inputRevision,inputDigest,plans,currentPlan:null,
  currentParentRuntimeGeneration:null,blockers:[],
  stageContinueRequired:false,pendingClarification:null}

plans = [{planId,planRevision,planDefinitionDigest}, ...]
```

state is exactly completed or cancelled. Other run field types are inherited:
runVersion 1..2^31−1, inputRevision 1..64, runSequence the positive admission
sequence string, epoch and digests lowercase 64 hex, IDs bounded ASCII.
runId equals admissionEpoch + one hyphen + runSequence. plans retains source
order, has 1..16 unique plan IDs and each revision 1..64; it is the frozen retained
plan-reference list, not embedded mutable proposals or grants. The selected
report's plan/revision must appear exactly. This profile excludes terminal runs
with nonempty blockers or other unsupported state; it cannot erase them to fit.
The complete key plus value is at most 8 KiB. Existing initial run_states variants
remain unchanged and cannot be accepted here merely because their keys match.

This variant is the sole mutable runVersion owner for its selected terminal run,
not a second cached counter beside another run-state row. Retained plan references
do not claim that the plan/configuration codecs or native source provenance are
qualified. The source owner must independently prove frozen question/configuration
scope and that run termination revoked grants, closed child authority and settled
pending reporting work. Missing proof is unavailable. The initial bootstrap or a
flag in this row alone cannot prove those events.

Correction increments only runVersion in this value. Preserve inputRevision,
inputDigest, terminal state, ordered plans and all other bytes. No new preparation
revision, proposal transition, stage continuation or task version/outcome change.
Terminal authority and ordinary-session lifecycle remain unchanged.

## Report head and receipt registry

For this profile use a compact source-owned report_heads table, rather than
materializing a duplicate generic heads row for the same stream. It maps to the
same logical research-report HeadKey; no competing head authority is permitted.

| Table / exact key | Exact value |
| --- | --- |
| report_heads / S | `{schemaVersion:1,head:H,lastCommit:CommitRef}` |
| report_receipts / `{stream:S,reportVersion}` | `{schemaVersion:1,operation,expectedTaskVersion,expectedRunVersion,expectedReportHead,requestDigest,response:ResearchReportReceiptV1,lastCommit:CommitRef}` |

The response has exactly `{schemaVersion:1,acceptedHead:H}`. Its canonical UTF-8
bytes are the response body: new acceptance returns HTTP 201, exact retry 200,
with identical body bytes. No current-head flag, live run version, task outcome,
event cursor, bearer or permission is included. The surrounding authenticated
stream scope binds the response; a detached receipt does not select a stream.

Retained receipt operations are submitResearchReport or correctResearchReport.
For submit, expectedTaskVersion is the original positive #742 version and
expectedRunVersion is null; author in the joined R is child. For correct,
expectedTaskVersion is null, expectedRunVersion is positive and joined author
is user. expectedReportHead is null only for version 1 child submission, otherwise
the exact prior H. This profile's **new writes** are correct only, versions 2..8.
It defines storage for retained prior receipt bindings, not authority to publish
child history or synthesize missing receipts during recovery.

Resolve response.acceptedHead to the exact immutable R with key.stream and that
version. Reconstruct the exact normalized request from operation, S, expected
versions/head and R, using the existing #742 request domain. requestDigest must
match; canonical request bytes must compare exactly on retry, not digest alone.
Thus the row binds author through R without copying it or the descriptor into
receipt metadata. Recompute canonical response bytes from the closed response;
no stored free-form response string can disagree with it. A collision or changed
request for the same key fails closed.

Each accepted version has one receipt and one immutable report. The reportId is
fixed across the stream's complete contiguous prefix 1..head.reportVersion; all
head/receipt/descriptor joins must match. History may not elect a head by taking
the maximum surviving version. A missing or extra accepted-version binding is
corruption, not permission to recreate it from report catalog presence.

The full report_heads value, including lastCommit, must fit the existing 512-byte
head metadata limit; its stream key is charged separately to aggregate/command
storage. Complete report_receipts key plus value must fit the existing 2 KiB per
version metadata ceiling. The source report and reconstructed normalized request
each independently fit 32 KiB, including all request overhead. At maximum ASCII
IDs/counters, author calculations give 996 key bytes plus 973 value bytes for a
user receipt (1,969 total), leaving 79 bytes under 2,048. This is sizing arithmetic,
not executed golden-vector qualification. Actual canonical bytes must always be
measured; the smaller containing limit wins. Never duplicate the 32 KiB descriptor
inside a 2 KiB receipt or charge it to #744 protected slots.

For run_states exclude only outer lastCommit from its row commitment. For
report_heads/report_receipts exclude only outer lastCommit. Keep the complete
key, terminal data, head, expected values, requestDigest and response in the
committed body. All excluded links resolve to the original exact proof. Receipt
responses carry no CommitRef, avoiding receipt/commit recursion. Existing
format-3 exclusions and all other source domains remain unchanged. Registry
extensions require explicit profile recognition; no existing database auto-upgrade.

## Complete consumer discovery and retained edges

For this first profile, a report may be referenced by a retained #744
assignment-evidence-source object of kind research-report. Its full source
identity/sizeBytes/sourceKey must join the report exactly. Admit this mapping:

```text
consumer = that evidence-source ObjectRef
 target = exact research-report ObjectRef
 purpose = evaluation-evidence

consumer = retained assignment-evaluation ObjectRef
 target = each exact cited evidence-source ObjectRef
 purpose = evaluation-evidence
```

These are explicit mappings of the existing dependencies purpose, not new source
fields. Canonical evaluation outcomes/dimensions/findings determine the exact
unique source set. Preserve original report-source edges when heads advance;
retained stale evidence still protects old bytes. No edge is removed or retargeted
by report correction. Uncited newer evidence is not substituted into a review.
EvidenceSource objects with no retained incoming evaluation can still protect their
report while retained; their owner/deletion rules remain separately gated.

Under the same workspace transaction/dependency boundary, derive the affected set
from retained source bodies and authoritative edges, never from a caller list or
only current evaluation heads. First verify complete known workspace object and
dependency membership, then traverse each old-report evidence-source and its
retained evaluation consumers. Recompute every selected source key and edge from
its immutable owner and compare exact membership. Missing and extra edges both
fence the operation. For every encountered evaluation, resolve its exact assignment,
result, reviewer head/disposition and eligibility/currentness inputs under #744;
include the subject even when that evaluation is historical or presently excluded.
Also resolve its owner run for snapshot invalidation.

Any other report consumer (review input, configuration, preparation, proposal,
plan, launched-assignment binding, learning or unknown kind) makes this profile
ineligible. Their records/edges must remain intact. A retained plan reference in
the terminal run is not itself evidence that the plan consumes this report; prove
the absence of such consumers from their source-owner inventory. If those owners
cannot supply that proof, fail unavailable. This profile never treats an unclosed
consumer as an empty set and cannot certify a full admitted run from this scan.

All visited rows count toward the existing 100,000 per-command bound, including
completeness scans, lookups and final revalidation; stop at overflow and deny,
never truncate or split the atomic affected set. The implementation must qualify
physical indexes/query accounting and the existing cooperative SQL deadline.
No full file/transcript/network read belongs in this scan. Recovery must verify
retained source-to-edge equality and all prefixes before current reads, writes or
purge; a valid edge list cannot prove completeness without its owner inventory.

## Exact report-publication effects

After existing Electron identity/current-epoch/lease/health validation, consume
or check the per-run user rate before any receipt lookup. Validate source scope
and immutable predecessor normalization. An exact accepted receipt returns before
live run/head CAS with **no durable changes**. Revoked/foreign scope, rate denial,
unsupported consumer, missing source, malformed request or corruption never gains
an accepted retry from an otherwise matching key. Retained retry needs the same
complete current authorization/source-access proof, not renewed child authority.

For a new request compare expectedRunVersion and expectedReportHead, require next
version and fixed reportId, and rebuild the complete immutable command if its
captured state changes. Set C to the next checked CommitRef. Let Affected be the
complete source-derived assignment set, Q the exact changed resource rows, S the
exact summary/lineage changes below, and K header replacement plus proof insertion.
The complete changed set is exactly:

| Effect | Required change |
| --- | --- |
| Immutable report | Insert one objects row for R; absent key required, source hash verified |
| Head | Replace exactly report_heads[S] with new H |
| Receipt | Insert exactly report_receipts[{stream:S,reportVersion}] with the normalized request binding and closed response |
| Run | Replace exactly run_states[workspace/run], runVersion+1 only in terminal data |
| Summary/lineage | S for every affected assignment under the branches below |
| Resources | Q computed from complete old/new key/body occurrences |
| Control | K under the unchanged format-3 header/manifest equations |

No dependency row changes, #744 receipt slots/lookup/claim rows, payload, candidate,
proposal, plan, task, preparation, runtime, grant or event row is changed. No proof
pruning occurs in this profile; retained proofs must fit capacity. The manifest's
changes are the whole changed set minus K, sorted uniquely; original before and
final after commitments appear once. New/replaced links equal C; unchanged rows
and immutable history retain their original links. Counter/commit overflow denies.

For each affected assignment derive old/new #744 aggregate material from exact
accepted result, retained evaluation selection/eligibility and changed source head.
Preserve the one immutable evaluation. Do not assume a changed report must change
material. For subjectRevision compare the exact before/after evidence-currentness map
for the subject's accepted result and its selected eligible, non-invalidated
evaluation-stream heads. For every such evaluation include every distinct cited
research-report source, across required/optional outcomes, quality and findings,
as `{evaluation:EvaluationReference,sourceKey,current}`. current is the existing
source-owner exact report-head/digest match plus all unchanged source-health proofs;
sort by canonical EvaluationReference bytes then sourceKey, retaining no duplicates.
These comparison tuples are derived inputs, not new stored fields or source records.
If any tuple is added/removed or its current value changes, advance the existing
summary_state.subjectRevision exactly once and link that row to C, with checked
u64 overflow denial. No other field encodes the CAS event; unchanged material,
checkpoint and pending fields remain byte-identical. Mere discovery of an unused
historical, invalidated or ineligible evaluation does not trigger this predicate.
R2 A and B both change one selected source-currentness tuple and therefore advance
subjectRevision once, although only A advances summaryRevision. This source-health
CAS mapping is a new proposal for owner review, separate from source record bytes.

For changed material use the existing current-row/checkpoint branch if capacity
fits, or retain its checkpoint, publish reconciling and append the exact ordered
before/after pending transition. Terminal exhausted rows stay exhausted; no wrap
or impossible new material revision. A never-initialized subject stays unavailable
without invented revision 1. If material is equal, preserve revision/checkpoint/
pending bytes; only a required source-health subjectRevision change may replace
summary_state. Missing admission, checkpoint, aggregate or dependency proof denies
publication in this profile; it supplies no new generic health-fence record.
If required reconciling/pending protection cannot persist, reject the entire source
command. No read or correction opportunistically compacts pending history.

R2 A therefore advances material revision 3→4 and excludes only stale f-c1, with
C2 still failing. R2 B keeps revision 3 and both required findings. Both publish
changed detail freshness and retain the same historical evaluation bytes.
These are conditional oracles under a separate terminal fixture context.

## Resource and command budgets

Add exactly report-retained-bytes for run and workspace owners in quota_usage;
reserved is always zero. Used is the sum of canonical key bytes plus committed-body
projection bytes for all research-report objects, report_heads and report_receipts
owned by that scope. Exclude only their named outer storage links. Repeated source
bytes in distinct stored occurrences are charged separately. No physical sharing
discount or #744 receipt credit applies. These rows are pre-existing prerequisites:
report-publication cannot create/reset a missing ledger or initialize an old store.
Initialization and producer-owned admission need a separately reviewed integration.

The run's new resource shares the existing 32 MiB preparation/report aggregate,
not an additional allowance. Its source owner must provide complete non-report
preparation usage for the same run; unknown usage denies. The author proposes a
256 MiB workspace report-retained-bytes ceiling. This is separate from #744 receipt
pools and bounded again by the existing 1 GiB main-file and command limits. Charge
terminal run-state bytes to the existing preparation-retained-bytes run/workspace
resource, preserving all other preparation occurrence charges. No run-count or
report-version credit is released by terminal status or retry.

Q contains only actual changed report run/workspace and preparation run/workspace
rows plus changed summary resources. Recompute exact summary-row count/byte and
checkpoint/pending lineage byte/count deltas under their existing shared limits.
Report correction adds no summary subject or incoming retained edge. Summary
capacity denial cannot spend hypothetical source quota refunds. All shared usage
must match retained ownership before commit; a missing/ambiguous ledger is no-go.

The existing command bounds remain 16 MiB and 4,096 changed logical rows, counting
all metadata keys, new source bytes, full receipts, summary/checkpoint/pending,
quotas, proofs and control. Deletion is not negative command credit. Old/new intent
and retained predecessor copies also consume their existing worker/scratch limits.
Test exact command byte sums; source legality alone guarantees no publication fit.
The 100,000 visited-row bound and queue/deadline/native limits are independent.
No batching, independent rename or post-commit write may omit an atomic effect.

## Authority, snapshots and recovery

#744 claim-first authority does not apply. This profile preserves #742's current
UI authority/current-epoch and workspace ownership cutoff; no permission survives
revocation through a new claim here. The adapter must demonstrate that ordering
through durable disposition without holding core/publication locks across SQL or
I/O. If it cannot, the profile remains native/runtime no-go; this document does
not solve that excluded authority race by changing a receipt or terminal flag.

Publish exact affected volatile run-snapshot invalidations after durable commit
and before releasing the ordered publication position or serving another current
read. Include the source run and each dependent assignment's tracked owner run
whose visible detail changed, even when material/overall is identical. Invalidate
once per changed run per committed command; exact retry does not invalidate again.
A fresh sidecar gets a new snapshot lifetime and recovers durable lineage before
serving reads. A volatile counter is not a durable material revision.

Capture/final validation racing either R2 correction rejects the old detail as
stale-reference when its bound snapshot changed. Missing source/index/lineage proof
is unavailable, not invented current detail or zero metrics. Preserve exact
immutable inspection on its own narrower proof. Equal snapshot versions cannot
stand for different visible outcome/source data, including optional-only changes.
Events follow commit and remain hints. Neither a receipt nor buffered success can
mark viewed; the accepted explicit-open/complete-material presentation rule remains.

Recovery yields complete old state, complete committed new state, or unavailable
reconciliation. Never expose a new head alongside current old dependent evidence.
Verify manifest/row links, source hashes, receipt prefix/request equality, resource
ownership and complete dependency mapping before reads or retry. No largest-version
head election, credential reconstruction, history retokening or event replay repair.
If commit succeeded before acknowledgement, authorized retry returns the original
closed response without new head/material/proof/snapshot effects. Physical crash,
flush, lease, power-loss and platform evidence need their own approved plan.

## Review choices and next evidence

Owner/producer review is requested for: (1) terminal-run/evaluation-only scope and
fail-closed exclusions; (2) terminal variant, compact head/receipt rows and exact
response/storage mapping; (3) retained two-hop edges, subjectRevision source-health
mapping and bounded completeness discovery; (4) 256 MiB workspace report quota and
shared run/preparation accounting. All definitions above are proposals until that
scoped acceptance, not conclusions inferred from a merge or instruction to draft.

After acceptance, create independent conditional terminal-prestate before/after/
denial witnesses for both R2 branches, exact retry after a later head, changed-key
conflict, missing/extra edge, unsupported consumer, current-scope denial, ledger/
command overflow, reconciling capacity, terminal exhaustion and snapshot mismatch.
Measure maximum row/receipt/command bytes and preserve the original component
catalog. Keep synthetic admission/termination/authority inputs explicitly unverified.

For those future conditional fixtures, use a harness-only mutation oracle with
exactly `{outcome,beforeStateDigest,afterStateDigest,commit,response}`. outcome is
accepted, retry, conflict, denied, unavailable or limit-exceeded; commit is C only
for a new accepted publication and null otherwise; response is the closed receipt
only for accepted/retry and null otherwise. State digests are raw SHA-256 of the
canonical complete profile-owned `{table,key,body}` row inventory, sorted by table
then canonical key, including header/proofs and all full links. These are harness
fingerprints, not a new source domain, API response or production receipt. Compare
complete inventories as well as digests. Every non-accepted outcome has equal
before/after inventories and no new durable publication effects; retry additionally
returns the exact original response. Independent authenticated request-rate charging
still happens before lookup and is not rolled back or refunded by this oracle.
If corrupted prestate is supplied, preserve its bytes and keep current reads
unavailable; a denial must not silently repair it. Read-failure expectations use
the already accepted stale-reference/unavailable detail codes separately, never a
partial success or viewed token. No free-form reason string chooses the outcome.
No dependent fixture/checker work starts in this proposal task.

Broader active/terminal lifecycle variants, initial/child report publication,
consumer/learning custody, producer/index creation, physical/native qualification,
#740 eligibility and separately approved executable plans remain independent.
Keep #1136 open for scoped review/acceptance and broader component issues open.

## Bounded builder and correctness review

One fresh read-only correctness review found no concrete owning-contract conflict;
receipt maximum-byte arithmetic, quota fit and native authority remain explicitly
unqualified. One fresh builder check requested a precise subjectRevision predicate
and denial oracle. The author added the exact selected-evidence comparison and
existing-row delta plus the harness-only no-commit/state-equality oracle above.
These author clarifications are proposed decisions, not owner acceptance or a
claim of post-edit independent rereview. Review covered specification planning;
no runtime tests, storage writes or native qualification were performed.

## Scoped owner acceptance — 2026-10-29

The owner explicitly answered “go ahead” after review of merged PR #1138 and
its four concrete choices: terminal-run user corrections with retained evaluation
consumers only; exact terminal/head/receipt storage and response mappings; retained
dependency and selected-evidence source-health CAS rules including detail-only
snapshot invalidation; and the 256 MiB workspace report quota within the existing
32 MiB per-run preparation/report aggregate. This accepts that profile for
conditional documentation fixtures, including its fail-closed exclusions.

It does not qualify admission or terminal transitions, completeness/authority/
custody/index/native proofs or an executable runtime plan. The earlier proposed
wording above is retained as design history. The
[conditional transaction evidence](../../validation/report-publication-components.md)
records exact component/row/commit witnesses under a separate synthetic terminal
context; original R2 source artifacts remain unchanged. Broader #742/#744/#746/
#1089, #740 and separately approved executable-plan gates remain independent.

## Focused producer and ordering handoff — 2026-10-29

The [qualification preparation plan](../plans/2026-10-29-terminal-report-publication-qualification.md)
separates the #742 user-authority/commit decision, real terminal producer/consumer
proof and a later bounded executable qualification plan. Terminal run completion/
cancellation does not itself revoke an explicit current-epoch UI correction.
Ordinary ended-session metadata is not orchestration settlement. The plan selects
no new cutoff and authorizes no adapter, native probe, migration or runtime work;
conditional evidence and existing approval gates remain unchanged.
