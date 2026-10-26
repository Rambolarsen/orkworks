---
type: Design Proposal
title: Root bootstrap and subject-preparation producer contract
description: Proposed identity-first review drafts and exact UI-approved bootstrap, run membership and subject-preparation bindings without runtime authority transfer.
tags: [orkworks, hierarchy, bootstrap, preparation, approval]
status: draft
---

# Root bootstrap and subject-preparation producer contract

Tracking: [#741](https://github.com/Rambolarsen/orkworks/issues/741),
[#742](https://github.com/Rambolarsen/orkworks/issues/742),
[#744](https://github.com/Rambolarsen/orkworks/issues/744),
[#746](https://github.com/Rambolarsen/orkworks/issues/746); initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738).
Inspection base: `f5cae3f2`.
**Scoped draft/source flow accepted for further design/qualification; no executable
plan, eligible tool profile, native probe or runtime implementation.** See the
[owner acceptance](#owner-acceptance-and-handoff--2026-10-25).
This continues the [accepted two witness choices](2026-10-25-hierarchy-empty-output-profile-design.md#owner-acceptance-and-handoff--2026-10-25).
Their synthetic bootstrap binding stays UNVERIFIED; this text does not upgrade it.
#740 stays deferred/no-go until November.

## Outcome and short plan

A root subject must resolve its criteria, rubric, output contract and immutable
identity from the exact UI-approved #741 bootstrap and #742 run membership.
Neither a seed hash, later plan nor current session label supplies those bytes.
Close the producer's review/admission ordering, strict source-owned records and
consumer joins here; review them, then write source-valid byte/recovery fixtures
and a separately approved implementation/qualification plan. No runtime code is
part of this deliverable and no additional production quota defaults are chosen.

Read with [bootstrap v2](2026-10-04-agent-role-configuration-design.md#placement-versions-and-canonical-bytes),
[run creation and namespace admission](2026-10-04-orchestrator-preparation-design.md#planning-authority-and-capability-lifecycle),
[the durable admission fence](2026-10-04-orchestrator-preparation-design.md#exact-wire-counters-and-generation-identities),
[#744 identity mappings](2026-10-04-assignment-evaluation-design.md#identity-and-approved-snapshots),
[storage registry](2026-10-24-hierarchy-storage-contract-design.md#proposed-strict-storage-representation)
and [excluded authority classes](2026-10-24-hierarchy-authority-ordering-design.md#amendment-scope-and-exclusions).

## Chosen boundary and alternatives

Start with output-bearing root bootstrap schemaVersion 2. A child still requires
its approved plan/configuration, successful reservation, frozen task version and
unique attachment under #742; no root draft or source record can stand in for
those proofs. Ordinary sessions and legacy bootstrap v1 gain no reporting scope.

Use an identity-first immutable review draft, then one exact approval/admission.
The bootstrap includes runId and parentSessionId, while #742 assigns run identity
at admission. Hashing a template before those identities exist, or filling them
after approval, would approve different bytes. Reserve identities before review,
without making a run live, then consume the exact draft once. Reserve no worktree,
child, PTY, planning bearer or launch grant merely to show that draft.

Rejected alternatives are caller-chosen run IDs, post-approval digest repair,
a reporting capability that prepares its own subject, and separate commits that
publish a summary before its immutable source/membership. This proposal chooses
one outstanding draft per workspace rather than an unbounded draft history.
The new draft lifecycle, storage rows and finite draft bound need owner review.

## Producers and authority

Electron main initiates draft/review/approval/cancellation using existing
ORKWORKS_OPEN_PLAN_TOKEN authority. Token bytes remain in main/sidecar, never in
records, prompts, renderer or coding-tool environments. Renderer actions are
validated requests to main, not authority. No new bearer or generic bootstrap
upload endpoint is introduced by this logical contract; concrete IPC/HTTP routes
remain implementation-plan choices.

The #742 run-creation source owner assigns identities and owns the durable
admission fence. The #741 owner strictly validates and composes the whole v2
bootstrap, including applicable rules/skills, rendered bytes, capability/model/
permission evidence, repository and source-worktree descriptors, criteria/rubric
and output contract. The hierarchy store alone computes/writes source rows,
summary/ledger effects and manifests. Producers submit typed immutable intents,
never caller-authored row sets or SQL.

Run/draft creation, approval and preparation **are excluded from #744's accepted
claim-first exception**. No producer/UI ClaimProofV2 is fabricated for them.
Their own current-authority cutoff must hold through durable admission, while
ordinary stop/workspace revocation stays responsive. No qualified native mechanism
for that ordering is supplied here. A pre-SQL boolean, receipt or stored draft
cannot waive a later winning revocation. This remains an execution no-go until
separately resolved/qualified; this proposal does not broaden the claim policy.

## Identity reservation before review

An explicit UI prepare request serializes through #742's existing workspace
admission fence. Reserve the next runSequence durably before persisting a draft;
runId is exactly admissionEpoch + "-" + canonical decimal runSequence. A crash
may burn a sequence but never reuse it. Draft reservation is not active run
admission and cannot satisfy a reporting/run-capability lookup. In particular,
a missing run at/below high-water remains unavailable to reporting replay.

Also reserve fresh non-adoptable parentSessionId, rootAssignmentId and draftId
from sidecar-owned ID sources. These are metadata reservations only: no existing
ordinary/foreign session may be silently adopted and no process starts. The
future parent identity must later be uniquely attached under the #742 session
producer contract before any runtime credential can be issued. Creating a new
parent process retains its separate UI/startup/capability gates.

Use a nonsecret reviewContextId, a fresh opaque 64-character lowercase hex value,
to bind one trusted main/sidecar review presentation to the selected workspace
lifetime. It is not a credential or a second UI token. The sidecar keeps its
currentness in volatile authoritative UI context; it cannot be selected or
revived by a request body. Retain one bounded immutable canonical draft-byte
snapshot with that live review context; before approval compare its bytes to the
current stored draft and source intent, not only their digest strings. Main's
reviewed view binds those same bytes. Charge retained copies to the eventual
bounded review/worker memory accounting; they are not extra uncounted attachments.
Workspace switch, sidecar restart, token/review-context
revocation or draft cancellation invalidates it immediately. It must not become
current merely because stored bytes match after returning to the workspace.

A persisted draft is immutable. Editing any goal/bootstrap/ceiling/input byte,
changing eligibility evidence, or invalidating review context requires explicit
UI discard and a new draft/sequence/context followed by a new exact review.
A restart may show the old draft as historical/unavailable for approval; it
never automatically finishes it or issues a new context for its old approval.
This intentionally burns abandoned identity reservations and uses no timer-based
expiry, automatic acceptance or recovered bearer.

## Closed source records

Use #741 canonical serialization, ID/Digest bounds and exact #742 numeric/string
encodings. Reject unknown/duplicate properties, wrong versions/tags, invalid
UTF-8, noncanonical stored bytes, unsupported nested descriptors or mismatched
identities before persistence. Embedded bootstrap/run values are complete logical
objects, not JSON-quoted strings or uncounted attachments. Stored canonical BLOBs
must equal their exact canonical logical bytes.

### Run definition

For this root producer, close OrchestrationRunDefinitionV1 to exactly:

```text
{schemaVersion:1,runId,workspaceId,parentSessionId,
 repositoryBindingDigest,sourceWorktreeBindingDigest,goal,goalDigest,
 admissionEpoch,runSequence,bootstrapConfigurationDigest,
 maxParallelChildren,runDefinitionDigest}
```

goal is the exact user-reviewed UTF-8 text; goalDigest is raw SHA-256 of those
bytes. The whole definition is at most 16 KiB. maxParallelChildren is integer
1..16. admissionEpoch is the existing 64-character lowercase hex namespace;
runSequence is its existing positive decimal string through 9223372036854775807.
Use the existing orkworks.orchestration-run.v1 + LF domain over the definition
excluding only runDefinitionDigest. This closes field names for the existing
logical binding, not a new run source domain or mutable lifecycle schema.

### RootPreparationDraftV1

Exactly:

```text
{schemaVersion:1,draftId,workspaceId,runId,parentSessionId,rootAssignmentId,
 sidecarGeneration,reviewContextId,bootstrap,runDefinition,draftDigest}
```

bootstrap is the complete OutputOrchestratorBootstrapConfiguration v2, at most
1 MiB inclusive of all snapshots/rendered context. runDefinition is the exact
object above. sidecarGeneration and reviewContextId are nonsecret 64-character
hex identities. draftDigest uses orkworks.root-preparation-draft.v1 + LF over
the whole canonical object excluding only draftDigest. Every repeated identity
must agree with the embedded objects. Unknown bootstrap versions are not upgraded.

The complete draft has a proposed 1 MiB + 32 KiB (1,081,344-byte) limit; nested
bootstrap/run limits and any lower verified adapter/source limits also apply.
This extra envelope allowance is not a source/payload quota increase. One pending
draft row per workspace is allowed; another prepare conflicts until explicit
cancel/discard or successful admission consumes it. There is no silent replacement.

### Exact approval and retained source

RootPreparationApproveRequestV1 is exactly:

```text
{schemaVersion:1,action:"orkworks.root-preparation.approve.v1",
 workspaceId,runId,draftId,draftDigest,reviewContextId,approvalId}
```

approvalId is a main-originated ID used as the idempotency key for this exact
workspace/admissionEpoch/run tuple, not a caller-chosen authority grant. Authenticate
current UI scope before any lookup and validate every request field. requestDigest
uses orkworks.root-preparation-request.v1 + LF over the full canonical request.
No token or mutable run state enters that digest.

The retained source row has exactly:

```text
{schemaVersion:1,bootstrap,runDefinition,
 approval:{approvalId,draftId,draftDigest,reviewContextId,sidecarGeneration,
           requestDigest},
 receipt:RootPreparationReceiptV1,lastCommit:CommitRef}
```

RootPreparationReceiptV1 is exactly schemaVersion:1, workspaceId, runId,
rootAssignmentIdentity, bootstrapConfigurationDigest, runDefinitionDigest,
draftDigest, approvalId, requestDigest and commit:CommitRef. Identity is the strict
#744 root AssignmentIdentity derived below. The receipt contains no bearer,
execution grant, parent runtime generation or promise that a parent started.

The source retains complete original bootstrap/run bytes, not just a lookup hash.
Reconstruct the original draft from these bytes and approval metadata, rehash its
draftDigest, then reconstruct the approval request and verify requestDigest.
No separate raw request log or mutable latest-configuration alias is retained.

## Storage membership and atomic effects

Propose two closed additions to format 3's logical registry:

| Table/key | Exact value / ownership |
| --- | --- |
| root_drafts / workspaceId | `{schemaVersion:1,draft:RootPreparationDraftV1,lastCommit}`; at most one per workspace, owned by its metadata/lease |
| root_preparations / `{workspaceId,runId}` | The retained source row above; one immutable admitted root preparation per run |

Commit logical decoded bootstrap/run objects in the existing row.v3 envelope.
Exclude only root_drafts.lastCommit, root_preparations.lastCommit and
root_preparations.receipt.commit at their named paths; keep every other field,
including approval/context identities and source bytes. Unknown tables/versions
remain inadmissible; no implicit migration or mixed-codec read is introduced.
A proposed complete retained source-row bound is also 1,081,344 bytes, including
receipt/approval/links. Each nested descriptor/receipt obeys its smaller bound;
RootPreparationReceiptV1 is at most 4 KiB. New metadata never escapes command/main/
physical/retention accounting through a content-deduplication assumption.

Add root-draft-prepare and root-draft-cancel as exact manifest command kinds for
the corresponding insert/delete plus required control/quota effects. The accepted
subject-preparation kind consumes the draft and inserts the immutable retained
source together with root subject state. This extends the prior synthetic-only
preparation shape; it does not edit that witness's bytes or claim its seed valid.

Atomic subject-preparation must include:

1. Revalidated complete bootstrap/run source, original UI approval receipt and
   exact membership key; consume exactly the matching workspace draft.
2. The #742 source-owned initial admitted run-state/membership effects, including
   an unambiguous reserved parent identity, before any runtime/grant delivery.
   The full mutable run-state and parent-attachment codecs remain a separate
   required producer contract: no generic object/extra column can stand in for
   them, and this command stays ineligible until their exact effects are closed.
3. A new #744 root summary_state with subjectRevision "1", derived from the
   approved criteria/rubric with no result. The capacity-fitting initialization
   branch supplies its checkpoint and initial summaryRevision "1"; the existing
   never-initialized/unavailable branch does not fabricate a row/checkpoint when
   capacity is absent. Use that existing summary-capacity policy rather than inventing
   empty criteria, a pass, or fresh lineage on later reads. The chosen initial
   capacity branch and its complete rows must be included in byte fixtures.
4. Every required source/receipt/quota/dependency/lineage effect, all changed
   existing owner rows and newly initialized rows, draft deletion, header and
   new commit proof. No source partial-commit or negative deletion-byte credit.

Unlike the isolated witness, a live workspace can already have runs and quota
rows. Preserve shared workspace/run accounting; initialize only provably new
owner/resource rows at zero. Never reset an existing shared row to zero, recreate
missing historical ledger state, or reuse a subject whose lineage is absent.
The exact complete changed set and source-specific aggregate limits must close
before execution. A separate draft/body bound does not settle finite production
run/workspace source/payload or source-history caps; those remain owner decisions.

Draft preparation follows the durable admission-fence reservation, not one atomic
transaction spanning the external fence file and SQLite. If the draft write fails,
that sequence is burned. Recovery reconciles the exact current fence and draft:
no draft/source at a burned sequence supplies a run or right to reconstruct one.
A present pending draft permits only explicit UI inspection/discard while context
is stale. An admitted root source must join its exact fence epoch/sequence and
complete committed source/run/subject proof before any capability is considered.

## Exact consumer bindings

Before subject preparation, and again before every runtime/reporting operation
that needs current source authority, the source owner checks:

| Binding | Required check |
| --- | --- |
| Workspace/run/parent/root/bootstrap identity | Exact equality between draft, bootstrap v2, run definition, retained source key/receipt and sidecar-owned reservation; no label/path/session-name inference |
| Admission | runId equals the bound epoch/sequence; the sequence was reserved under the current exact workspace fence; currentness is checked independently of the high-water magnitude |
| Repository/source worktree | Strict complete #745 descriptors in bootstrap; rehash under their existing domains; equal repository/worktree digests in run definition; fresh qualified identity revalidation |
| Goal | Exact reviewed goal bytes hash to both run and bootstrap goalDigest; no normalized/summarized replacement |
| Bootstrap digest | Recompute over the entire v2 bootstrap excluding only bootstrapConfigurationDigest, using orkworks.orchestrator-bootstrap.v2 + LF; equal run/source/receipt value |
| Run definition digest | Recompute full canonical immutable definition under its unchanged domain; compare every receipt/reference, not only runId |
| Rules/skills/instructions/model/permissions/outputContract | Complete owner validation/composition, source content and effective settings/capability evidence; drift requires a new draft/approval, no silently widened fallback |
| Root identity | Copy kind:root, workspaceId, runId, rootAssignmentId, parentSessionId, bootstrapId and bootstrapConfigurationDigest from this one validated source; never future plan/task fields |
| Criteria snapshot | Exact bootstrap.assignment.criteria, sorted by stable ID for #744's criteria-snapshot.v1 domain, with 1..32 entries and at least one required; configuration keeps its original meaningful order |
| Rubric snapshot | Exact bootstrap.rubric with ID/version/evaluatorRole and 1..16 dimensions, hashed under #744's rubric-snapshot.v1 domain |
| Outputs | Exact inline v2 outputContract and owner-validated bounds; text, reports or a later ParentPlanBinding cannot add declarations |
| Parent/runtime membership | Match #742's unique reserved/attached parent and current lifetime proofs. Immutable preparation is historical identity, never proof of current runtime or authority |

Later plans bind the original run/bootstrap/source descriptors through unchanged
ParentPlanBinding fields. They do not refresh root skills, permissions, criteria,
rubric, outputs or source identity. Changed bootstrap requires the existing fenced
new-run path. Root preparation gives no child reservation/taskVersion and cannot
qualify root telemetry from the child evidence protocol.

## Retry, cancellation, restart and retention

After current authentication/scope/context validation, locate the exact retained
root_preparations key before trying to consume a pending draft. Reconstruct and
compare the full approval tuple/request bytes. An identical authorized retry
returns the original receipt without new source, summary, quota, sequence, proof
or process. Changed bytes with the same approval tuple conflict; a different
approvalId cannot create a second root for an already admitted run. A missing
source never means recreate it from the old UI request or current high-water.

Approval may be retried only with still-current authority and its originally
validated review context. Once context is revoked, no receipt lookup reactivates
it: use the existing separately authorized read/history path, not mutation retry.
Response delivery retains its own current-authority cutoff. A committed source
can remain retained even if a later UI response cannot be delivered; no token is
restored to make delivery succeed.

UI draft cancel deletes only the exact draft after checking draftId/draftDigest,
revokes review context immediately and leaves high-water/sequence burned. An old
cancel cannot delete a replacement draft or an admitted root. Cancel/approval
must have one proven winner under their excluded-action authority/commit ordering;
no already-started SQL claim exception is inherited. Stuck/ambiguous I/O fences
admission and preserves ordinary stop responsiveness; no replacement worker or
blind replay around it. This ordering remains native-unqualified/no-go.

Retain admitted bootstrap/run/approval/source bytes and proofs with the exact
root subject while any #741/#742/#744/#746 plan, result, evidence or consumer needs
them. No partial snapshot eviction, namespace rotation, run completion or token
renewal changes identity or refunds protected source storage. Old-epoch records
remain historical under existing rules. Root source deletion needs the existing
closed-subject, incoming-dependency, observed subjectRevision and shared-custody
protocol plus the run owner's own deletion/fence rules; no new automatic cleanup
or tombstone list is selected. Draft discard cannot purge admitted history.

## Written acceptance oracles

These are source-contract cases, not executed runtime or source-valid bootstrap
fixtures. No partial projection or made-up verified capability snapshot is offered
as a positive admitted configuration.

| Case | Required result |
| --- | --- |
| UI prepares, then abandons before approval | One bounded non-executable draft; no run/grant/process; sequence not reusable |
| Fence reserved but draft commit fails/crashes | Burn sequence, no inferred draft/run |
| User changes one bootstrap byte after review | Draft/request digest conflict or new draft/sequence and exact approval |
| Goal text differs only by normalization | Different exact goal digest; no equivalence substitution |
| UI approval repeats after response loss with unchanged current context | Original exact receipt, no second preparation, source charge or parent |
| Missing source at/below high-water | Unavailable/retired, never reconstructed from receipt request |
| Existing run/workspace quota rows contain prior usage | Checked increment/preservation, never zero reset |
| Historical root source exists but summary/preparation proof is missing | Unavailable; do not reinitialize lineage or supply a report grant |
| Bootstrap criteria reorder without configuration digest change | Reject byte/digest drift even if sorted #744 snapshot digest matches |
| Root identity fields agree but rubric/output/source bytes differ | Reject source/digest mismatch, no latest-source substitution |
| Eligibility changes while draft is reviewed | New draft/review; unverified #740 evidence still blocks admission |
| Draft cancels or review context is revoked before approval commit | Deny under its own ordering; never a #744 claim-first survivor |
| Sidecar/workspace lifetime changes, then old approval arrives | Old context unavailable; explicit discard/new sequence/review, no automatic resume |
| Old draft cancellation arrives after replacement/admission | No-op/conflict under exact identity; never deletes the new owner/source |
| Preparation commits but parent has not attached/started | Retained source identity only; no running parent, bearer or launch |
| Exact run resumes with changed bootstrap | Fence old scope and require new run; retain original source history |
| Caller tries a child subject or root bootstrap v1 | Reject unsupported scope/version; no inferred reservation or schema upgrade |
| Record/command/aggregate/source or summary-proof capacity cannot fit | Reject whole command; preserve old state, no uncharged attachment or partial source |

## Bounded review disposition

One read-only review checked authority/identity ordering and another checked
record reconstruction, retry/cancel/restart, membership and producer coverage.
Neither found a material issue in this scoped proposal. Subsequent author
clarifications require exact frozen-draft byte comparison and distinguish normal
summary initialization from its existing capacity/unavailable branch. No native
or source-valid bootstrap fixture was run, and no post-edit independent technical
rereview is claimed. Runtime gates remain explicit rather than assumed complete.

## Review handoff and remaining gates

The owner accepted the scoped identity-first draft/source flow, associated exact
records/registry and finite record envelopes for further design/qualification,
as recorded below. This settles the named flow choices, not every broader
component contract or an executable preparation profile. Existing #744 claim
policy, source/report schemas, receipt pools and fixture bytes are unchanged.
The record envelopes do not establish global production source/payload defaults
or native resource guarantees.

Next, supply full #741 source-valid bootstrap/run/receipt/row/transition
byte fixtures and close the #742 initial-state/parent-attachment codec/effects,
finite production aggregate storage/custody limits and excluded-action authority
ordering. Then update the previously synthetic seed with separately reviewed,
properly bound evidence; never relabel its old hash as verified. #740 eligibility,
repository/platform proof, shared custody, native resource/query/durability and
finalization-versus-writable-ownership qualification, and separately approved
executable plans remain gates. No runtime code, database, migration or probe is
authorized; keep all four component issues open.

## Owner acceptance and handoff — 2026-10-25

After PR #1066 merged and its draft/source flow was presented for acceptance,
the owner answered “go ahead”. Accepted for further design and qualification
against reviewed source [PR #1066](https://github.com/Rambolarsen/orkworks/pull/1066),
merged as `41f52bd7ad7f7a911ab6071ad97a95589140083f` (reviewed head
`32a7b800da727798276a9ac0c807f6147f127271`):

- Identity-first non-executable review drafts with one pending draft per workspace.
  Sidecar-owned run/parent/root identities precede exact review; abandoned or
  failed reservations burn sequences and cannot be reused or inferred as runs.
- The exact draft/approval/request/receipt and retained bootstrap/run membership
  records, root_drafts/root_preparations registry additions, draft prepare/cancel
  kinds and root-only atomic subject-preparation/source/summary/ledger obligations.
  The reviewed draft/source envelopes are 1,081,344 bytes each, with a 4 KiB
  receipt bound and all smaller nested bounds preserved. These are record bounds,
  not new aggregate production quotas or executable storage guarantees.
- A non-revivable review context and frozen canonical byte comparison in addition
  to digest checks. Context invalidation, restart/workspace change, or changed
  reviewed inputs requires explicit discard, a fresh draft/sequence/context and
  new exact review. No recovered bearer, automatic acceptance or silent renewal.

The accepted usability cost includes one draft slot, burned abandoned identities
and repeating review after context invalidation. Current authentication remains
mandatory for retries and response delivery; historical receipts confer no new
run/report/launch authority. Child reservation, frozen version and unique attachment
proofs are unchanged. Preparation stays excluded from #744's claim-first exception;
this acceptance supplies no native commit/revocation ordering mechanism.

This record changes no reviewed technical definition, canonical field/domain,
record bound, fixture/checker byte, #744 receipt quota or claim/lease policy. It
accepts the named flow and records, not the whole broader component/storage
contract or any eligible tool/source profile. The earlier synthetic bootstrap
snapshot binding remains UNVERIFIED.

Source-valid bootstrap/run/request/receipt/row/transition byte fixtures, initial
run-state/parent-attachment codecs and complete effects, finite production
aggregate storage/custody limits, excluded-action authority ordering, #740 evidence,
repository/platform and native resource/query/durability/finalization qualification,
shared custody and separately approved executable plans remain gates. No database,
migration, model/native probe, runtime implementation or launch eligibility is
approved. #740 stays deferred/no-go until November; keep #741/#742/#744/#746 open.

Next bounded design task: full source-valid bootstrap/run/approval/retained-source
and transition byte fixtures for the accepted root flow. Close any missing source
codec before labeling a fixture source-valid; review new schema/default choices
separately before dependent implementation.


## Component-byte progress — 2026-10-26

The [component fixtures and source-closure audit](../../validation/root-preparation-components.md)
pin the closed run-definition and approval-request bytes and their boundary
rejections. These are not full bootstrap, retained-source or transition fixtures.
Inspection also identified root assignment, executable-identity and provenance
codec gaps in #741, alongside the already open #742 initial-state/attachment
codecs. Close and review those source definitions before constructing complete
source-valid fixtures; repeated synthetic digests cannot supply missing objects.
No reviewed source schema, previous catalog, quota, authority or eligibility
changes. All acceptance and runtime gates above remain open.
