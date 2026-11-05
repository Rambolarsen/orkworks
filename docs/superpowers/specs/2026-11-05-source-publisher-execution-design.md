---
type: Execution Design
title: Source publisher execution and qualification boundary
description: Proposed writer participation and exact registry ordering for the source publisher, reusing the accepted joint-preparation direction and preserving physical authority gates.
tags: [taskmaster, hierarchy, sources, sessions, qualification]
status: draft
workflow_status: owner-review-pending
---

# Source publisher execution and qualification boundary

## Outcome and decision boundary

[#1411](https://github.com/Rambolarsen/orkworks/issues/1411) continues
[#1176](https://github.com/Rambolarsen/orkworks/issues/1176). After reviewing
[#1398](https://github.com/Rambolarsen/orkworks/pull/1398), the owner said
“go ahead” to the recommendation accepting its interface/ordering choices and
writing this execution design. The [source-admission acceptance record](2026-11-05-evaluation-source-admission-design.md#scoped-owner-acceptance-for-further-design--2026-11-05)
records that limited scope.

A separate [joint original preparation proposal](2026-11-05-joint-child-preparation-design.md)
supplies the attached-transaction broker, lineage commitments, new profile and
retention consequences. Its [scoped architecture acceptance](2026-11-05-joint-child-preparation-design.md#scoped-architecture-acceptance--2026-11-05)
and [ADR 0106](../../adr/0106-joint-child-preparation.md) accept that direction for
further qualification. Reuse that work without asking for the same decision
again; implementation and physical-profile proof remain gated.

This design adds one proposed mechanism: retained immutable live-registry roots
for atomic affected-binding discovery versus registration. It maps actual writers
to the required source transitions and names the conditional execution sequence.
It does not claim an eligible publisher or an executable adapter plan. No product
code, database/SQL invocation, migration, native/model experiment or activation
is authorized. The source-command physical authority cutoff remains unresolved.

## Pinned source and writer inventory

Source inspection base: `dbf9bd7ecd29bf85cacfa52e869632b9ff087dd5`.
Decision/status reconciliation base: `4f4f63e2154a2c0343b5ba0def67f3f9d4163c89`;
its incoming change is documentation only. “Required publisher” below names future responsibility, not an existing endpoint.

| Actual seam | Established behavior | Required publisher participation |
| --- | --- | --- |
| [child_sources.rs](../../../crates/orkworksd/src/taskmaster/child_sources.rs), validate_unverified_child_sources_v1; [root_bootstrap.rs](../../../crates/orkworksd/src/taskmaster/root_bootstrap.rs) | Pure validation of supplied bytes; explicitly unverified-source-bytes-only | Source owner must resolve original retained source/proofs and complete membership; no validator-to-admission conversion |
| [CoordinatorStore](../../../crates/orkworksd/src/taskmaster/coordinator_store.rs), put_proposed/activate/transition/resume/recover | Persists current JSON coordination plan/status/approval history, with no runtime authority | Actual #741/#742 proposal, approval, run-control and recovery owners need closed source commands and their own current-authority cutoff; copying these JSON records is insufficient |
| [create_session_workflow](../../../crates/orkworksd/src/session_application.rs) | Installs an ordinary handle and persists a JSON creating record before startup | Joint publisher must establish original ordinary lineage plus complete hierarchy attachment/preparation before any handle/startup publication; do not wrap public create_session and append hierarchy writes |
| [SessionStore Database::insert](../../../crates/orkworksd/src/session_store.rs) and [worker](../../../crates/orkworksd/src/session_store/worker.rs) | Private ordinary SQL owner, its own transaction/revision/history/search work and bounded scheduling | Conditional joint broker must use owner-private transaction-scoped functions; public create cannot independently commit one participant |
| [resume_session_workflow / try_install_claimed_resume_handle / ResumeGuard](../../../crates/orkworksd/src/session_application.rs) | Exact ordinary replacement/rollback ownership | Close old exact evaluation carrier first; publish the actual current runtime/session bridge through its distinct event publisher, preserving original assignment/spawn history |
| [session_runtime](../../../crates/orkworksd/src/runtime/session_runtime.rs), start_session_runtime/handle_runtime_exit/send_runtime_kill | Creates ordinary RuntimeIdentity, captures close guard before polling, closes on exit/kill; production carriers remain None | Qualified native/ordinary owner must prove actual RuntimeAttachment and map the current ordinary identity to accepted RuntimeRef/AuthorityGeneration; attempt/PID/status is no proof |
| [transition_session_status / delete_session](../../../crates/orkworksd/src/session_application.rs), [EndingPersistence](../../../crates/orkworksd/src/runtime/terminal_runtime/ending_persistence.rs), [SQL lifecycle](../../../crates/orkworksd/src/session_application/sql_writes/lifecycle.rs) | Close-first ordinary controls and generation-bound ending persistence | Immediate close stays independent; later bridge publication has its own exact producerRevision/current-runtime/capacity outcome. Persistence failure cannot reopen authority |
| [open_workspace_with_identity](../../../crates/orkworksd/src/session_application.rs) / [WorkspaceLease](../../../crates/orkworksd/src/workspace_runtime.rs) | Exact directory/lease capture and ordinary adoption | Close old hierarchy scope before exchange, preserve one retiring position and exact retained resources; a new ordinary workspace cannot bypass an unresolved hierarchy worker |
| [forget_session](../../../crates/orkworksd/src/session_application.rs), [retention](../../../crates/orkworksd/src/runtime/retention.rs), [workspace_gc](../../../crates/orkworksd/src/runtime/workspace_gc.rs) | Ordinary evidence cleanup paths; none proves complete hierarchy custody | Every linked physical deletion needs the complete source-owned incoming-consumer decision. The accepted linked-profile direction protects retained lineage tombstones; physical purge still requires complete qualified joint coverage |
| [publication_authority](../../../crates/orkworksd/src/taskmaster/publication_authority.rs) / [lifecycle](../../../crates/orkworksd/src/taskmaster/publication_authority/lifecycle.rs) | Dormant claim/close/response state and close-only handles; no source registry constructor | Admission module owns retained ticket/fence/registry ordering, not durable truth. Existing synthetic test bindings cannot become production evidence |
| [hierarchy production opener](../../../crates/orkworksd/src/taskmaster/hierarchy_storage/connection.rs) | Refuses unqualified profile before I/O | Remains closed until all selected-profile gates pass |
| [mechanical catalog](../../../crates/orkworksd/src/taskmaster/hierarchy_storage/mechanics/schema.rs) / [journal mechanics](../../../crates/orkworksd/src/taskmaster/hierarchy_storage/mechanics/publication.rs) | Test-only 17-family fixture and individual journal operations | Neither complete 33-family format-4 nor the proposed 34-family format-5 profile, typed source commands or metadata-only physical membership is supplied |

The five existing [source-command responsibilities](2026-10-31-child-review-source-preparation-design.md#five-accepted-command-kinds)
remain distinct: propose, exact-plan approve, reserve, allocate intent/outcome and
attach/preparation. The [control-event inventory](2026-10-31-child-review-source-preparation-design.md#control-events-not-given-a-publisher-by-these-five-tags)
additionally requires root/profile preparation, parent attach/resume, standalone
input, spawn intent/outcome, ordinary ending/resume, coordination and pause/cancel/
continue/recovery. No generic journal, session status or one source tag publishes
these missing event representations. Label/attention/display updates confer no
quality or original preparation authority; changes to admitted facts still require
the corresponding source owner. Qualification must reconcile every actual write
call-site with this semantic inventory, including error/rollback/deletion paths.

## Proposed registry ordering

Source membership at store header H and the live admission registry are different
read sets. Complete durable discovery cannot prove that no new live binding
registered after that discovery. A fence identity alone is insufficient for that
race: successful registration need not advance the source fence.

Propose one owner-retained immutable RegistryRoot reference R containing the
bounded live binding/dependency registry. Entries contain exact owner-created
binding references and typed immutable dependencies. It is neither a durable
membership index nor a caller-supplied affected list. No new counter, bearer,
serialized codec or numerical allowance is introduced.

1. Capture the current source fence and R under the publication latch. Build
   complete source evidence, a candidate registry root and bounded closure plans
   outside the latch. Account every owned copy and retained old reference.
2. Registration compares its retained fence, R, selected scopes, exact immutable
   binding and current admitted source/runtime authority under the same latch.
   Only then install the prebuilt replacement root and transition that binding
   UNISSUED to OPEN together. Every insertion/removal replaces R; retained old
   roots prevent address reuse from masquerading as equality.
3. A source writer captures R and derives the complete affected binding references
   from that snapshot plus its exact typed event/dependencies outside the latch.
   At begin, compare both R and the retained source-fence identity. If either
   changed, deny this prepared transition before source effects; do not patch the
   list, silently retry or close only the stale subset.
4. When comparisons succeed, terminally close the source fence, invalidate all
   pending tickets, close every exact affected binding and install the exact
   SourceTransition identity under that one cutoff. No registration can appear
   between membership comparison and closure. Unknown membership closes the
   enclosing scope and leaves hierarchy mutation unavailable.
5. Immediate runtime/run/workspace close uses its already retained exact handle
   or scope and needs no discovery or registry-root retry. Closed entries may
   remain retained conservatively; removing them replaces R. Neither pruning a
   registry entry nor creating a fresh root reopens a terminal binding.

All replacement roots and closure vectors are allocated before locking. Under
the latch use only bounded prevalidated reference/scalar work; retire old roots,
buffers and last-reference destruction after unlocking. The future profile must
prove bounds on the registry, traversal work, retained roots/tickets/transitions
and allocator copies under existing limits. Failure to fit denies admission;
this mechanism supplies no additional memory pool or claim that maxima fit.

Use the accepted projection-before-publication order wherever projection admission
participates. No source/session/workspace/prompt lock is acquired under the
publication latch; no latch or core guard spans SQL, fsync or native work.

## Source execution and exact disposition

The proposed child_source_publisher is the domain planner. Each closed event
owner derives its own original identities, authenticated authority, complete
read set and effect set. It cannot accept arbitrary keys, SQL, booleans, absence
claims or supplied dependency lists. SourceTransition remains non-Clone and is
not a source-write permit. Only the qualified storage/native owner may construct
the sealed proven-commit/proven-rollback disposition for that exact transition.

| Stage | Required result before advancing |
| --- | --- |
| Prepare | Capture exact selected workspace/database epochs, hierarchy H, complete source/control/consumer/proof/ledger reads, current runtime/source ownership and R. For a joint event also capture ordinary epoch/history generation G and its exact creation/lineage reads |
| Begin | Apply the registry/fence compare-and-close above before dependent source/native effects. No source fact changes on a denied begin |
| Execute | Use the sole admitted storage position and owner-private typed operation. Revalidate H, paired G when applicable, and the complete read set at the transaction boundary. Actual source authority must hold at the separately qualified physical commit cutoff |
| Publish disposition | Resolve complete old/new outcome through that same qualified owner. Preserve newer invalidations, scopes and unknown fences. A stale completion records only its own exact outcome; it cannot make a later transition available |
| Admission | After proven preparation and actual runtime/bridge outcome, obtain a fresh ticket and rederive all current evidence. No settlement, receipt, history row or old binding restores a capability |

Overlapping invalidations are not independent callbacks that each clear a busy
flag. A later begin supersedes earlier reopening rights; fresh admission requires
one complete supported reconciliation of relevant outcomes. Unknown/drop/deadline
keeps admission unavailable and retains required worker/lease state. An already
claimed evaluation keeps only its accepted exact settlement right; it cannot
write these source/control events or bypass changed source eligibility.

## Conditional joint preparation and physical membership

Under the [scoped joint architecture acceptance](2026-11-05-joint-child-preparation-design.md#scoped-architecture-acceptance--2026-11-05),
use its ordinary-worker transaction broker as the candidate original-attach
executor for further qualification. The ordinary main database and exactly one hierarchy peer use one
persistent rollback transaction. SQLite's [ATTACH contract](https://www.sqlite.org/lang_attach.html)
and [multi-file commit mechanism](https://www.sqlite.org/atomiccommit.html#multi_file_commit)
support evaluating that candidate outside WAL; they do not qualify this build,
VFS, filesystem or source-authority ordering. No SQL experiment was run here.

The broker must include ordinary normalized creating metadata/history/search,
its immutable lineage member/index, the complete hierarchy attachment/source/
prepared bridge/launch/summary/five-assignment-ledger effects, direct memberships,
quota effects, header and owning proof in one outcome. The ten run/workspace
peers already exist and reconcile; unchanged peers keep their original links.
No handle/startup work precedes proven joint preparation. Exact retry resolves
the original complete outcome and never initializes rows or issues a grant again.

Reuse the accepted direction's exact schema-2 ordinary lineage and new-store-only
format-5 commitment rules, including the preparationCommit exclusion and separate
proof validation. Do not implement another ordinary member, hash domain, format
variant, application decision journal or post-commit registration in this design.
ADR 0106 narrowly supersedes the old linked-profile no-ATTACH/worker direction.
Unlinked/legacy profiles remain unchanged; no existing file upgrade, qualified
linked opener or executable publisher follows from that architecture decision.

Required physical membership qualification is explicit:

- Reconcile the entire selected catalog: accepted 33 families, plus the
  ordinary_child_lineage projection only for the accepted format-5 direction.
  Close every family/key/owner/retention expansion before enabling any publisher.
- Provide exact forward/reverse/direct metadata lookups at one coherent H/G pair.
  Returned occurrences must resolve their owning row/projection and original
  proof; absence/completeness cannot come from a current-head scan or digest.
- Maintain the ordinary lineage projection and both directions of every hierarchy
  membership in the same original transaction. Recovery must prove complete
  correspondence, including historical references and protected tombstones.
- Preserve the content/precharge boundary: discovery uses qualified committed
  metadata projections, not uncharged artifact copying/hashing. A projection that
  cannot be validated without content access is not an eligible discovery path.
- Charge cumulative 4,096 observations / 16 MiB discovery work, repeated copies,
  independent complete-command limits and every smaller owning pool. Exhaustion
  without proving completion is unavailable, never a partial consumer set.

These are physical acceptance obligations, not a completed index schema or
qualification result. The partial mechanical catalog cannot be extended with
empty table names and presented as their implementation.

## Resources, recovery and the blocking native condition

Reuse #1406's explicit linked-profile costs: ordinary 32 waiting mutations/eight
reads/32 MiB admitted bytes/1 MiB intent limit and hierarchy one active-or-retiring
position/eight waiting commands, with the lower actual byte limits. One job cannot
hide its plan behind a reference to evade either admission. Shared-worker SQL
availability and protected linked tombstones are costs explicitly accepted in
the joint architecture decision, separately from #1398. Their physical behavior
and resource enforcement remain unqualified.

The linked proposal keeps hierarchy 4,096-byte pages / 1 GiB file ceiling, a shared
2 GiB rollback/auxiliary ceiling including both databases' journals and the
super-journal, and separate 8 MiB SQL scratch. Prove actual native amplification,
paired recovery, old/new workspace ownership and all retained registry/transition
copies fit; do not grant a second allowance per file, queue or old root. Ordinary
backup/cutover/producer gates remain prerequisites; production JSON is ineligible.

A proven joint commit with lost response retains its complete original ordinary/
hierarchy proof. Unknown or one-sided state is unavailable until supported owner
recovery; no partial repair, empty-store recreation, auto-spawn or authority
restoration. Retain exact old resources through unresolved I/O; ordinary input/
close/kill remain independent and a new workspace cannot start a replacement
hierarchy worker. Linked ordinary SQL latency may degrade as #1406 discloses.

**Blocking condition:** no qualified physical source-authority cutoff is supplied
by this design or by the joint broker. The five source commands and ordinary
bridge/control writes remain excluded from evaluation claims and debit permits.
A before-COMMIT callback, retained fence, SQLite hook, advisory lease or successful
atomic transaction is insufficient evidence that source authority remained valid
at its actual physical acceptance point. Do not introduce a custom VFS or broaden
the claim policy as an implementation shortcut.

Before an executable publisher plan, name and qualify the actual native commit/
revocation mechanism under the accepted strict source policy, with ordinary stop
independent of blocked I/O. If that mechanism cannot be demonstrated, the current
profile remains no-go; a policy change would require a separate explicit owner
choice. No native test is authorized merely by naming this requirement. The separate
[original-preparation authority investigation](../../validation/child-preparation-commit-authority.md)
describes an owner-pending one-command permission alternative. This design does
not adopt it. Even accepting that alternative would not cover spawn/runtime
bridge or other source writers; their separate cutoff obligations remain.

## Verification and finite handoff

| Case | Required oracle |
| --- | --- |
| Registration commits after an affected-set snapshot but before source begin | R differs; stale begin has no source effect, and cannot leave that new dependent binding open through an allowed mutation |
| Source begin wins before registration | Old ticket/fence fails; no OPEN or grant, even after a proven rollback |
| Exact runtime closes during discovery, startup or before registration | Terminal carrier remains closed; replacement uses a distinct association and full admission |
| Later source invalidation wins before an old completion | Old completion cannot clear the new fence, publish a fresh ticket or reopen a binding |
| Unknown membership, invalid index, unsupported event or source/read-set drift | No mutation/admission through partial evidence; retain supported original disposition only |
| Joint commit interrupted or response lost | Complete old/new ordinary and hierarchy state under qualified recovery; exact original retry without duplicate rows/charges/grants |
| Ordinary lifecycle/deletion, stalled I/O or workspace replacement | Immediate exact close/kill stays independent; linked lineage/custody and retirement rules hold without a replacement hierarchy worker |
| Registry/catalog/command/journal/scratch budget exhaustion | Deny before excess work/copy/publication; no new allowance, silent truncation or max-profile claim |

The next bounded work is to resolve the pending source-permission decision in
that investigation and close the corresponding authority/native evidence,
alongside the selected catalog/paired-membership specification. Only their accepted, evidence-backed outcome
can support a coupled original-publisher implementation plan. A generic journal,
synthetic admitted witness or standalone eligibility flag is not an interim unit.

Scoped new decision requested: accept the retained RegistryRoot compare-and-close
ordering for further design/qualification. The accepted broker/schema/format/
retention direction requires no repeat decision. This document's disposition is
**Investigate before implementation**; it supplies the writer integration and
registry mechanism, not the missing native/profile proof. #1176 remains open.


## Bounded review and validation

One read-only lower-cost round used independent correctness and builder questions.
Both reviewed substantive artifact `57a1f02d7783ae9159335f97bf636f7a6cab8b07`.
Correctness review found no material contradiction in the registry race ordering,
source/current-authority exclusions, paired H/G reads, accounting or approval
scope. The builder reported nothing important left for this scoped qualification
handoff; the physical/profile questions are explicitly gated, not delegated as
ordinary implementation guesses. Root owns the final text and checks; reviewers
ran no tests or build. This evidence-only record follows the reviewed content.

Node 22 frozen docs installation, docs metadata/index/link checks, 1,202 docs
tests and the site build passed. Whitespace, documentation and worktree checks
passed. Existing Node color and VitePress chunk warnings do not change the
proposal's source/installer claims. No Rust, SQL, model or native result is claimed.
These reviews and publication do not accept the new registry mechanism or
establish the missing physical source cutoff. During rebase, root reconciled
incoming PR #1412's architecture acceptance and separate authority investigation;
this changes decision status/routing, not the reviewed RegistryRoot mechanism.
No additional physical qualification or review round is claimed.


## Preparation policy accepted; representation handoff — 2026-11-05

The owner subsequently accepted the [one-original-preparation persistence exception](../../validation/child-preparation-commit-authority.md#scoped-original-preparation-policy-acceptance--2026-11-05)
for further design/qualification. Earlier pending-source-policy wording above is
historical for that exception. It does not extend to spawn/runtime bridge or
other source writers, whose current-authority obligations remain unqualified.

[#1415](https://github.com/Rambolarsen/orkworks/issues/1415) owns its exact evidence,
commitment/lookup, accounting/retention and compatibility proposal. No interim
synthetic permit or callback proof is authorized. The RegistryRoot mechanism
remains owner-pending; complete source/profile/native/resource gates remain.
No implementation, source registration or experiment follows from this record.


## Authorization-evidence representation proposed — 2026-11-05

[ADR 0107](../../adr/0107-original-child-preparation-permit.md) records the accepted scoped policy.
The [authorization-evidence proposal](2026-11-05-child-preparation-authorization-design.md)
for #1415 adds original control snapshots, one immutable P-keyed recovery row
and new-store-only format-6 commitments under existing source budgets. These
representation choices remain owner-pending. The accepted broker/schema-2/format-5
architecture and source-admission interface are unchanged; RegistryRoot remains
a separate proposal. No fixture, SQL/schema/migration, native/model experiment,
live source issuance or runtime activation is authorized.


## Physical-index acceptance and source-owner handoff — 2026-11-05

The [authorization representation](2026-11-05-child-preparation-authorization-design.md#scoped-representation-acceptance--2026-11-05)
and [physical membership/index design](2026-11-05-paired-membership-index-design.md#scoped-physical-design-acceptance--2026-11-05)
now have scoped owner acceptance for further qualification. Earlier format5/
representation/index-pending wording above is historical for this selected next
format6/schema2 direction: the full 35-family logical inventory is reconciled and
the exact derived physical layout is selected. Neither is an implemented profile,
actual producer, feasible maximum or native/recovery qualification result.

Reuse the accepted original-preparation-only late-save/no-spawn policy and its
exact authorization row; the stricter source-authority conditions above still
apply to other commands. Complete source/H/G/read-set/lease/health/resource checks
remain required. Existing source-admission and broker/lineage/tombstone decisions
need no repeat approval. No product code, SQL/database/schema/migration, fixture,
executable/native plan or live capability is authorized.

The retained RegistryRoot compare-and-close mechanism above is still separately
proposed. Its purpose is precise: a new live binding registered after affected-set
discovery must cause stale source begin to deny, rather than remain open through
an allowed source mutation. Every insertion/removal replaces the retained root;
begin compares it and the source fence under the same short latch. It adds no
counter/bearer/serialized codec/allowance, but may conservatively deny after any
registry change and must retain/budget old roots/copies. Immediate close uses its
exact handle/scope without waiting for that discovery. This mechanism does not
supply a source physical-commit cutoff, actual writer or membership index.

That existing reviewed mechanism is the remaining scoped design decision here;
this status update neither accepts it nor requests a new version. After its own
decision, actual closed initializer/source/control/ordinary event ownership and
bounded native/resource/recovery evidence still precede a coupled executable
publisher plan. #1176 remains open, #740 closed research. Tracking: [#1431](https://github.com/Rambolarsen/orkworks/issues/1431).
