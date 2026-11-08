---
type: Design Contract
title: Complete maintenance registration and original-job settlement
description: Proposed private application owner for complete root and producer coverage and retained finite physical completion obligations.
tags: [orkworks, sessions, backup, maintenance, ownership]
status: draft
workflow_status: proposed-owner-review
---

# Complete maintenance registration and original-job settlement

Tracking: [#1547](https://github.com/Rambolarsen/orkworks/issues/1547), parent
[#1250](https://github.com/Rambolarsen/orkworks/issues/1250).
This is the next written prerequisite after [#1545](https://github.com/Rambolarsen/orkworks/pull/1545).
It refines the accepted [maintenance design](2026-11-08-backup-maintenance-admission-design.md)
using the [actual owner-interface investigation](../../validation/backup-maintenance-writer-audit.md#owner-interface-investigation--2026-11-08).
The accepted [backup/restore](2026-11-07-session-backup-restore-contract.md),
[bundle/evidence v1](2026-11-08-backup-bundle-evidence-v1.md) and
[adoption recovery](2026-11-07-legacy-adoption-publication-recovery.md) remain authoritative.

**Status:** proposed integration contracts, not available runtime operations.
No producer is qualified by this document. Existing approvals stand; the new
choices below require owner approval before dependent implementation. No code,
SQL/native/model execution, live inspection/stop/capture/restore or numerical
budget is supplied. Source context is `692821e6` for the documentation base;
the actual-interface investigation retains its explicit `1ab168d3` source pin.
This proposal consumes those inspected local interfaces without claiming later
uninspected code implements them.

## Proposed choices

1. Use an application-private, fixed producer catalog and root-specific owner
   registrations. Actual constructors establish the complete scope; callers
   cannot select modules or register a successful generic empty owner.
2. Separate the coordinator's retained completion obligation from the one
   transferable execution capability. Caller/token loss cannot remove the job,
   release its last physical resources or acknowledge success.
3. Reserve a finite original-job continuation graph before admission. Each slot
   has an exact effect/owner/root and one admission; Closing permits only those
   admitted jobs' settlement, including file-to-SQL progress and publication.
4. Use generation-bound closure and readiness acknowledgements from every real
   owner, with one coordinated Open transition. Missing coverage/acknowledgement
   blocks; the catalog or an empty ledger cannot fabricate physical proof.

These are design recommendations, not claims of user acceptance. The aim is a
small private interface with ownership enforced once, rather than public tokens,
per-handler maintenance booleans or a second storage authority.

## Catalog, root registration and completeness

The application maintenance owner retains a fixed catalog for the supported
build/profile. Each entry identifies its actual producer owner, affected scope
rules, ingress paths, recipe kinds, scheduling closure operation, physical
completion source, readiness operation and existing resource/native dependencies.
An entry is supported only when all those real operations are implemented and
qualified. A future code review must enumerate every writer route in that entry;
a name, trait implementation, grep result or fixture is insufficient evidence.
There is no dynamic plugin-registration protocol in this slice.

The proposed registration would attach to the application constructor that owns
the root, before ordinary producers can obtain a writer or schedule work.
It binds captured native identity, logical workspace, incarnation, retained
lease/owner and the catalog version to the existing RootKey. It begins
Blocked/unverified. Required startup/format/fence and external-state validation
runs under separately admitted exact startup/recovery ownership while ordinary
admission remains closed. Only complete actual readiness can open the root.
For a new output scope, capture the real parent with exclusive constructor
admission; create/bind the child through that owner. Do not register a fictional
native identity for a directory that does not yet exist.

Every catalog owner returns one of: a supported root-specific registration;
qualified not-applicable with actual scope/format evidence; or unavailable.
An empty owned namespace is still a supported registration with its real owner,
revision and physical acknowledgement. Not-applicable is not inferred from zero
files or lack of an active handle. Global/native settings and repository bytes
retain their existing exclusions, but their work that can affect a captured root
must use the root's admission. Unknown scope or an unsupported external direct
writer blocks qualification; registration cannot confine arbitrary same-user I/O.

The complete registration has two obligations: every captured namespace/relation
has its actual owner under v1, and every producer capable of changing it crosses
admission. Excluded transient spool data must still have its ingress/consumer
classified. No overlap, alias or extra namespace can be silently dropped; actual
native scope owners must either establish an unambiguous partition or reject.
It is a reviewable closed coverage argument plus actual owner operations, not a
caller-provided list. It emits existing v1 Modules fields through their evidence
owner; this internal catalog introduces no new receipt codec.

| Catalog responsibility | Existing owners to integrate; no generic substitute |
| --- | --- |
| Root construction, switching and lifecycle | Application open/create/resume/ending, workspace lease/identity and organization reservations; switching/opening that touches a registered root is itself admitted. |
| Session durable state | SQL worker, legacy transactions and quarantine-capable reads, metadata events, organization and hook/HTTP mutation. |
| Runtime-derived persistence | Terminal output/size/checkpoints/final trim, Peon/provider callbacks and blocking publication; active/pending runtimes still reject maintenance without stopping them. |
| Recommendation and evaluation | Periodic/manual Taskmaster scheduling, delivery, graph/journal operations, SQL cleanup/open ownership. Global analysis work is classified by the roots it can actually affect. |
| Observation and cleanup | Captured observation writes/sequence updates, all trim/spam/retention jobs, startup GC and original deletion file/progress stages. |
| Remaining workspace payloads | Workspace-memory edits/settings synchronization, image upload/publication/recovery/cleanup, branch associations and selected-detail writes. In-memory capacity projection is not invented durable storage; old capacity files need actual classification. |
| External ingress and scopes | Authenticated HTTP/helper/relay, installed hook/spool consumers and unclean descendants; source authentication is not external quiescence. |

A root cannot unregister, replace its incarnation or discard an owner while jobs,
maintenance effects or unresolved protection obligations reference it. Workspace
switch changes selection only; original registrations/leases remain retained.
A catalog/profile/scope change invalidates affected maintenance attempts and
requires requalification. New roots cannot extend a closing attempt's root set;
independent registration cannot route an effect around an existing closing root.
Multi-root job admission checks all original roots and installs all obligations
in one short registry transition, or installs none.

## Original job record and execution capability

Before any effect, the actual producer reserves its complete finite recipe and
resources, then registers the job while every original root is Open. Failed
registration releases only unused reservations and performs no effect. The
registry retains a job record containing original roots/incarnations, owner,
checked admission serial, recipe instance, effect slots, completion sources,
retained physical resources and unresolved recovery/protection disposition.
A unique execution capability is transferred to the actual worker/callback;
it is not cloned to authorize another job. Existing domain identity/currentness
checks still apply after admission and before effects.

The coordinator owns that obligation independently of the request's reply channel.
Resource transfer from an application local into a worker, completion object or
blocking callback must be gap-free: the old owner remains until the next physical
owner is established. Retained resources may live in the actual worker/completion
object rather than a clone in the registry; the record retains their exact owner
and completion route. A lost execution capability or coordinator failure records
unknown/Blocked, never decrements a successful-drain counter. Destructors may
report loss and request cancellation; they cannot certify settled I/O or release
resources still used by physical work.

An actual producer reports a slot's known no-effect, physically completed success,
physically completed failure/partial result, or unknown completion. Logical failure
can be physically settled; it does not erase changed bytes, missing progress or
recovery/protection obligations. A record becomes physically settled only after
all admitted work has acknowledged completion and every unstarted slot is
permanently rejected/no-effect. Recoverability is a separate owner verdict.
Only actual completion owners can discharge unknowns; no timeout, caller message,
receipt replay, digest match or zero sampled workers resolves them.

## Finite continuations and concrete integration

A recipe is a finite acyclic graph selected by the actual owner before admission.
Each slot binds the original job, roots, incarnation, effect kind, responsible
owner, predecessors, finite allowance and physical acknowledgement. Admission is
at most once; a successor requires its specified predecessor outcomes. A recipe
cannot add slots, roots, retries, analysis or new user work during Closing.
Potential retry/recovery needs a separately declared finite slot with the same
exact identity and resource bounds, or a later explicitly authorized recovery
operation while blocked. This design chooses no retry policy or numeric limits.
Unknown/unbounded recipe or unavailable reservation rejects before admission.

Continuation admission and cancellation use the registry's short transition;
local owner checks run outside that lock, with identity revalidation before the
effect. Admission produces an owned slot even if physical execution begins later;
closure must await it. A later stale domain check reports actual no-effect rather
than redirecting the slot. Dropping an async task is never that acknowledgement.
Maintenance cancellation forbids new maintenance effects but does not cancel the
distinct pre-Closing ordinary job's required settlement slots. Ordinary-job
cancellation rejects unadmitted optional slots; a required progress/publication
continuation follows the actual owner's accepted recipe and truthful outcome.
No cancellation path can admit a fresh user operation through those slots.

| Recipe | Required physical owner transfer and acknowledgement |
| --- | --- |
| SQL mutation then publication | Retain `RuntimeWriteResources` through enqueue/grant/execute, `InternalMutationTicket::finish`, `InternalCompletion` and actual blocking publication return. `MutationCancellation` does not imply rollback after grant. Committed-detached is an actual durable result; waiter delivery is not the completion source. |
| File cleanup then progress | `CapturedDeletion` retains original deletion identity/lane/lease/file capacity. The exact stage owns replay/image/settings/recommendation guards through removal and SQL progress. `DeletionCleanupResult.files`, `.progress` and `.detached` are independently recorded. Failure after removal leaves the truthful progress/recovery obligation; close SQL only after all such continuations settle. |
| Observation batch or branch/settings/image write | Register the actual captured writer/edit/publication owner before admission. A synchronous call's return is completion only for its physically bounded work; any child/callback has its own predeclared retained slot. Locks do not close future producers. |
| Terminal/provider/evaluation work | Actual persistence joins and retained blocking work report completion through original-job ownership. Production scheduling needs real pause/no-new-work operations; test-only shutdown and async abort do not satisfy the contract. |
| SQL recommendation reopen during release | Keep the actual owned open task/candidate/lease, identity validation and close-on-failed-delivery behavior. `SqlRecommendationWorkspace` still needs a real production adoption/readiness owner; returning the candidate cannot open ordinary admission. |

The source audit supplies local handles for these transfers, but no current
module supplies the complete registration/retained-record/continuation protocol.
These rows specify required integration, not success wrappers around missing
interfaces or permission to alter existing cleanup/domain authority.

## Closing, physical completion and readiness

Closing atomically closes the exact root set against ordinary admission and
freezes the expected registrations/generation. Outside the registry and application
locks, every real scheduling owner acknowledges no new scheduling for that
attempt. Already scheduled/blocked callbacks still cross ordinary admission;
only exact original slots may proceed. A scheduler pause acknowledgement cannot
stand in for its in-flight jobs' physical completion.

The coordinator waits for all original outer records/continuations and relevant
recovery/protection verdicts, then asks the actual SQL owner to close and await
connection destruction. It obtains actual module closure, namespace/revision and
native/format evidence before Held/Q. Legacy no-worker needs actual absence;
preservation-only can classify known damaged durable evidence after all physical
writers settle, without claiming semantic recoverability. Unknown completion,
unsupported coverage or a hung owner stops within the qualified bound and leaves
Blocked. No successful partial Modules list or fake drained owner is emitted.

Release uses a new readiness generation while ordinary gates stay closed.
Actual owners prepare readiness without starting ordinary scheduling or reopening
a writer bypass. The coordinator verifies original lease/identity, no outstanding
physical work, complete format/recovery/restore-fence state and every expected
owner's readiness acknowledgement for that generation. SQL reopen/adoption is an
exact maintenance effect, not a normal-open escape hatch. One short transition
opens all required roots and retires old claims/Q; schedulers can run only after
observing that Open generation. Any failed/stale acknowledgement leaves Blocked.
Prepared owners/handles, including a reopened SQL worker, remain retained by that
readiness attempt and inaccessible to ordinary writers while Blocked. Failure is
not assumed side-effect-free. The actual owner must acknowledge physically safe
teardown before its handles/resources can be released; otherwise retain an
unresolved protection obligation. Late readiness results record their actual
outcomes against the original attempt but cannot open a stale generation or
silently transfer prepared handles to a replacement attempt. Explicit qualified
resolution must account for those handles before retrying release.
Startup likewise cannot reconstruct live authority from old receipts.

Existing Q retirement, cancellation linearization and explicit owner resolution
remain as accepted. Capture completion and required backups must be established
before the atomic first-data-effect admission retires Q; this protocol does not
add another capture/install transition or change restore/marker authority.

## Lock order, boundedness and conditional checks

Use the accepted capture-context, register, revalidate sequence. The registry
transition lock covers only in-memory checks/transfers. Never call into a module,
wait for resources, close/reopen SQL, touch files or acquire application/module
locks while holding it. Producer completion reports after releasing its domain
locks; notifications carry stable original identities and cannot execute callbacks
inside the registry transition. All waits use retained owners outside projection,
workspace, session, prompt-transition, module and registry guards.

Registration/job/slot/root counts, retained bytes/handles, notifications, waiting
deadlines and recovery scratch require actual finite enforcement before code.
Existing file/SQL allowances are not backup budgets. No proposed catalog entry
can be qualified while resource or native/external inputs remain Unknown.

Later actual-owner checks must cover: admission racing closure across multiple
roots; rejected partial registration; empty namespace versus unsupported owner;
switching with retained old-root jobs; waiter loss before/after SQL grant; file
success with failed/pending progress; publication after SQL completion; late
ordinary callback; slot replay/undeclared successor; callback completion racing
cancellation; owner/coordinator loss; readiness failure and stale generation;
startup with lost volatile ledger; and an alias/unknown ingress route. These are
conditional verification oracles, not executed tests or a new probe allowance.

## Handoff and approval boundary

Approval would accept these four integration choices for further qualification.
It would not close the [qualification plan](../plans/2026-11-08-backup-maintenance-qualification.md)
registration/recipe/readiness boxes: actual owner operations and complete physical,
native/external, inert-validator, resource and restore-fence evidence remain
required. A later bounded implementation plan must identify real constructors,
writer routes, retained worker/completion owners and actual checks; no pure model
or mocked catalog may be presented as that closure. #1242/#1243/#1248/#1250/#1244
retain their responsibilities. No product/runtime implementation is authorized
by drafting or accepting this document alone.


## Review disposition

Fresh builder review identified partial-readiness failure ownership: prepared
handles, including reopened SQL, must stay owned and gated while Blocked until
actual safe teardown or explicit qualified resolution. That rule is now explicit.
Fresh independent authority/coverage review found no material conflict or missing
writer family; constructor wording was clarified to avoid implying registration
already exists. These source/design reviews supply no runtime qualification.
