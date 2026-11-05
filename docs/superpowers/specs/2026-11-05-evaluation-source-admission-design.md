---
type: Design
title: Evaluation source-to-runtime admission
description: Proposed private source admission and mutation participation for exact child-reviewer evaluation bindings, without production activation.
tags: [taskmaster, hierarchy, evaluation, authority, sources]
status: draft
workflow_status: owner-review-pending
---

# Evaluation source-to-runtime admission

## Outcome and scope

Tracked by [#1397](https://github.com/Rambolarsen/orkworks/issues/1397), a bounded
prerequisite of [#1176](https://github.com/Rambolarsen/orkworks/issues/1176).
The proposed interface connects original child preparation, an actually attached
ordinary runtime and the [accepted private evaluation authority](2026-11-04-evaluation-publication-authority-design.md).
It does not promote a byte validator, session ID or stored status into authority.

The author proposes one source-admission module inside the private publication
owner, sharing its short serialization latch. It owns admission freshness and
participation by relevant source writers; the qualified source store continues
to own durable evidence. No latch covers storage, native operations or transport.
This document defines that interface and its refusal conditions. It does not
supply the missing source publisher or a mechanism for cross-store atomicity.

The accepted codecs, identities, quotas, claim-first evaluation exception,
read-work debit policy, discovery limits and coding-tool trust standard remain
unchanged. No new wire command, durable row, proof domain, bearer or numeric
limit is proposed. Same-user instruction adherence is the practical harness
standard; admission is not OS isolation or process-origin attestation.

This proposal does not authorize production constructors/attachment, AppState
initialization, live registration, database/SQL or migration work, hierarchy
storage activation, reporting transport, model/native probes or a verified
profile. [#740](https://github.com/Rambolarsen/orkworks/issues/740) remains closed
research. #1176 and full source/physical/native qualification remain open.

## Inspected source and existing contracts

Inspection base: `464662c02095a4474a53426a8d6805678d1f248d` (PR #1396).

| Existing module / contract | Established behavior and remaining obligation |
| --- | --- |
| [child_sources.rs](../../../crates/orkworksd/src/taskmaster/child_sources.rs), validate_unverified_child_sources_v1 | Validates supplied original source/configuration and independence joins. Its result is explicitly unverified-source-bytes-only, with no I/O or admission. It cannot construct an admitted source witness |
| [publication_authority.rs](../../../crates/orkworksd/src/taskmaster/publication_authority.rs), BindingData / register_source_binding | The dormant core stores opaque source bytes and changes UNISSUED to OPEN under its latch. Factories are test-only. This is an ordering mechanism, not a source-evidence interface |
| [lifecycle.rs](../../../crates/orkworksd/src/taskmaster/publication_authority/lifecycle.rs) and [session_runtime.rs](../../../crates/orkworksd/src/runtime/session_runtime.rs) | PR #1396 routes exact close/cancellation with all production carriers None. The ordinary RuntimeIdentity has runtime_instance_id plus numeric run_generation; it is not the accepted 64-hex AuthorityGeneration |
| [Accepted child command effects](2026-10-31-child-review-source-preparation-design.md#durable-command-effect-map--2026-11-03) | Original attach/preparation commits lineage, source, summary and five assignment zero rows together. The ten owner-run/workspace peers already exist; no establish-on-missing or post-commit index registration |
| [Runtime and ordinary-session controls](2026-11-02-child-launch-control-design.md#runtime-attachment-history) | RuntimeRef and immutable actual runtime history are separate from an attempted spawn. ChildSessionState has its own producerRevision. Ordinary resume preserves original assignment identity and requires fresh runtime authority |
| [Accepted membership](2026-11-03-child-consumer-membership-design.md#coherent-discovery-and-publication) and [current audit](../../validation/evaluation-artifact-source-closure.md#current-source-owner-and-membership-audit--2026-11-04) | Complete header-bound metadata-only source/proof/consumer discovery is required. Existing ordinary search indexes and the test-only partial hierarchy catalog do not qualify it |
| [Assignment evaluation](2026-10-04-assignment-evaluation-design.md#reviewer-eligibility-and-disagreement) | A child evaluation capability binds exact reviewer/configuration/current generation, target result and declared output. It is distinct from parent launch, research, result and user authority |

These are bounded source findings. They do not establish the absence of an
external owner, eligibility of a platform/harness or completeness of all writers.
No native executable, hosted model, raw transcript or external credential was
inspected for this design.

## Exact evidence, resolved by the source owner

Use P for the complete producing reviewer AssignmentIdentity and T for its exact
worker target. The source owner derives the following at one coherent eligible
store header H; a caller supplies only its already authenticated operation's
closed identity, not arbitrary records, keys, affected lists or eligibility flags.

| Join | Required original and current evidence |
| --- | --- |
| Workspace/run/root | Exact repository/workspace identity, retained root preparation and run definition/bootstrap, selected namespace/storage/admission epochs, original owning CommitRefs and current run/parent controls |
| Reviewer P | Original exact approved v3 configuration and plan/input/approval, successful reservation with frozen assignmentTaskVersion, independent allocation and unique attachment/child-assignment-source. Compare live task versions only where their owning transition requires it; never rewrite P from the latest counter |
| Target T and frozen review input | Accepted target result revision/digest, original result/configuration/evidence and criteria/rubric snapshots, exact reviewer input snapshot and evaluation declaration. Derive contributor exclusion and approved read-without-write access from source-owned allocations/effective scopes; no parent assertion or prompt is proof |
| Preparation and custody | Original ordinary lineage and complete attachment/preparation disposition; this assignment's five actual zero-initialized resource rows and their original owning links, ten reconciled run/workspace peers, source/control pools, summary initialization branch and complete direct proofs/membership. Current zero counters alone cannot prove initial preparation |
| Runtime | Exact current child RuntimeRef and immutable successful RuntimeAttachment, matching nonterminal ChildSessionState/producerRevision and actual ordinary runtime owner. Preserve original launch history on resume; an attempted generation, PID, native session label or copied ended/running status is insufficient |
| Scope and health | Actual current source/store/native health, selected scope, retained physical lease ownership and current authenticated AuthorityGeneration. Advisory lease existence or a stale handle is insufficient. No source write gains the evaluation claim-first exception |

Every required record has its accepted full identity/key, digest and owning
proof; retain exact source dependencies, not just one aggregate hash. A bounded
metadata-only lookup must prove completeness/absence where needed under the
accepted 4,096-observation / 16 MiB discovery profile. Content access and read-work
charges keep their separate accepted authority rules. No uncharged artifact
copy/hash/materialization is added to resolve admission.

This table is a required read set, not a new serialized bundle or an assertion
that all maxima fit. The actual source owner must derive the complete applicable
set, including supported historical dependencies and read-only peers. Unsupported,
missing, corrupt, extra or ambiguous evidence denies admission.

## Proposed private interface

Names here describe internal roles, not committed Rust types or wire codecs.
Only the qualified source owner can construct source evidence. Keep the byte
validator's existing result type and classification unchanged.

| Internal operation | Owner and contract |
| --- | --- |
| Create unissued runtime carrier | Private runtime/source admission owner, after proven original preparation and before startup future/task creation. Freeze P/T/configuration/declaration/target identities and the exact intended runtime association. Attach its close handle to that generation; this grants no capability and proves no successful spawn |
| Capture admission ticket | Source-admission module, after the actual runtime/ordinary bridge outcome is proven. Under the publication latch capture the exact current source fence, selected scope and the same immutable unissued binding. Reject a closed scope or in-flight/unknown source transition |
| Resolve admission evidence | Qualified source owner, outside the latch, reads H and derives the complete evidence/read set above, revalidating it through its qualified storage mechanism. Supplied source bytes may be checked by existing pure validators but are not the evidence's provenance |
| Commit registration | Same source-admission module, under the publication latch. Compare the retained ticket/fence identity, selected scope, complete immutable binding and current source/runtime authority, then transition that exact UNISSUED binding to OPEN once. Do no I/O, sessions/workspace lock acquisition or callbacks under this latch |
| Begin source transition / publish disposition | Actual source writer participates through the same module before changing any fact admission depends on. Invalidate pending admission tickets, close affected registered bindings/scopes and hold new admission unavailable until the source owner's complete disposition is proven. This authorizes no durable write and is not a claim or debit permit |

The unissued binding contains expected immutable semantic identities, not a
fabricated RuntimeAttachment proof. Its expected runtime key/body digest can be
computed from the reserved exact generation; only the actual native/ordinary
producer can prove the corresponding successful attachment later. Original
proofs remain retained by their source owner and are resolved at admission.
A matching expectation without that proven original outcome never opens a binding.

The source-admission module holds live typed dependencies and opaque owner
references; it does not persist generic caller-supplied source bytes. The future
production adapter must replace the core's synthetic test construction with
this private construction path. No conversion from ValidatedUnverifiedChildSources,
ordinary metadata, workflow-report token or a deserialized caller bundle can
produce admitted evidence or an OPEN binding.

## Accountable owners and typed disposition

The author's proposed code location is
`taskmaster::publication_authority::source_admission`, private beside lifecycle.
It governs admission under the existing owner's latch; it is not a new durable
store. Its only durable-operation caller is the future qualified hierarchy
source publisher. That publisher owns the closed child source commands and the
accepted event-specific control/runtime bridge publishers. The ordinary runtime
owner supplies actual runtime observations and immediate exact close; it does
not directly assert hierarchy disposition or edit source rows. Ordinary-session
persistence remains with its current owner until the separate cross-owner
execution design proves their coherent disposition.

These are explicit proposed responsibilities, not claims that these publisher
entrypoints exist at the inspection base. No general AppState/session/HTTP
caller may finish a source transition. The source-publisher interface must be
sealed to its typed command owners: original preparation/attach, exact approval,
reservation/allocation, target/configuration/dependency publication, and the
separately defined run/runtime/recovery events. The full actual writer inventory
and the unimplemented event-command representations remain prerequisites of that
publisher's execution design, not work delegated to a generic callback.

A begin operation returns one non-Clone, owner-created SourceTransition holding
its exact event identity, original source/read-set context and retained fence
reference. It is no durable-write permit. Its completion accepts only an opaque,
source-owner-validated disposition for that same transition: a proven complete
committed outcome with original owning proof/read-set identity, proven rollback,
or unknown. Qualification and any proof I/O occur outside the publication latch.
A boolean, current row digest, callback success, timeout or ordinary status cannot
construct the validated disposition. Dropping or mismatching the transition
leaves admission unavailable; it cannot imply rollback or publish a fresh fence.
No serialized SourceTransition or new CommitRef/proof codec is introduced here.

The existing storage worker/position remains the sole place for a source
mutation; this proposal adds no worker, retry loop or unbounded transition queue.
Immediate lifecycle/source invalidation may close admission while that worker
is occupied. A later invalidation supersedes the old transition's right to make
admission available: its eventual settlement may record only its exact durable
outcome, never clear the newer fence. Overlapping/unknown events require the
source owner's complete supported reconciliation before any fresh fence, not
one completion callback per event. If the existing bounded path cannot retain
that necessary disposition, keep hierarchy admission unavailable and ordinary
stop responsive. The physical mechanism and its exact resource proof remain
part of the named source-publisher execution design.

## Admission freshness and source-writer participation

The author proposes one in-memory admission fence reference owned under the
existing publication latch. An admission ticket retains that exact reference.
Beginning a participating source transition closes the current fence terminally
and makes new admission unavailable. After a proven disposition, the owner may
create a fresh fence for new complete admissions. Compare retained references,
not recyclable integers or an unretained address. This is no durable epoch,
namespace reset, authority generation, quota reset or permission token.

All pending tickets captured before a participating transition fail even when
that transition eventually rolls back. Existing live bindings close only when
the writer's source-owned change affects their exact dependencies, or the owning
run/namespace/workspace/health scope closes. The source owner derives that set
from its complete supported live registry and accepted membership semantics;
callers cannot omit a binding with an affected-list argument. Unknown affected
membership fences the scope and denies the transition's hierarchy admission.
No claim that the physical membership/profile is qualified follows from this
in-memory rule.

A transition cannot issue a new admission ticket while its disposition is
in-flight or unknown. After a proven outcome, a fresh ticket requires the whole
current source check again. Creating a new fence does not reopen an old binding,
restore a grant or automatically rebind a live process. Existing worker/queue
and source limits apply; this proposes no additional queue or retiring worker.
The adapter must demonstrate bounded retained dependency references and ticket
lifetimes within the owning profile before activation; missing memory/accounting
proof keeps it unavailable rather than introducing a new numerical allowance.

| Source event | Required participation before its effects |
| --- | --- |
| Original preparation, approval, reservation, allocation or attachment change | Invalidate pending tickets; close bindings whose exact source/configuration/admission facts change, with scope closure on unknown membership. Prove original durable outcome separately |
| Target result, configuration, frozen evidence, review access or contributor-scope change | Resolve complete affected reviewer dependencies, close their current grants before the change, and preserve historical artifact/receipt/custody state. Do not automatically recapture a newer result into the old binding |
| Child runtime start/resume/ending or replacement | Close the old exact runtime before exchange; retain historical assignment/spawn identity. Publish the actual bridge/runtime outcome, then capture a fresh admission ticket. Ending/cancellation closes even a still-unissued carrier |
| Run cancellation/final completion, namespace/workspace replacement, source health or lease loss | Close exact enclosing scopes; unknown physical disposition cannot be repaired by making a fresh fence. Ordinary stop remains independent |
| Recovery/restart | Recover only supported original durable outcomes. Fresh sidecar/runtime authority and full admission are separate; preserved source rows, receipts or a successful exact retry restore no capability |

The source fence does not substitute for current source authority at physical
commit. The five child source commands and ordinary bridge/control mutations
remain excluded from the evaluation claim-first exception. Their actual durable
cutoff/atomicity and native recovery mechanisms must still be qualified. In
particular, ordinary create_session followed by unrelated hierarchy writes is
not a complete attachment/preparation disposition. This design does not select
a multi-database transaction, journal format, outbox or repair mechanism.

## Ordering, failure and recovery

1. Prove complete original child preparation before a separately authorized
   spawn. Reserve the exact runtime association and attach the UNISSUED carrier
   before creating its startup future. Cancellation before first polling closes it.
2. The native/ordinary source producer proves successful attachment and the
   coherent current bridge through its own qualified source transition. A failed
   or unknown outcome leaves the carrier unissued/closed; ordinary cleanup runs.
3. Capture a fresh admission ticket after that outcome, resolve evidence at H
   outside the latch, and commit only if the same fence/scope/binding remains
   current. A subsequent relevant source transition closes the binding before
   changing its dependent facts.
4. Source mutation/stop and registration share one cutoff: registration first
   can open only until the subsequent close; close/fence replacement first makes
   registration fail. Never acquire workspace, sessions or prompt guards while
   holding the publication latch. Use the accepted projection-before-publication
   order where projection admission participates; never hold a core guard or
   publication latch through SQL/fsync/native work.
5. A denied or interrupted admission returns no grant and creates no claim,
   receipt, ledger row or durable source event. The source owner preserves any
   separately committed preparation/runtime outcome. No automatic retry,
   backfill, zero initialization, generation alias or old-authority restoration.

Actual runtime ownership is an owner-retained association between the ordinary
RuntimeIdentity and accepted RuntimeRef/AuthorityGeneration. Their representations
are distinct. Do not cast run_generation into a 64-hex launch generation, reuse a
native session ID, or invent a second sidecar generation disconnected from the
accepted authenticated reporting owner. The qualified native/sidecar producer
must establish the association and freshness; this design creates no such proof.

An evaluation already claimed before later close retains only its accepted exact
settlement right. It cannot register a replacement, authorize a source transition,
fetch content or commit a new response. Unknown disposition keeps the original
worker/lease retention rules; dropping a task or record proves no rollback.

## Verification and finite handoff

| Case | Required oracle |
| --- | --- |
| Valid copied bytes, zero-looking counters or stored runtime status without original proof | No evidence constructor/registration; no capability or side effect |
| Source transition begins after capture but before final registration | Old retained fence ticket is rejected even after rollback; no stale OPEN |
| Registration wins before relevant source transition | Exact affected binding closes before that transition's effects; unaffected bindings are not silently rebound |
| End/cancel before actual outcome or registration; resume/replacement races | Exact pending carrier is terminally closed; the new runtime has a different carrier and requires full independent admission |
| Successful spawn without complete bridge/preparation proof; lost response/unknown disposition | No grant or inferred original source; retain supported recovery evidence and ordinary cleanup |
| Target/frozen input/configuration or independence mismatch | Reject without choosing a new head, rewriting identity or broad filesystem fallback |
| Unsupported consumer, incomplete live dependency membership or unhealthy store/lease | Scope unavailable; no eligible registration, implicit repair or silent truncation |
| Admission witness retained beyond a source transition/restart | No generation/fence reuse or authority restoration; original quotas/custody remain unchanged |
| Ordinary locks, blocked storage/native operation or poisoned publication owner | Close stays independent of I/O and ordinary controls; denial cannot disable ordinary stop |

The next deliverable is one reviewed source-owner execution design covering the
original attachment/preparation disposition, actual runtime/ordinary-session
bridge, participating source-writer inventory and coherent metadata membership.
It must identify real storage/physical/native mechanisms and complete resource
bounds before an executable adapter plan. Do not implement another standalone
"eligible source" flag, generic publisher or synthetic witness factory as a
substitute. Existing complete-command/packing/native gates remain independent.

## Scoped decision

New author-selected choices are: one private admission module sharing the
publication cutoff; retained per-attempt source-fence identity that invalidates
pending admissions on source transitions; source-derived exact live dependency
closure before mutation; and private registration only after proven original
preparation plus actual runtime/bridge evidence. These are proposed for one
scoped owner review. Existing custody, codec, quota and reporting decisions are
reused without reopening them.

Accepting this document would approve these interface/ordering choices for
further design and qualification. It would not approve a production source
adapter, source/storage command, native probe or activation. The missing physical
and cross-owner mechanisms remain concrete prerequisites, not unnamed work that
a caller may satisfy with a boolean. #1397 delivers the reviewed design; #1176
remains open until its larger acceptance criteria are actually established.


## Bounded review and author dispositions

One read-only lower-cost review round used two independent questions. The
correctness reviewer approved the proposal/interface against the linked accepted
contracts and actual dormant source, with physical membership, writer coverage,
cross-owner disposition and memory bounds explicitly unverified. The builder
reviewer found the lifecycle coherent but required the accountable source owner
and typed begin/disposition seam before implementation.

The author resolves that builder question above as a proposed private source
publisher plus exact non-Clone transition and sealed validated disposition. The
ordinary runtime owner cannot assert the durable outcome; stale completion or
Drop cannot clear a later fence. This clarification is an author-selected part
of the same owner-pending interface, not an already accepted implementation or
an independently re-reviewed physical mechanism. No extra review round, numeric
allowance, durable codec or production construction was introduced. Acceptance
of these proposed choices remains a single scoped owner decision.


## Concrete original publisher proposal — 2026-11-05

[#1406](https://github.com/Rambolarsen/orkworks/issues/1406) supplies the
[joint original preparation publisher design](2026-11-05-joint-child-preparation-design.md).
It proposes a single attached rollback transaction for ordinary creation plus
complete child-source-attach, with explicit shared-worker availability,
ordinary-lineage/index, format-5 commitment and retained-tombstone choices. These
are new owner-pending amendments, not acceptance or implementation of this
admission interface. The original source authority-at-commit, complete
catalog/membership and native gates remain; atomic SQL supplies no source permit.
