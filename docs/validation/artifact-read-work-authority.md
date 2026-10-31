---
type: Authority Investigation
title: Artifact read-work reservation authority and revocation
description: Accepted scoped debit-permit tradeoff, source-backed authority trace and gated command-proof/native qualification handoff.
tags: [orkworks, hierarchy, artifacts, authority, qualification]
status: draft
workflow_status: scoped-policy-accepted-qualification-required
---

# Artifact read-work reservation authority and revocation

Tracking: [#1184](https://github.com/Rambolarsen/orkworks/issues/1184),
[#1176](https://github.com/Rambolarsen/orkworks/issues/1176), initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738).
Inspection base: `754fab618e1d7cf8565dbfacbe54134ba5f2c48c`.
**Scoped debit-permit policy accepted for further design and qualification;
artifact read-work transactions remain no-go.** See the
[acceptance record](#owner-acceptance-and-command-proof-handoff--2026-10-31).
The investigation and proposed wording below record the pre-acceptance analysis
from PR #1209. Acceptance supersedes only its owner-decision-pending and
current-authority-at-commit status for this one debit. No command/proof codec,
native plan or runtime implementation is approved.

## Accepted boundary and investigation outcome

The [accepted artifact precharge](../superpowers/specs/2026-10-30-evaluation-artifact-producer-design.md#artifact-read-work-reservation)
requires current read authority at durable reservation, then again before content
fetch and response commitment. It explicitly excludes the accepted
[#744 publication claim exception](../superpowers/specs/2026-10-24-hierarchy-authority-ordering-design.md#amendment-scope-and-exclusions).
The [response cutoff](../superpowers/specs/2026-10-04-orchestrator-preparation-design.md#proposed-retained-review-input-admission)
serializes a bounded transport handoff with revocation. It cannot authorize an
earlier durable work debit. Persistence, content access and delivery are three
separate decisions, even when they concern the same immutable artifact.

The inspected mechanisms below do not establish revocation-before-physical-commit
denial for the debit. This is a bounded integration finding, not a proof that a
future native mechanism is impossible. Preserve the current cutoff/no-go unless
the owner accepts a specific alternative. The proposed alternative below changes
only the internal debit's authority cutoff; it grants no content or response right.

## Exact authority trace and owners

These are contract responsibilities; a named proposed module is not production
implementation. In particular, own-artifact reads use
`AssignmentEvaluationCapability`, not `AssignmentReviewReadCapability`.
The latter scopes retained target inputs for the reviewer and is not a substitute
for the former's own-output stream. Both use the #742 response cutoff.

| Stage | Exact binding and responsible owner | Status / denial consequence |
| --- | --- | --- |
| Framing and authentication | #744 reporting transport authenticates the evaluation capability's reviewer assignment, sidecar/child-launch generation, exact target and evaluation declaration; workspace/run/runtime and admission namespace fences apply before lookup | [Reporting actions/authentication](../superpowers/specs/2026-10-04-assignment-evaluation-design.md#authentication-exact-retry-and-atomic-writes) and action `orkworks.evaluation.artifact.read.v1` are accepted contracts; no production artifact handler/registry identified in this inspection. Planning/report/result/retained-input bearers cannot substitute |
| Rate and slot admission | #744 reporting owner charges workspace producer-read gate, then immutable producer-assignment gate, then producer slot; capability binding chooses the buckets | Existing 240 workspace / 30 assignment reads per rolling 60 seconds and four workspace producer slots; no refund of earlier admitted gates, no waiting queue. Authentication/rate admission is no persistence permission |
| Selected workspace and namespace | Proposed hierarchy publication owner binds selected workspace identity, sidecar generation and exact store/admission epoch; run/runtime transitions close the applicable capability | Ordinary workspace adoption has identity/lease owners, but no artifact authority transition coverage is established. Missing epoch/selection proof denies; current UI selection alone cannot authenticate a child |
| Lease | Workspace runtime owns the open advisory-lock guard; session application adopts/revalidates filesystem identity | Held guard is ownership evidence, not a read permission or commit/revoke interlock. Existence of `.sidecar.lock` is insufficient |
| Metadata and bound | Artifact/source owner resolves exact reviewer/target/declaration stream, evaluation revision/digest, immutable mapping/descriptor, content custody and protected consumer membership | Derive positive reply bound B from retained metadata and the accepted canonical wrapper before content fetch. Missing/corrupt mapping, ledger, header, consumer or custody proof is unavailable; never synthesize zero usage |
| Durable reservation | Proposed hierarchy store/private worker revalidates exact read set/header and assignment/run/workspace producing-owner ledgers, all quotas and complete command effects | Current read authority must hold at durable cutoff under accepted policy. No inspected native fence qualifies that condition. The ledger is work accounting, not a receipt, source publication or capability |
| Content fetch | Artifact materializer rechecks current exact authority, namespace/selection, lease and source/health/dependency fences after proven charge and before fetching original content | Debit alone is insufficient. Scope remains the original immutable version; no newest-head substitution. Integrity failure denies and preserves charge |
| Response commitment | Proposed hierarchy publication/transport owner verifies complete bounded bytes outside the short boundary and rechecks capability/generation/scope/custody and publication tokens under revocation serialization | Revoke-first discards buffer; handoff-first permits only those bytes to arrive later. No blocking network I/O under the boundary. No partial stream bypass |

The [storage owner split and publication boundary](../superpowers/specs/2026-10-23-hierarchy-transaction-storage-design.md#publication-and-ordinary-session-integration)
assign durable source/header/ledger/dependency validation to `hierarchy_store` and
pending/health/reference publication to `hierarchy_publication`. These remain
proposed modules. Actual source-health changes announce an immediate fence;
source-owner reconciliation must commit before it clears. A plain attention or
lifecycle display generation cannot stand in for source health. Complete incoming/
outgoing protected membership and fresh producer ledgers remain the separate
[#1183 source prerequisite](../superpowers/specs/2026-10-31-child-review-source-preparation-design.md).
No permit below freezes mutable source eligibility or clears an intervening fence.

## Production evidence and the unclosed race

Read at the pinned base:

- [terminal_runtime.rs](../../crates/orkworksd/src/runtime/terminal_runtime.rs),
  `set_workflow_report_token` (104), `clear_workflow_report_token` (125),
  `clear_workflow_report_token_if_matches` (132), and
  `verify_workflow_report_token` (146): a mutex protects live token lookup/removal.
  Verification returns a boolean after releasing that mutex. It neither owns an
  artifact SQL transaction nor carries its lock to physical commit. These are
  ordinary workflow-report credentials, not implemented evaluation credentials.
- [session_application.rs](../../crates/orkworksd/src/session_application.rs),
  `clear_ended_session_tracking_with_identity` (3486–3511): runtime-aware cleanup
  removes the report token and prompt authority. `complete_session_ending` (3651)
  checks runtime generation and persists replay/ending metadata; it does not
  settle an artifact debit. `open_workspace_with_identity` revalidates identity
  around acquiring/reusing an Arc lease (3773–3810). None supplies the proposed
  artifact registry's participation across all revoke/replace paths.
- [workspace_runtime.rs](../../crates/orkworksd/src/workspace_runtime.rs),
  `WorkspaceLease` (166–187): an open file owns the exclusive advisory lock until
  Drop. This excludes another cooperating workspace owner; it does not order
  same-process capability revocation against a work transaction.
- [coordinator_store.rs](../../crates/orkworksd/src/taskmaster/coordinator_store.rs),
  module contract (1–4), `StoredPlan` (49–55), `CoordinatorStore` (78 onward):
  durable definitions/approvals and a mutation mutex, explicitly no runtime
  authority. Durable plan data cannot restore a live read permission.

Reproduction: `rg -n` for the above symbols in those files, then bounded exact
search for `evaluation-artifact-read-work`, `AssignmentReviewReadCapability`,
`AssignmentEvaluationCapability`, `ClaimProofV2`, `evaluation_artifact_streams` in
`crates/orkworksd/src`, `apps/desktop/electron`, `apps/desktop/src`. The latter
returned no matches (exit 1). This search alone proves no architectural absence;
the inspected owner behavior is the evidence for the gap.

Concrete counterexample to a precheck-only adapter: authenticate/validate at T1;
release the authority lock; start a bounded SQL transaction at T2; revoke the
capability at T3; physical commit at T4. Nothing in the inspected report-token
mutex, lease or ending record makes T4 fail. A final response check at T5 can
prevent delivery, but leaves the forbidden post-revocation debit persisted.
Moving T3 behind SQL, or retaining an ordinary core/publication lock until T4,
would couple stop to SQL/fsync; that violates the accepted responsiveness
boundary. A callback/interrupt/timeout also needs proof of actual rollback before
it can report denial. The [existing ordering analysis](../superpowers/specs/2026-10-24-hierarchy-authority-ordering-design.md#why-a-choice-is-needed)
already distinguishes storage atomicity from runtime authority. No native probe
ran here and no impossibility claim follows.

## Proposed owner decision: one exact debit permit

This is an explicit proposed exception for only `evaluation-artifact-read-work`,
independent of #744 producer claims. Do not reuse `ClaimProofV2`, broaden its action
enum or interpret the accepted producer policy as acceptance of this proposal.
There is no new bearer, public API, reusable reservation or user-issued permit.

Proposed policy: immediately before SQL, the sole private worker may acquire one
volatile, immutable debit permit under the same short ordering as every applicable
revocation transition. Revoke-first denies it. Permit-first allows that one
attempted debit to reach proven commit/rollback after later capability revocation,
without authorizing any content fetch or delivery. The durable charge still
linearizes at physical commit. This explicitly relinquishes revoke-before-commit
denial for that exact debit only. It does not claim to implement the currently
accepted stronger cutoff.

The permit binds the authenticated assignment/reviewer and generation, sidecar,
workspace/store/namespace, original stream/version/digest/descriptor identity,
producing assignment/run/workspace ledger keys, B and exact internal command
identity. It cannot move to a resumed generation or replacement store. It freezes
only persistence permission; transaction validation still checks current durable
source/header/custody/dependency/health and all source/global caps. Actual lease
loss has no exemption. Permission alone cannot make an ineligible debit commit.

No parsing, hashing, copying, SQL/fsync, network, notifications or waits occur
inside that ordering. Every applicable child end, run cancel/final completion,
namespace close and workspace detach must participate; this is an evidence
obligation, not a claim of current universal coverage. Queue/rate/slot admission
creates no permit. The permit uses the existing sole worker slot; no additional
queue, connection, materializer, memory allowance or retired worker is proposed.
Ordinary stop publishes revocation and proceeds without waiting for debit I/O.

A successful transaction must bind nonsecret authorization-decision evidence to
its exact internal command/old-new ledger/header/proof effects. Exact canonical
shape, placement and versioned format-4 commitment need a separate reviewed codec
before conditional transaction fixtures: none is invented or accepted here.
Never persist token bytes. Evidence is audit/recovery data, not restored authority.
On restart recover complete old state or the complete original debit; never
recreate a volatile permit or resubmit uncertain work. No accepted mutation receipt
or new subject/snapshot is manufactured for this internal debit.

The one active-or-retiring worker retains captured store/lease and its charged
resources through proven disposition; a workspace switch immediately detaches
its old authority/publication scope. It cannot fetch content after detach or
publish into the newly selected workspace. New hierarchy work cannot bypass an
unresolved retired worker. Namespace replacement must not activate an epoch
around an unresolved old command: close admission, return bounded conflict and
reconcile first. Actual lease loss/uncertain locking stays unavailable. This
proposed lifecycle cost is explicit, not inherited permission from #744's claim.

User consequence: stop may be followed by one completed quota debit for work
whose artifact was never fetched or delivered. No refund means that capacity
remains consumed. A stuck worker may keep hierarchy storage/old lease unavailable
without blocking ordinary stop. The alternative is to retain the current cutoff
and no-go until a native fence is demonstrated. Do not replace lifetime accounting
with sliding rates, skip precharge, refund failures, or move debit after delivery.

## Complete state and failure oracles

Written conditional oracles only. Let U = (assignment, owner-run, workspace) used
read-work counters for the producing owner; caps C = (16, 64, 128) MiB inclusive.
B is the accepted positive canonical reply bound. Reserved usage stays zero.
A successful charge is U′ = U + (B, B, B), atomically with normal exact header/
proof effects; all other ledger values and source/artifact/receipt/subject/summary/
snapshot rows remain unchanged. B and command overhead must fit all existing
command/row/store/query/physical limits. Retained-source pools and read rates are
independent, not interchangeable credits.

| Case | Complete charge disposition and fetch/response oracle |
| --- | --- |
| Wrong class/generation/namespace/workspace, missing source/ledger or known revoked authority before admission | No new durable effect, no content/response. Already consumed earlier rate gates remain consumed; never initialize missing rows |
| Queue/slot admission followed by revoke before proposed permit | U unchanged, no new header/proof, no fetch or response; queue admission cannot grandfather work |
| Permit wins, revoke precedes commit | Under current policy this is no-go/required denial by a qualified fence. Only if the proposal is accepted may U′ and exact header/proof persist; no post-revoke fetch/response commitment |
| Durable predecessor, source/health/dependency or quota failure during transaction | Proven rollback preserves complete old state. No content fetch; permit grants no CAS/quota/health exemption; do not report quota exhaustion for uncertain/corrupt state |
| One component exactly reaches its cap / exceeds by one byte | Equality may commit only if every other bound fits; isolated overflow preserves all three old counters and header/proof, returns existing bounded capacity denial, fetches nothing |
| Commit succeeds, then disconnect, malformed content, digest/custody failure or revoked fetch check | Keep U′ and original header/proof, no refund; no successful content delivery inferred. Health/custody failure retains its unavailable fence |
| Authority changes during fetch/assembly before response handoff | Keep U′, discard uncommitted buffer; no later response commitment. The accepted pre-fetch check does not retract already-started I/O or delivered bytes |
| Response handoff wins, then revoke before client receipt | Keep U′; only committed bounded bytes may arrive. No next fetch/read or scope expansion |
| Crash/lost ack at uncertain commit | Keep one worker/position and fences until proven disposition. Recovery yields complete U or complete U′ with matching header/proof; no guessed rollback, refund or automatic replay. No content fetch without proof of charge and fresh live authority |
| Authorized caller rereads content after a lost reply | A new read requires fresh admission and another B charge. Recovery/metadata-only original receipt lookup is not a free content replay; authenticated receipt lookup does not charge B |
| Exact resume, new artifact version, completed run, retention or namespace rotation | Preserve cumulative historical usage; no capacity reset or permission revival. Fresh credential binds the exact approved scope; a new content read charges again |
| Namespace close during active debit | Current cutoff still requires qualified denial if revoke precedes commit. Under proposed permit-first policy only that old command may settle before replacement epoch activation; unknown disposition conflicts/unavailable, no old grant restoration |
| Workspace switch while debit or materializer is active | Immediately close old admission and detach responses; one captured old worker/lease/materializer and its memory remain charged until disposition/release. No replacement worker or old reference publication into new scope |
| Actual lease loss, epoch/source ownership uncertainty or broken lock support during I/O | No permission to commit through loss. Fence unavailable and reconcile on the same worker; interrupt does not prove rollback. Such a native adapter remains ineligible unless exclusive ownership/disposition can be proved |

All cases retain the accepted one active-or-retiring materializer, zero queued
materializers and 4 MiB aggregate instance memory across selected/retiring buffers.
Do not add retries, store allowances or capacity by switching namespaces/workspaces.

## Reviewed handoff and remaining gates

1. Owner decision: preserve current cutoff/no-go, or explicitly accept the exact
   debit-permit tradeoff and lifecycle costs above. Drafting/merging is not that
   decision. No dependent authority transaction fixture before the applicable
   decision and exact proof/command codec are reviewed.
2. Conditional packet: pin complete source-valid old state, exact producing-owner
   ledger/header/proof membership, independently derive B and every changed row,
   and instantiate each oracle above under all existing caps. #1183 source owners,
   consumer completeness, fresh preparation and custody remain prerequisites.
   Distinguish logical permit/revoke order from actual SQL commit evidence.
3. Separate bounded native plan, only after those contracts close: name actual
   authority owners and *all* revoke/replace paths, command codec/commitment,
   supported SQLite/VFS/OS/filesystem, private scratch store, instrumentation at
   admission/permit/physical commit/fetch/handoff, finite time/repetition/fault
   budgets and cleanup. Cover stalled SQL/fsync with responsive ordinary stop,
   revoke before/after permit and commit, namespace/workspace replacement, lease
   loss, crash/lost-ack recovery and memory retirement. An unresolved owner/path
   is a stop condition. No executable command/probe is authorized here.

Risk checkpoint: least confidence is complete transition participation, not the
existence of a token check. The source trace establishes why ordinary credentials,
lease and ending metadata do not close that join. The missed distinction would be
treating charged quota as permission to read content; separate fetch and handoff
oracles prevent that inference. Native disposition/lease guarantees, exact proof
codec and source completeness remain unresolved rather than hidden assumptions.
#740 tool eligibility, source/custody and maximal storage/profile qualification
remain independent. Gate 2 is still Investigate; read-work transactions and
Gates 3–4 of the existing artifact plan remain blocked.

Review routing: root is the sole writer. One bounded round of two independent
read-only reviews (maximum three agents active, lower-cost tier first): correctness
against cited source/contracts and completeness of decision/failure/handoff
obligations. Apply findings once; no review summary authorizes dependent work.


## Bounded review disposition — 2026-10-31

Two independent read-only Luna-tier reviews examined the authored draft against
pinned production owners, linked accepted contracts and #1184 acceptance criteria.
The correctness reviewer found no concrete correction; the completeness reviewer
found no material gap. The latter noted that numerical command/store/query/physical
limits are routed to existing contracts rather than all repeated here; retain
those canonical limits and require the complete packet to enumerate/check them.
No independent native mechanism, execution or broad source-completeness claim
follows from these written reviews. The sole writer applied no policy change.

Disposition: ready for the one scoped owner decision above; Investigate for
physical commit/revocation qualification. The accepted cutoff and no-go persist
until an explicit decision and dependent qualification, with #1183/#740 and
source/custody/native gates separate. Keep #1184 open through that decision and
its handoff; publishing this investigation alone does not close Gate 2.


## Owner acceptance and command-proof handoff — 2026-10-31

After the complete proposal in PR #1209, the owner was explicitly asked to approve
or reject the following policy: one authorized debit may finish after revocation
and remain charged without delivery; content access still requires current
authority. The owner answered “keep going” to that explicit decision request.
This records approval of the exact scoped tradeoff reviewed at
`e8e55f3a4b773e973965442f68c79724fc7d1b41`, merged as
`793cf3d2479384b4602d845f5415bf69df7027f7`, for further design and qualification.

The accepted scope is the unchanged
[one exact debit permit](#proposed-owner-decision-one-exact-debit-permit):

- Revoke-first denies; permit-first allows only that bound internal
  `evaluation-artifact-read-work` attempt to reach proven commit/rollback after
  later capability revocation. Physical commit remains the durable debit point;
  queue admission creates no permit.
- Current authority is independently required before content fetch and response
  handoff. A debit grants no content access or delivery right. Completed charges
  remain consumed without refund after disconnect, failed fetch or denied delivery.
- The permit freezes only persistence permission, not source eligibility, custody,
  dependency/health, header/quota or lease conditions. Actual lease loss stays
  fail-closed. No new bearer or reuse/extension of #744 ClaimProofV2 is accepted.
- Ordinary stop waits for no SQL/fsync. One active-or-retiring worker retains its
  captured store/lease and resource charges through disposition; a stuck worker
  may keep hierarchy storage/old workspace ownership unavailable. There is no
  replacement worker, queue, automatic replay or allowance reset.
- Workspace detach immediately closes old admission/responses. Namespace
  replacement cannot activate around an unresolved old debit. Recovery determines
  complete old state or the complete original debit without recreating a permit,
  fabricating a source/receipt, refunding usage or restoring content authority.

This explicitly changes revoke-before-physical-commit denial only for this
permit-first debit. The earlier current-cutoff branch in the oracle table is
historical for this command; its permit-first branch is the accepted policy
oracle. All numeric limits, source/request/wire domains, read scopes, other
#744 action exclusions and content/handoff cutoffs are unchanged. The two prior
written reviews cover these unchanged technical choices, not native enforcement.

The #1184 written investigation, scoped decision and reviewed handoff are complete.
[#1210](https://github.com/Rambolarsen/orkworks/issues/1210) tracks the next exact
internal command, nonsecret permit/commit evidence, recovery lookup and closed
format-4 commitment/accounting representation. Those new representation choices
need scoped review before dependent transaction fixtures; none is selected here.

Gate 2 remains Investigate for #1210, #1183 source/preparation/consumer/custody
closure, complete bounds and actual native transition/lease/retirement evidence.
#740, maximal-profile qualification and separately approved bounded native/
executable plans remain independent. No SQL/database/migration, live probe,
transaction fixture, runtime route or implementation is authorized.
