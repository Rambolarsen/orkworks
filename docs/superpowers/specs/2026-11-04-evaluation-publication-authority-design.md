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
