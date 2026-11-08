---
type: Design Contract
title: Backup maintenance admission and physical settlement
description: Proposed live per-root ownership, complete writer closure and explicit release before backup or restore can claim quiescence.
tags: [orkworks, sessions, backup, maintenance, ownership]
status: stable
workflow_status: accepted-for-qualification
---

# Backup maintenance admission and physical settlement

Tracking: [#1539](https://github.com/Rambolarsen/orkworks/issues/1539),
[#1250](https://github.com/Rambolarsen/orkworks/issues/1250).
Consumes accepted [backup/restore](2026-11-07-session-backup-restore-contract.md),
[bundle/evidence v1](2026-11-08-backup-bundle-evidence-v1.md),
[adoption publication/recovery](2026-11-07-legacy-adoption-publication-recovery.md)
and the [pinned writer audit](../../validation/backup-maintenance-writer-audit.md).
The accepted storage policy, v1 bytes, purpose rights and marker authority stand.
After #1540, the owner answered “go ahead” to the explicit four-choice approval
request. [#1541](https://github.com/Rambolarsen/orkworks/issues/1541) records
acceptance on 2026-11-08 of this unchanged conditional design for further
qualification. It is not an implementation plan, registered writer profile,
product endpoint or executed quiescence/native result.

## Accepted choices for further qualification

1. **Per-root owned work, including detached work.** One private application
   maintenance owner tracks every admitted job against its original captured
   roots and workspace incarnation, including jobs whose caller or selected
   workspace has changed. Empty current session lists never replace that ledger.
2. **No automatic runtime stopping.** Reject maintenance for active/starting/
   stopping runtimes, pending start/resume/lifecycle work or unproven descendants.
   The owner must resolve quiescence separately; maintenance cannot kill, adopt,
   resume or silently close another session to obtain a backup window.
3. **Outer settlement before SQL close.** Close new scheduling/admission, let
   already admitted finite jobs settle their declared callbacks/file effects/SQL
   progress, then close SQL and establish Q. Cancelling an async waiter or timing
   out cannot be reported as physically drained.
4. **Explicit validated release.** Guard drop, cancel, timeout or owner loss never
   automatically reopens ordinary admission. Newly registered/restarted roots
   also start blocked until actual startup/readiness validation; an empty volatile
   ledger is not a clean-state proof. Safe cancellation can return through the
   same explicit release checks; ambiguous state stays blocked for recovery.

These maintenance-interface choices are accepted for further qualification.
Reuse this approval and accepted backup/adoption decisions within scope. Actual
owner/profile/native/resource inputs below remain unavailable until qualified;
no success adapter is assumed. The [qualification plan](../plans/2026-11-08-backup-maintenance-qualification.md)
closes those inputs before a bounded dependent implementation plan.

## Ownership and scope

The application maintenance owner is the sole writer of an in-memory admission
registry. It owns each maintenance operation and its settlement coordinator
independently of the requesting waiter; a dropped request cannot destroy the
root's completion obligations or become the only actor able to release them. It does not replace existing metadata, SQL, image, recommendation,
observation, terminal or lifecycle owners. Those owners supply their actual
admission and settlement operations through one private interface. No caller
may construct a permit from a path, nonce, stored receipt or agent capability.

A RootKey binds the captured native directory/parent owner, logical workspace,
workspace/owner incarnation and actual module scope. It is not a display path or
selected-tab ID. The registry retains original roots/leases and admitted-job
resources across workspace switches. Unknown native identity, aliases/overlaps
or unregistered namespaces block the operation. Adoption closes both selected
source and global roots; restore closes its exact destination and captures the
selected immutable backup under its own qualified read ownership. Output parents
are disjoint captured scopes with exclusive constructor admission; a generated
output directory does not exist until that owner creates/binds it. No virtual
root identity is invented for a nonexistent output directory.

Per-root closure is process-local admission ordering, not an atomic multi-root
filesystem transaction or same-user confinement. Actual root lease/identity and
external-writer profiles remain required for every root. Acquiring the existing
global WorkspaceLease alone cannot qualify a retained local source or restore
destination. Incomplete scope/producer registration rejects before capture.

The closed module registry derives from actual producer contracts and native
scope, not caller input or a UI inventory. It includes starts/resume/ending,
metadata/organization and mutating reads, HTTP/agent/hook/relay writers, terminal
output/size/event/checkpoint persistence, provider/Peon publication, SQL,
Taskmaster/evaluation/delivery, recommendation graph/journal, observation/sequence/
trim, retention/GC, workspace settings, branch associations and image upload/
publication/cleanup. Global settings/native/repository data stay excluded, but
work from those owners that can mutate captured roots still needs admission.
Every empty namespace requires its real owner's acknowledgement under v1.
The audit is a map of available seams, **not** this qualified closed registry.

## Small live interface and job ownership

The private interface has five responsibilities; names below describe contracts,
not a new public protocol or prewritten Rust implementation:

| Operation | Required live result |
| --- | --- |
| admit ordinary job | While all original roots are Open, reserve finite job/continuation/resources and return one owned WorkClaim. No effect before admission. |
| begin maintenance | Close ordinary admissions/scheduling for the exact root set, validate unchanged context/no runtimes, and start bounded physical settlement outside app guards. |
| acquire held claim | Only after all module/outer work and SQL closure validate, return one MaintenanceClaim and fresh Q for the captured roots/purpose. |
| admit maintenance effect | Consume one exact phase/target/resource admission under that live claim, through the existing backup/adoption/restore owner. |
| release or report blocked | Validate complete physical settlement, format/recovery state and module readiness before reopening; otherwise retain blocked state. |

A WorkClaim records exact owner profile/job identity, original RootKeys, admission
generation/serial, declared finite effect recipe and retained resources. One
physical job owns it; transfer into blocking tasks/callbacks is allowed, arbitrary
cloning into new work is not. Memory, resource slots and native handles must remain
owned through physical completion, not just through an async reply or SQL commit.

The admitted recipe names bounded **continuation slots** for the same job: for
example file cleanup followed by its SQL progress write, or SQL mutation followed
by its publication callback. Slots bind the original identity, owner, phase and
resource allowance; each is consumed at most once. During Closing, these original
slots may settle while new ordinary work is rejected. They cannot start another
analysis, runtime, session, user mutation or undeclared retry. Missing/overflowing
bounds or a dynamically unbounded continuation means the producer is unqualified.
A caller cannot wrap unrelated work in a pre-existing claim to bypass closure.

An owner explicitly settles every effect and continuation as known completion,
known no-effect/rejection, or partial/unknown outcome. Physical completion is not
logical success: a failed file job may have changed bytes and left durable work.
Such a job can release its execution resources only after its actual operations
settle, but its unresolved recovery/protection obligation remains recorded and
can still block Q/release. No pending SQL progress bit may be fabricated.

The registry owns the completion obligation and retained resources independently
of the waiting caller. Dropping an unsettled claim records unavailable/unknown,
not successful completion or an empty ledger. Actual module constructors must
prove that all child/blocking work remains tracked through settlement. Unknown
ownership keeps the root blocked; there is no destructor-based “all done.”

## Admission order and lock discipline

Use a short registry transition lock only for in-memory identity/counter/state
changes. It never waits for a module, SQL, filesystem, provider, UI or app mutex.
No maintenance wait or module stop/close runs under projection/workspace/session,
prompt-transition, module-store or registry locks. Do not hold all existing app
locks as a substitute for maintenance: detached work deliberately outlives them.

Ordinary application entry follows these ordering constraints:

1. Capture the opaque original root/context under its existing short application
   ownership checks; release those guards without effects.
2. Admit/register the job under the maintenance owner while the roots are Open.
   Release the registry transition lock.
3. Revalidate the same context/identity under the existing application/module
   owner, then perform the original operation through its captured WorkClaim.
   A stale snapshot settles as rejected/no-effect; it cannot switch to another
   root after admission. All physical work executes outside application guards
   wherever the existing domain contract requires that.

Maintenance closes the selected root set together under the registry lock, then
releases it before rechecking current app/native context or waiting. A stale
capture or already admitted workspace switch can make begin fail; the operation
must settle through explicit release/blocked rules, not replace its root set or
silently switch the selected workspace. Each open/switch that can touch a closing
root is itself an admitted job; the selected UI workspace is never the ledger.
No ordinary effect may sneak between a successful close and Q through a path
that omitted registration. A missing bypass route blocks qualification.

## State transitions and establishing Q

| State | Allowed work and transition |
| --- | --- |
| Open | Registered ordinary admission allowed with actual existing domain checks. begin changes the entire target set to Closing; no capture receipt exists. |
| Closing | Reject new ordinary job/scheduler admission. Pause module scheduling and wait for original admitted continuations/physical outcomes. No Q or backup success. |
| Held | All required closure proofs hold; fresh MaintenanceClaim/Q may admit exact authorized maintenance work. Ordinary admission remains closed. |
| Releasing | Maintenance effects physically settled; revalidate authority/store and prepare every module for a new Open generation without allowing early scheduling. |
| Blocked | Unknown owner/effect, incomplete recovery, deadline/abandonment or failed readiness prevents Q/reopen. Retain ownership/evidence and require explicit qualified resolution. |

One registry transition serializes job admission, maintenance closure and claim
cancellation per root set. Generations/serials are checked counters with fresh
owner-instance/attempt nonces; overflow blocks before admission and never resets
an existing live claim. Stored counters/nonces are data, not restored authority.
Finite counts/bytes, per-job recipes, wait bounds and resource retention must be
qualified before the controller is implemented; this contract assigns no budget.
No unbounded busy polling, force-stop-on-timeout or borrowed deletion allowance.

Closing establishes Q in this order:

1. Validate exact root set, lease/context, registry and explicit owner-resolved
   external quiescence. Reject active or pending runtime/lifecycle states; do not
   turn a user-requested backup into session shutdown. Close ordinary admissions
   and pause producers; obtain each module's no-new-scheduling acknowledgement.
2. Settle every original outer job and declared continuation: terminal final
   persistence/trim, provider publication, hook/agent requests, Taskmaster delivery,
   observation rewrites and file removal/progress among them. Drain notifications
   carry exact original root/job identities, not a sample of visible handles.
3. Require capture-relevant recovery/protection work settled or classified by
   the purpose's qualified owner contract. Recoverable-workspace needs restorable
   semantics. Preservation-only may retain known raw unresolved/corrupt durable
   evidence after every physical writer settles; that does not make the state
   recoverable or ready for ordinary opening. Current-body bytes plus zero
   sampled jobs cannot bless unknown physical completion or protection state.
4. For roots with a SQL worker, only now close it, while it can no longer be
   needed by an outer continuation. Await closed admission, queue exhaustion and actual connection
   destruction. Obtain the SQL owner's real closed-image/auxiliary result;
   close() alone does not provide integrity, sealing or full native durability.
5. Obtain every actual module's drained/closed acknowledgement, current relevant
   revisions/protection and original roots/format evidence; recheck no admitted
   work or stale ownership remains. Establish fresh Q and Held only on complete
   known results. Emit v1 maintenance evidence through the actual owner with its
   existing qN/claimId/serial/Modules fields; no byte-format change is introduced.

Legacy roots with no SQL worker require their actual format owner's proven
absence/eligibility, not a phantom close result or invented SQL counters. Q proves
live writer closure, **not semantic recoverability**. Preservation-only may bind
known corrupt/unresolved format evidence and complete readable native state
without a valid SQL image; it still requires actual no-writer/closure and full
physical scope proofs. Unknown physical ownership or unreadable data blocks.
Recoverable-workspace needs the separate full SQL/record validation before its
seal; retained-evidence keeps its accepted exclude-only rules. No damaged-state
capture can issue a recoverable-workspace result through this distinction.

This deliberately keeps SQL available while earlier file jobs still need to
write their truthful progress. It neither aborts them to empty a counter nor
retries failed effects. A hung/unsettled owner causes bounded stop/Blocked, not
a quiescence receipt. Current source has only parts of these interfaces; the
[pinned audit](../../validation/backup-maintenance-writer-audit.md) records them.

## Held capture versus maintenance installation

A live held claim binds exact purpose, roots, owner instance/Attempt, current
serial and allowed maintenance phase. It is necessary but not sufficient for an
effect: actual native/control, backup, application, resource, SQL/payload/marker
and recovery owners must all admit their own exact work. It grants no generic
file-write/delete/overwrite authority and no new capability to an agent.

For capture, data roots remain unchanged from Q through complete final validation
and seal publication. Maintenance effects can create only the separately admitted
private output objects/control files under captured disjoint output parents.
Every effect is tracked through physical settlement; cancellation can prevent
its next admission but cannot undo an admitted effect. Releasing maintenance
before seal settlement gives no complete-capture acknowledgement.

Held has an internal capture-eligibility state: capturable or data-effect-admitted.
Every capture-effect admission checks the current Q, operation/phase, generation,
serial and cancellation state under the registry transition lock. An already
admitted effect may settle; later serial increments do not erase its obligation.

Adoption/restore may mutate data roots only through their separately approved
per-effect owners and progress/fence rules. First data-effect admission must,
in **one registry transition**, require all capture effects physically settled
and their required completed backup/preservation results, consume the current
exact domain admission, change capturable to data-effect-admitted, retire Q for
new capture admissions, and advance the checked serial before returning the
owned effect claim. Reject if any prerequisite/serial/cancellation check fails.
No I/O occurs under that lock. A capture racing this transition either owns an
earlier admitted effect that must finish first, or is rejected with stale Q.
Even known no-effect settlement of the data claim does not revive old Q; fresh
capture needs a new qualified point. This is conservative retirement, not a
claim that data changed atomically with memory state. Later current
maintenance evidence validates the actual authorized phase and baseline plus
exact owned changes; it cannot reseal a new backup as if the original Q were
unchanged. Original backup receipts remain captured data subject to their actual
current-validity checks. New capture after data changes needs a fresh qualified
capture point, inventories and applicable confirmation; no preserved old claim
can be relabelled to avoid review.

Restore must first preserve current destination state and establish its qualified
durable, opener-discoverable fence before replacement. An old valid marker cannot
open a mixed destination behind that fence. This in-memory controller is not
that durable protocol and supplies no startup recovery codec. Adoption still
uses restore-first after owner loss; no controller record can revive its old
approval/publication claim. The existing valid format-2 marker remains the
activation point, with no JSON fallback on unknown cutover evidence.

## Cancellation, release and restart

Before Closing wins admission, cancellation means no maintenance claim. During
Closing, stop admitting new work but retain original job/resources until their
physical outcomes settle. If the request disappears or its deadline expires,
record Blocked and report the exact unresolved owners; do not declare drained,
force-kill a session or release a held lease to make progress appear complete.
An admitted bounded effect may still settle and record its truthful outcome.
Cancellation is an irreversible flag for that maintenance Attempt, serialized
under the same registry transition as Closing, Held issuance and each effect
admission. If cancellation wins before Closing, no roots close. If Closing wins
first, cancellation revokes all future Q/maintenance-effect issuance and leaves
the root Blocked until explicit resolution/release. A Held claim issued before
that flag is thereafter stale; it cannot admit new work. If an effect admission
wins first, that effect retains settlement-only ownership/resources and can
report the actual durable outcome, including a valid seal/marker; cancellation
does not make it rolled back. The application-owned coordinator continues
tracking/draining original jobs; if the coordinator itself is unavailable, the
root stays blocked for qualified resolution. No later ordinary/capture/installation
effect may begin under a cancelled MaintenanceClaim. An ordinary WorkClaim admitted before Closing remains a
distinct owner of only its original bounded settlement continuations; cancelling
the maintenance waiter cannot strand that job's required SQL progress.

Only explicit release/resolution can move Closing/Held/Blocked to Releasing.
It requires no outstanding physical work or unknown completion, current root/
lease ownership, fully validated format/recovery evidence and a qualified
restorable state. For capture-only failure/cancel with no data effects, private
partial outputs may remain retained; they grant no success and require separate
exact-owner cleanup. A safe abort may reopen after these same release checks;
“cancel” by itself is not a readiness proof.

If SQL was closed, its actual owner must reopen the same current validated store
under the retained lease and expected schema/epoch/generation/import binding,
without empty recreation or stale JSON fallback. Reopening and module readiness
are maintenance effects while ordinary scheduling remains closed. Failure keeps
Blocked. All modules acknowledge readiness for the new generation before the
single registry transition to Open; scheduling must observe Open and cannot
restart early merely because its own pause flag was cleared. The successful
transition retires every old MaintenanceClaim/continuation and Q witness.

After data-root installation effects, release additionally requires the complete
adoption/restore owner result and any required durable fence settlement. An
unresolved file effect, missing progress or ambiguous fence cannot be ignored to
restore usability. A cancelled marker admission that already published a valid
format-2 marker retains SQL authority; controller cancellation does not roll it
back. No automatic recursive rollback or downgrade is selected here.

Blocked obligations are resolved only by a fresh, explicit application-owned
resolution operation with the exact affected roots and separate existing domain
recovery authority. It uses real owner/native observations of each original
job/effect/progress identity to establish known physical completion/no-effect or
a qualified preserved/restorable state. Merely clearing a counter, matching a
pathname/hash, dropping a token or submitting a receipt cannot discharge it.
Any replacement still needs the accepted preservation/restore-fence authority;
this selects no new repair or adoption roll-forward algorithm. Resolution may
admit only its separately bounded, exact recovery/readiness work while ordinary
admission remains closed. If the original ledger/evidence is missing, the owning
profiles must establish complete actual state from startup evidence or remain
unavailable; absence of volatile entries is never success.

Process loss loses every live claim. New/restarted registry entries initialize
as Blocked/unverified, not Open with a zero job count. Qualified startup inspects actual
format/cutover/restore evidence and either proves its existing normal-open rules
or remains blocked; it never reconstructs old tokens from receipts. Valid sealed
backups can be inertly validated as data by a new operation. Unclean descendant
state requires explicit owner resolution; the registry cannot prove OS process
termination from a previous owner's absence. Unsupported older binaries remain
unsupported, not controlled by this process-local gate.

## Conditional failure and ordering oracles

| Boundary/race | Required result |
| --- | --- |
| Active runtime, resume/start in progress, unknown descendant or missing module | Reject begin/Q; no silent session stop or fake drained entry. |
| Ordinary job wins before Closing | Track original job and bounded continuations through physical settlement; no source inventory yet. |
| Closing wins before ordinary admission | Reject new work, including read paths that can quarantine or reconcile. Pure inert diagnostics require a separately proven no-write path. |
| Caller/workspace changes after job admission | Original captured root/resources persist; stale context rejects publication or produces truthful detached result. No ledger transfer to the selected tab. |
| File removal admitted, SQL progress still pending | Keep SQL open until this original continuation settles; incomplete progress/recovery blocks Q. |
| SQL worker closed but detached publication still outstanding | No Held/Q/release; connection closure alone is insufficient. |
| Peon JoinSet aborted, blocking work still admitted | Retain physical completion obligation; no fabricated zero count. |
| Module paused, late HTTP/hook/scheduler call arrives | Ordinary gate rejects. Original finite continuation may only settle its predeclared job. |
| Deadline, dropped waiter or unreported token destruction | Stop/Blocked until actual owners settle/resolve; never automatic reopen. |
| Registry/root/profile/revision changes or bound overflow | Invalidate attempt; no partial Modules or new root appended to the old claim. |
| Capture output effect settles after cancellation | Retain actual data/partial seal outcome; no next admission or rollback promise. |
| First adoption/restore data mutation admitted | Old Q stops certifying unchanged-source capture; original domain progress/fence remains authoritative. |
| SQL reopen/readiness or durable fence settlement fails | Keep ordinary admission blocked; no empty store, JSON fallback or early scheduler. |
| Restart after lost claim, even with matching digest/nonce | Validate actual state with new authority or stop; never revive a permit or roll forward lost-owner adoption. |

## Qualification and handoff

Before code, close each actual module's
registration, no-new-work transition, finite original-job continuation contract,
physical completion and readiness interface. Map every audit family and excluded
producer which can affect captured roots; validate empty namespaces too. Establish
native/root/external-quiescence proof, inert store/record validators, all SQL
auxiliary/closure/reopen semantics, trusted local confirmation, restore-fence
startup routing and finite resource/time bounds. No mocked “drained” owner can
replace a missing path. This contract registers no production Profile.

Later copied/synthetic verification must exercise every race above with actual
owners, including switched-away jobs, late callback admission, SQL-progress after
file effects, cancellation before/after grant, timeout/abandonment, lock contention,
release/reopen failure, unknown native state and post-cutover writes. Product
implementation needs a bounded reviewed plan, TDD/full-repo/current-diff review
and the existing dependency gates. Native execution requires separate finite
authorization; no live capture/restore, session stop, runtime rollout or new
model/native allowance is authorized by this written contract.

## Review disposition

Fresh authority review found no material conflicts with accepted purpose,
marker/restore, v1 or no-automatic-stop rules. Fresh builder review identified
three concurrency ambiguities. Clarified atomic Q retirement with first data
admission and racing capture rejection; actual-owner resolution of blocked
obligations with startup blocked/unverified rather than a fabricated empty ledger;
and cancellation linearization with independently owned settlement coordination.
These are reviewed interface rules, not executed races or qualified profiles.
