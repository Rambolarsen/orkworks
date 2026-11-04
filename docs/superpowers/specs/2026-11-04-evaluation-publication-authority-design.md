---
type: Design Proposal
title: Initial evaluation publication authority owner
description: Proposed private claim and revocation owner, concrete lifecycle integration seams and a bounded non-I/O first unit under the accepted evaluation policy.
tags: [orkworks, hierarchy, evaluation, authority, lifecycle, qualification]
status: draft
---

# Initial evaluation publication authority owner

Tracking: [#1369](https://github.com/Rambolarsen/orkworks/issues/1369), under
[#1176](https://github.com/Rambolarsen/orkworks/issues/1176) / #738.
Inspection base: `4c14c836cefa8a267cadf671ec89dce267defcc0`.
Disposition: **Proposal for scoped owner decision; no implementation approval.**

## Outcome and unchanged decisions

Give the [initial evaluation/artifact recipe](2026-10-30-evaluation-artifact-producer-design.md#initial-child-evaluation-publication-derivation--2026-11-04)
a concrete owner for its volatile authority cutoff. Reuse the
[accepted claim-first policy](2026-10-24-hierarchy-authority-ordering-design.md#decision-in-plain-language):
revoke first denies a new claim; one exact claim that wins first may reach proven
commit/rollback after stop. SQL commit remains durable acceptance; response
commitment is separate. This proposal does not reopen that policy or extend it
to preparation, plan approval, allocation, finalization, read-work debit or purge.

This is an author-selected module/integration design, not an accepted source
codec or evidence of current enforcement. It changes no serialized field/domain,
request/receipt key, resource limit or bearer class. The #1356 opener stays
unqualified; no database, runtime route, capability issuance or native/model
probe is enabled by accepting or merging this document.

## Source findings that constrain the design

| Inspected seam | What exists; what it does not establish |
| --- | --- |
| [AppState](../../../crates/orkworksd/src/main.rs), sessions/projection_lock/workspace | One sidecar instance already owns ordinary runtime/workspace state. It has no hierarchy publication authority owner |
| [SessionRuntime](../../../crates/orkworksd/src/runtime/session_runtime.rs), RuntimeIdentity/RuntimeGeneration | Exact runtime instance and process-local u64 generation support ordinary stale-completion checks. Neither is the existing hierarchy AuthorityGeneration Digest or durable child AssignmentIdentity; do not cast/hash/rename one into the other |
| [terminal_runtime::new_workflow_report_token](../../../crates/orkworksd/src/runtime/terminal_runtime.rs) | Existing getrandom::fill obtains 32 OS-random bytes and propagates failure. Reuse that primitive pattern with independently generated nonsecret identity bytes, never an actual reporting token |
| [SessionApplication::transition_session_status](../../../crates/orkworksd/src/session_application.rs) | Serializes ordinary transition decisions with prompt_authority::transition_lock and later performs metadata work while its guard remains in scope. It cannot simply become the no-I/O hierarchy latch |
| [SessionApplication::delete_session](../../../crates/orkworksd/src/session_application.rs) | Captures the live generation and sends kill before the generation-bound ending transition. Hierarchy close must precede this irreversible signal for the captured runtime, rather than wait for final persistence |
| [clear_ended_session_tracking_for_runtime](../../../crates/orkworksd/src/session_application.rs) | Generation-aware cleanup revokes ordinary reporting/prompt state. It is a final backstop, too late to be the only hierarchy revocation point |
| [SQL lifecycle_current](../../../crates/orkworksd/src/session_application/sql_writes/lifecycle.rs) and [EndingPersistence](../../../crates/orkworksd/src/runtime/terminal_runtime/ending_persistence.rs) | Ordinary SQL completion checks captured workspace/runtime ownership. Its execution ticket is not an evaluation publication claim or source preparation proof |
| [WorkspaceIdentity / WorkspaceLease](../../../crates/orkworksd/src/workspace_runtime.rs) and open_workspace_with_identity | Retained directory identity and an Arc-held advisory OS lease are real integration seams. A clone/path/revalidation precheck proves neither whole-I/O exclusive ownership nor the hierarchy retirement protocol |

These are bounded source findings, not exhaustive coverage of all callers. Future
adoption must inventory every actual producer, revocation and publication writer
at its implementation base, including JSON and SQL ordinary-session branches.

## Proposed owner and private interfaces

Add `taskmaster/publication_authority.rs`, owned once by AppState when a later
qualified adapter is adopted. It is separate from prompt-authority and the
ordinary session-store worker. There is no process-global registry keyed only by
session ID, model-accessible constructor or generic SQL/publication method.

The owner stores bounded immutable binding references, close/health/namespace
state, a checked claim sequence and the sole active-or-retiring claim. Authority
entries correspond only to actually admitted source/runtime bindings, never
requests, receipts or historical evaluation revisions. Old workspace entries
close on detachment; only the one unresolved claim may retain its old binding.
Complete entry/FIFO/buffer memory accounting remains an adapter gate, with the
existing source/worker limits unchanged; this proposal creates no extra queue.

| Proposed internal operation | Contract |
| --- | --- |
| register_source_binding | Only the future source/capability owner can install an immutable binding after proven preparation and runtime admission. Bind full P/T/configuration/declaration/target, workspace/storage/admission epochs, actual runtime identity and existing AuthorityGeneration. No supplied eligibility boolean, session label or reconstructed history |
| prepare_evaluation | Authenticate/rate-check and resolve exact retry first; for a new write build the complete immutable source/read/effect intent outside the latch. Return an opaque ReadyEvaluation bound to the one worker position; preparation/queue admission is not a claim |
| try_claim | Only the private worker at the mutation boundary can consume ReadyEvaluation. Under the short owner latch compare its exact registered binding, selected scope, namespace and close/health state; consume the checked sequence and install one exact claim |
| close_binding / close_scope | Atomically close the exact registered runtime/run/workspace/namespace, deny new claims and response commitments, preserve an already-claimed operation under the accepted policy. Idempotent close cannot rearm or close a replacement binding accidentally |
| settle | The same private worker supplies a proven committed/rolled-back disposition for the exact installed claim. Unknown, dropped waiter, deadline or interrupt retains the claim and unavailable fence; no replacement worker or automatic retry |
| commit_response | For a complete bounded response, recheck current binding/read authority and selected scope under the short owner cutoff. Commit only its exact buffer reference; perform transport I/O after unlocking. Old claim/proof possession grants no read permission |

A binding reference is owner-created before its runtime can start and begins
Unissued. Source registration can open it once only after the actual preparation/
admission joins succeed. Close also applies to Unissued and is terminal for that
reference: a delayed registration after exit/cancellation cannot resurrect it.
Runtime callbacks retain that exact reference, not only an ID lookup. Replacement
uses a different reference; failed registration never reopens the old one.
Run/workspace/namespace/sidecar scopes are shared owner-created close handles in
the same ordering, so closing a scope checks/changes bounded state rather than
walking every session or invoking callbacks under the latch. A claim/response
checks its exact binding and every enclosing scope.

These names describe private responsibilities, not a new wire API. Registration
has no production caller until the actual source owner and every closing path
are qualified. The first unit below uses explicitly synthetic test bindings.

### Identity, token and lifetime

At sidecar authority-owner construction, propose independent 32-byte OS randomness
via the existing getrandom primitive, encoded as the existing 64-character Digest
for sidecarGeneration. Failure closes hierarchy admission and leaves ordinary
operation available. Do not reuse a credential, runtime u64, PID, timestamp or
maximum retained proof. The existing hierarchy AuthorityGeneration producer must
use this same qualified sidecar identity; a second unrelated identity source is
not permitted. That producer integration remains unimplemented, not inferred.

Allocate the existing positive u64 ClaimIdentity.sequence only at successful
claim, using checked addition. A rolled-back claim consumes its number; replay
and queueing consume none. Exhaustion closes new claims for this sidecar and
never restarts or resets it. Claim proof serialization/hashing stays outside the
latch and uses the unchanged ClaimProofV2 codec.

Propose a non-Clone, non-serializable ClaimedEvaluation handle, constructible only
by the owner. It binds the exact immutable intent, claim identity, store/epoch,
worker position and retained lease ownership handle. It cannot authorize another
request, retry, source action or SQL attempt. The owner retains the installed
claim independently of the caller's handle: dropping a handle/waiter cannot free
the slot or certify rollback. Arc/resource destruction and notification callbacks
run outside the latch. A token's type/privacy is a software boundary, not proof
of authentic source history or native ownership.

## Concrete closing and lock integration

Use a two-phase close. First capture the exact immutable runtime/source handle
under the ordinary owning lock when necessary, release that lock, then close
that handle under the publication owner. The owner compares its own current
binding; a stale close cannot touch the replacement. Release the publication
latch before prompt-transition guards, metadata lanes, persistence, kill signals
or other cleanup. Do not acquire the latch while holding sessions/workspace or
prompt locks. No callback, allocation/canonicalization, SQL/fsync, channel send,
blocking core-lock acquisition or resource destruction runs inside it.

For a path requiring a combined projection/publication decision, preserve the
[accepted order](2026-10-24-hierarchy-authority-ordering-design.md#lock-and-caller-ordering):
projection first, publication latch next, then only permitted ordinary state
access, with no reverse acquisition. The proposed minimal implementation avoids
nested ordinary access under the latch: capture/release, compare/close, release,
then revalidate the same generation in the existing ordinary path. It does not
wrap the current prompt transition guard or lifecycle_current callback with the
publication latch. Ordinary lifecycle failures after a valid close leave that
binding closed; persistence failure never restores its old authority.

| Event / concrete integration root | Required close and later work |
| --- | --- |
| Explicit stop: delete_session | Capture exact live runtime/source ownership; close before its kill_tx signal. Preserve the existing generation check when continuing the ordinary stop. Never stop or revoke a replacement using an old snapshot |
| Natural exit, transport/server failure, ending: session_runtime and transition_session_status | Runtime owner closes its exact binding before scheduling terminal persistence/final scans. transition_session_status and clear_ended_session_tracking_for_runtime supply idempotent backstops; neither is sole coverage |
| Resume/replacement: session creation/resume ownership exchange | Close the old binding before a replacement can be registered. New runtime source admission is separate; no transferred claim, generation alias or automatic grant |
| Workspace adoption: open_workspace_with_identity | Prepare the new ordinary workspace outside the latch, close/detach the old hierarchy scope before replacement publication, retain the old claimed store/lease until its proven disposition. A failed ordinary adoption does not revive closed authority |
| Run cancellation/end and explicit finish | Future #742 owner must participate before acknowledging revocation. Cancellation closes immediately; explicit finish returns the existing bounded conflict while required claims/candidates remain unsettled. These owners are not supplied by the ordinary session status fields |
| Namespace change / sidecar shutdown | Close before rotation or shutdown admission proceeds. Rotation cannot activate a new epoch until the old claim settles; process replacement never resumes a volatile claim. Shutdown/kill is not rollback evidence |
| Health change / actual lease loss | Close/fence immediately; an existing claim gives no permission through actual lease loss. Complete semantic/physical disposition still needs the qualified adapter; a volatile last check is not that mechanism |

All response paths must use the same close ordering, including exact receipt
retries. Preserve intervening health/workspace fences during post-commit snapshot
invalidation/reference publication. A completed old-workspace command publishes
nothing into the new workspace and cannot clear its unavailable state.

### Why these choices

Reusing prompt_authority::transition_lock would inherit ordinary persistence and
unrelated prompt lock lifetimes; it is not the accepted no-I/O latch. Reusing a
session-store ticket would import a different execution/cancellation policy.
Holding a lock through SQL or waiting for the worker before stop violates the
accepted responsiveness rule. The separate private owner and two-phase close
keep the accepted claim policy while making each actual integration obligation
visible. This does not prove the future lock graph or native ordering.

## First bounded unit after scoped approval

Approve the owner design for a **non-I/O authority core and deterministic tests**
only. The subsequent concise execution plan must reuse this exact boundary:

1. Implement the private owner state machine, immutable synthetic-binding test
   seam, checked identity/sequence and opaque exact-claim handle in Rust. No
   AppState field, production registration, ordinary lifecycle hook or runtime
   caller is added in this first unit.
2. Exercise claim/revoke, source-binding replacement, close/response and exact
   settlement using deterministic barriers and injected readiness/disposition.
   The fake backend may park one operation; it is not SQL/native evidence.
3. Verify no production caller or storage bypass, run focused tests plus required
   repository checks and current-diff manual review, and record exactly the
   ordering evidence supplied. Keep the production opener fenced.

This isolates an independently checkable authority invariant before changing
ordinary lifecycle code. It is neither another source codec nor a source-valid
publication test. Actual lifecycle adoption is a later coupled implementation
unit requiring a complete writer inventory, exact adapter plan, source/ordinary
bridge participation and all remaining catalog/resource/native gates. No unit
may issue a live capability or activate storage by accepting this design alone.

## Verification oracles and remaining risks

| Case | Required first-unit oracle; later integration obligation |
| --- | --- |
| Revoke before claim | No claim/token/sequence issuance; no backend invocation |
| Claim before revoke | Only that exact intent may settle; new claims and response commitment denied; the owner close primitive completes while the fake backend remains parked |
| Replacement between capture and close | Old close cannot revoke a new binding; a prepared old intent cannot claim against the replacement |
| Exit/close before source registration completes | Unissued binding closes terminally; delayed registration cannot activate it or issue a claim |
| Changed target/request/epoch or second consume | Deny without widening the installed claim or starting a second operation |
| Dropped waiter/token, timeout, interrupted/unknown backend | Installed claim stays charged and unavailable; no new claim or inferred rollback |
| Proven rollback then later valid admission | Prior sequence is not reused; new admission still requires current binding and all normal checks |
| Commit after workspace switch | Old retained context settles only; no new-workspace publication or old-authority restoration |
| Response committed before/after close | Before may deliver later; after is denied, with no network I/O under the cutoff |
| Randomness failure, poisoned state or u64 exhaustion | Close hierarchy admission, no weak identity/reset; ordinary controls stay independent |
| Actual lease loss / filesystem fault | First unit proves nothing physical. No runtime adoption until bounded native fault/lease/durability and complete old/new recovery evidence exists |

Risk checkpoint: dependency coverage 4 (actual source, ordinary lifecycle and
storage owners); blast radius 4 for later adoption but isolated for the first
unit; durable state changes none in the first unit, native state Unknown;
reversibility Unknown for later adoption; uncertainty 4 (native/source cutoffs
unqualified). Sum incomplete, not an approval threshold. Source inspection resolves
the mistaken assumption that an existing prompt/ticket/generation is a hierarchy
authority owner; complete lifecycle coverage, source binding truth, physical
lease/health ordering and memory/retirement behavior remain explicit evidence
gates. A parked old worker may retain its lease indefinitely under the already
accepted policy; there is no bounded retirement or automatic kill guarantee.

## Scoped decision requested

The author's proposed choices are: one separate private owner; two-phase exact
close before ordinary prompt/metadata work; independently generated sidecar
identity plus checked sequence under existing codecs; and an opaque single-intent
claim handle with disposition-owned lifetime. Accept those choices **and the
non-I/O first-unit boundary** before preparing its execution plan. This requests
no approval to wire lifecycle handlers, create a store, implement a generic SQL
publisher, run a native/model probe or qualify the complete #1176 profile.


## Bounded builder review

One independent read-only Luna builder check reported **nothing important left**
before planning the explicit non-I/O unit, with no factual or accepted-policy
contradiction. It checked the actual delete/ending prompt-lock seam, accepted
claim/sequence semantics and deferred adoption boundaries. The author clarified
that the parked-backend oracle concerns the owner close primitive, not proof of
actual PTY stop latency. No new decision arose from that clarification. Source/
native adoption and the four implementation choices remain unaccepted until the
scoped owner decision above; this review is not their approval or qualification.


## Scoped owner acceptance and first-unit execution — 2026-11-04

After reviewed PR #1372 (head 67630684b00e514f2e8e6dc39cb8416138707d42,
merged ae417f10f461f6fec5c0e11a7ef2b542a59a08fb), the owner answered “lets keep
goin” to the concrete request approving this design and its first non-I/O unit.
This accepts the separate private owner, two-phase exact close, fresh sidecar
identity/checked sequence and opaque single-operation claim for the described
unit. The [execution plan](../plans/2026-11-04-evaluation-authority-core.md), tracked
by [#1376](https://github.com/Rambolarsen/orkworks/issues/1376), implements and tests
only in-memory ordering. Earlier owner-pending wording is superseded for these
choices and this bounded unit, not for actual lifecycle/source/native adoption.

No AppState field, production source registration, ordinary lifecycle hook,
SQL/store activation, serialization/limit/policy change or native/model probe is
approved. The #1356 admission fence remains; #1176 qualification stays open.


## Close-only lifecycle proposal — 2026-11-04

Tracking: [#1380](https://github.com/Rambolarsen/orkworks/issues/1380), under #1176.
Inspection base: `90bc6cb824cfe2125d235a2be406a7847ebb9c2d` (merged core PR #1379).
Disposition: **Proposed scope amendment; owner acceptance pending.** The earlier
first-unit approval covers the merged core, not the runtime changes below.

### Outcome and finite boundary

Make the ordinary lifecycle ready to revoke an exact future evaluation binding,
without granting that binding. Propose one coupled **dormant close-only plumbing**
unit: optional private revocation carriers in ordinary owning records, exact
capture/close/revalidation at every root below, and deterministic integration
tests using the existing synthetic test owner. All production carriers remain
None. No production constructor, attachment operation, owner initialization,
registration, readiness/response construction or claim/publication caller is
introduced. Runtime code cannot manufacture a binding from a session ID, an
eligibility flag, empty source bytes or persisted metadata.

This is deliberately less than active lifecycle adoption. It proposes permission
to add the optional carriers and close call sites only. The previously required
source/ordinary bridge, complete catalog/resource/physical/native and executable
activation gates remain prerequisites to any production attachment or opening.
A later qualified source owner must supply the actual immutable source/runtime
join and sole sidecar owner; synthetic integration tests cannot satisfy it.
No database, capability, model/native probe or complete profile is enabled.

### Source-backed root inventory

Links point to the inspected source; names identify roots when line numbers move.
The inventory covers ordinary sidecar lifetime roots for this proposed unit,
not desktop/native process-ownership proof or future hierarchy writers.

| Root | Observed behavior and integration obligation |
| --- | --- |
| [delete_session](../../../crates/orkworksd/src/session_application.rs) | Sends kill while projection/workspace/sessions guards remain in scope, then schedules generation-bound killed status. Close must precede that signal, outside workspace/sessions guards |
| [session_action_with_timeout](../../../crates/orkworksd/src/session_actions.rs) | Active Close/Archive/Delete sends kill under exact row-token admission. Preserve its reservation/token and bounded wait; insert close before signal without widening a stale token |
| [send_runtime_command / TerminalAction::Kill](../../../crates/orkworksd/src/runtime/session_runtime.rs) | Terminal kill bypasses delete_session. Propagate the originating attachment RuntimeIdentity from spawn_command_future, then capture its matching carrier/sender, close, revalidate and signal; never re-resolve a replacement by session ID alone |
| [start_session_runtime](../../../crates/orkworksd/src/runtime/session_runtime.rs) | Pre-spawn cancellation/errors return before a driver exists; post-spawn abort kills/waits directly. Both close the captured Unissued carrier before error cleanup/kill; delayed registration must remain impossible |
| [runtime driver](../../../crates/orkworksd/src/runtime/session_runtime.rs) | Watch/queued Kill, native-observer failure, initial-editor failure and writer-task failure kill directly. Child-exit receipt precedes drain/report work and the eventual handle_runtime_exit callback. Close on first observed termination, before those waits; retain an idempotent callback backstop |
| [transition_session_status / clear_ended_session_tracking_for_runtime](../../../crates/orkworksd/src/session_application.rs) | Terminal transition acquires the global prompt-transition lock before sessions; cleanup invalidates diagnostics/reporting. These are backstops, not the first cutoff. Capture and close before entering those existing lock/wait sequences |
| [try_install_claimed_resume_handle / ResumeAdmission::drop](../../../crates/orkworksd/src/session_application.rs) | Replacement uses projection/workspace/sessions; pre-spawn cancellation can restore the prior handle. Close old before installation, close abandoned replacement before restoration/removal; restoring ordinary metadata/handle must not reopen its carrier |
| [create_session_workflow](../../../crates/orkworksd/src/session_application.rs) | Installs a runtime before creating-record persistence; persistence failure removes it. Close that exact carrier before removal; carry None on current ordinary creates/resumes |
| [open_workspace_with_identity](../../../crates/orkworksd/src/session_application.rs) | Prepares stores/lease, then replaces WorkspaceState under workspace lock and reconciles metadata. Close the old optional workspace carrier before publishing replacement; no path-only equality shortcut or silent reopening on same-path reload |
| [EndingPersistence / lifecycle_current](../../../crates/orkworksd/src/runtime/terminal_runtime/ending_persistence.rs) | JSON ending completion and dormant SQL completion are generation/workspace checked. SQL current-owner callbacks hold projection/prompt/workspace/sessions guards. Neither is a suitable first close point or hierarchy claim; do not wrap either callback with the latch |
| [sidecar serve](../../../crates/orkworksd/src/main.rs) and [WorkspaceLease](../../../crates/orkworksd/src/workspace_runtime.rs) | Serve currently has no graceful-shutdown hook; advisory lease unlock occurs at Drop. Do not claim a reliable shutdown/lease-loss callback. This unit leaves production hierarchy admission impossible; active adoption still needs those owners and recovery evidence |

### Proposed carrier and ordering choices

Keep implementation ownership in a small private facade beside
`taskmaster/publication_authority.rs`. Expose only opaque cloneable runtime and
workspace revocation handles plus idempotent close to ordinary code. They retain
exact core binding/scope references and their owner; no public constructor,
serializer, registry lookup, source eligibility or settlement method. Only
cfg(test) fixture construction/attachment exists in this unit. The core's
synthetic source bytes remain test data and are never promoted into a production
source representation. AppState need not gain or initialize an authority owner.

`SessionRuntime` holds an optional runtime carrier, and `WorkspaceState` an
optional workspace carrier, both None in every production construction path.
The startup/driver closures capture the exact runtime carrier before asynchronous
startup can lose registry ownership; the carrier outlives replacement/removal
as needed. A separate non-Clone runtime-task guard is constructed before spawning
or polling startup, then transferred to the driver on successful startup. Its
Drop closes only that exact carrier on task cancellation/unwind or early channel
teardown. Cloning/dropping the ordinary carrier never revokes authority. Arrange
its lifetime outside ordinary guard scopes, so task teardown cannot acquire the
publication latch while an ordinary guard remains held. It ignores close errors in Drop (poison remains fail-closed) and performs no kill,
I/O, settlement or callback. Explicit first-observation closes still precede
kills/drains; Drop is a cancellation backstop, not their substitute. Driver
kill-channel closure and select exhaustion must close before breaking.
No session-ID map, per-request allocation or historical close log.
With None there is no latch acquisition, entropy request or changed permission.
Additional production state is a fixed optional field per existing record;
test-attached handles remain bounded by the explicit fixture. Production memory
qualification for real attached bindings remains an activation gate.

For admitted user stop/replacement operations, retain the existing projection
serialization and organization reservation while making the two phases explicit:

1. Validate current admission and capture exact carrier, runtime identity and
   kill sender under the existing ordinary guards. Release workspace/sessions
   guards, retaining only the admission reservation/projection guard as needed.
2. Close the captured carrier under its owner latch. Release the latch before
   acquiring any ordinary guard, sending kill or performing persistence.
3. Revalidate the captured identity and admission under ordinary guards before
   signaling/exchanging the handle. A mismatch rejects the ordinary action and
   never signals or closes a newly resolved replacement. A failed ordinary tail
   leaves the old carrier closed; it cannot roll back revocation.

Projection may precede the latch under the accepted order; sessions/workspace/
prompt guards must not. Native driver/exit callbacks already own a captured
carrier and close it directly before kill/drain/metadata waits. Generation checks
on their ordinary tail remain intact. No parent/run authority is inferred from
RuntimeGeneration; existing run cancellation/finish and namespace/health owners
remain absent and explicitly outside production qualification.

Workspace adoption prepares fallible stores outside the close cutoff, then uses
projection serialization for its capture/close/revalidate/exchange tail. Recheck
an exact captured workspace instance/lease reference, not merely its path. Close
the old carrier outside workspace lock before the replacement becomes visible;
keep retired references alive until after all guards are released. A failed tail
may leave the ordinary old workspace selected with a closed hierarchy carrier;
there is no implicit rearm. Same-workspace reload is also replacement unless a
future accepted source-admission contract explicitly proves identity continuity.
No actual carrier is attached here, so no lease is newly retained by production.

If a test-attached owner is poisoned/unavailable, publication stays denied while
ordinary kill/cleanup remains available; close failure must not delay a kill
awaiting a worker or claim settlement. An already claimed operation retains its
existing exact disposition context; close/stop/drop is never rollback evidence.
Do not add graceful shutdown, automatic native cleanup or physical lease-loss
claims as incidental changes to this unit.

### Verification and decision

The [conditional execution plan](../plans/2026-11-04-evaluation-lifecycle-close-plumbing.md)
names the coupled integration oracles. It is ready for review of this proposed
scope, not execution authorization. Tests must exercise the actual ordinary
entry points with injected handles, not just repeat core tests. They prove
routing/order under deterministic scheduling, not real native stop latency.

Investigated uncertainty: a single delete/status hook misses direct action and
driver kills; post-exit finalization occurs after drain/report work; resume Drop
restores an ordinary handle; same-path workspace adoption is not a hierarchy
epoch proof. The inventory above resolves those assumptions. Full active source,
shutdown/native/lease and publication writers remain unqualified; this proposal
keeps them unreachable rather than certifying them by omission.

Owner decision requested: accept only these optional close-only carriers and
coupled ordinary call-site tests, with every production attachment/registration
still absent. This is a new narrow scope decision, not a request to reapprove
claim-first policy or the merged core. Keep #1176 open and the #1356 opener fenced.


### Bounded builder review of the close-only proposal

One fresh read-only Luna builder check found two material clarifications:
propagate the originating terminal attachment identity through delayed Kill, and
provide a separate non-Clone task guard for cancellation/unwind/early teardown.
Both are now explicit in the proposal and conditional tests. The reviewer also
checked actual workspace adoption: moving store preparation outside the current
workspace-locked tail, then projection-serialized capture/close/revalidation, is
feasible and preserves the existing admission order. No material builder question
or lock-order contradiction remains. This is written-design review, not runtime
or native verification. Disposition: **Ready for scoped owner decision.**


### Close-only scoped owner acceptance — 2026-11-04

After PR #1386 merged, the owner answered “go ahead” to the concrete close-only
inline implementation request. This accepts the optional exact carriers,
capture/close/revalidate tails, originating terminal identity, cancellation guard
and coupled tests described above. The approved plan and ADR 0090 amendment
record provenance; implementation is tracked by #1389. Earlier pending-owner
wording is superseded for this dormant unit only. No production attachment,
source/claim/publication, AppState initialization or storage activation follows.


Implementation refinement: ordinary store opening/recovery retains its existing
workspace writer fence during preparation, then releases it before projection
admission and the authority cutoff. RecommendationStore opening can migrate and
recover graph files; removing that fence could race same-path writes. The
execution plan records this source-backed lock-lifetime ruling. The no-ordinary-
locks-under-publication-latch contract and every activation gate are unchanged.
