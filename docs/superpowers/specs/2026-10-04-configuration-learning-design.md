---
type: "Design"
title: "Repository-scoped configuration learning and skill improvements"
description: "Design history: Repository-scoped configuration learning and skill improvements."
tags: ["orkworks", "design"]
---

# Repository-scoped configuration learning and skill improvements

- Status: repository-identity contract accepted; learning/history contract proposed for review; runtime and platform evidence gated
- Date: 2026-10-10
- Tracking: [#745](https://github.com/Rambolarsen/orkworks/issues/745), initiative [#738](https://github.com/Rambolarsen/orkworks/issues/738)
- Consumers: [role configuration](2026-10-04-agent-role-configuration-design.md), [ordinary-child admission](https://github.com/Rambolarsen/orkworks/blob/main/docs/superpowers/plans/2026-09-26-taskmaster-orchestrated-child-sessions.md)
- Product direction: [hierarchy and configuration learning](2026-10-04-agent-hierarchy-and-configuration-learning-design.md)

## Scope and status

The accepted first portion defines one local repository identity across its
Git worktrees. The proposal below specifies #745's remaining repository-scoped
learning and history contract on top of that identity, consuming the versioned usage and
evaluation records defined by #743/#744. It does not authorize runtime
implementation: #743/#744 and their upstream dependencies still require their
own review and reconciliation, and the identity substrate remains separately
gated by #810 and platform evidence.

The ordinary-child scope is accepted under [ADR 0077](../../adr/0077-taskmaster-orchestrated-child-sessions.md), and the identity/persistence contract is accepted in [ADR 0080](../../adr/0080-repository-identity-across-worktrees.md).
There is no runtime implementation, new endpoint, eligible adapter or approved
code execution plan. The scoped registry substrate is tracked by
[#810](https://github.com/Rambolarsen/orkworks/issues/810); its exact plan and
native identity/durable-publication evidence remain gates before source changes.

## Investigated uncertainty and alternatives

**Least confidence:** existing `GitContext.repo_root` names a worktree, not a
repository shared by worktrees. At base `0c6562a1`, `git.rs::detect` and
`worktree_root` use `Repository::discover`/`workdir`. `taskmaster/context.rs`
collects facts beneath the canonical workspace without a durable repository
identity. `plan_handoff.rs` compares a common directory for plan-file access;
that advisory path check is not a registration or child-admission receipt.
Do not strengthen its production semantics through this documentation change.

**Project blind spot:** a directory name can survive removal/replacement of
the repository beneath it. A remote URL, branch, shared object database or
matching commit is also insufficient to identify a local worktree family.
Keep family identity, workspace ownership, worktree ownership and artifact
revision separate. In particular, same-repository membership grants no right
to adopt an existing branch/worktree or read arbitrary files.

Three alternatives were considered:

| Approach | Tradeoff | Disposition |
| --- | --- | --- |
| Remote URL or commit identity | Easy labels, but combines separate clones/forks and may expose embedded credentials | Reject as identity input; never collect URLs for this contract |
| Hash of canonical common-directory path | Shares worktrees, but replacement at the same path reuses the history key | Retain the path as a locator, not the complete identity |
| Local random registration bound to canonical common directory and filesystem object identity | Needs bounded local persistence and platform evidence; detects observable replacement without writing to Git metadata | Accepted contract; platform evidence pending |

Git defines private worktree administration and a shared common directory.
Resolve this through Git's repository interface, not by removing a
`worktrees/<name>` suffix. See the primary [worktree documentation](https://git-scm.com/docs/git-worktree)
and [repository layout](https://git-scm.com/docs/gitrepository-layout).
Those sources establish layout semantics only, not OrkWorks authorization.

## Meaning of repository and workspace identity

A repository is one locally resolved Git common-directory object at one
canonical locator, enrolled in one installation-local registry epoch.
Its primary checkout and linked worktrees share `repositoryId`. Separate
clones remain separate even if their remotes, commits or alternates match.
A submodule or nested repository has its own common directory and identity;
never fall back to the enclosing repository to satisfy a binding.

`workspaceId` remains the selected workspace's existing path-derived metadata
identity and lease. Children in sibling worktrees continue using that selected
workspace sidecar/store. Sharing `repositoryId` neither changes their
`workspaceId` nor opens another sidecar, peers with another instance or grants
cross-workspace control. Future repository-local advice may compare eligible
records bearing this identity, but this draft introduces no history read or
pooling interface. A repository match alone is never sufficient comparison
cohort eligibility.

V1 supports discoverable non-bare working trees, including a valid separate
Git directory and a working tree backed by a bare common repository. A bare
path itself and non-Git directories cannot start an orchestrated run under
this contract. Ordinary sessions retain their existing behavior. Unborn HEAD
can resolve an identity; clean-base/commit eligibility remains #610's separate
admission check. Submodules can identify their own repository but this does
not establish support for allocating child worktrees from that layout.

## Logical records and digest

Top-level registration/binding records use schema version 1; all records reject duplicate JSON keys/unknown fields,
and use the strict integer/serialization rules in #741. No secrets, remotes,
Git configuration contents, environment values or transcripts are retained.

| Record | Fields |
| --- | --- |
| `RepositoryRegistration` | `schemaVersion`, `registryEpoch`, `repositoryId`, `canonicalCommonDirectory`, `commonDirectoryObject`, `state` (`active` or `retired`) |
| `RepositoryBinding` | Same identity fields as the registration, excluding `state`, plus `bindingDigest` |
| `PathIdentity` | `encoding` (`unix-bytes` or `windows-utf16le`), `base64`; exact canonical absolute native path units, never lossy display text |
| `DirectoryObjectIdentity` | `platform`, `mechanismVersion`, `volumeId`, `fileId`; opaque bounded identifiers from an opened directory handle, not agent-supplied strings |
| `ResolvedWorktree` | `repositoryBinding`, `canonicalRoot`, `rootObject`, `canonicalPrivateGitDirectory`, `privateGitDirectoryObject`, `kind` (`primary` or `linked`), `worktreeBindingDigest` |

`registryEpoch` is 16 OS-random bytes encoded as 32 lowercase hex characters.
`repositoryId` is `repo-` plus another 16 OS-random bytes in lowercase hex.
IDs are generated only by registration; collision or randomness failure fails
closed. The epoch scopes IDs to this registry; equal IDs under a different
epoch do not match. A binding contains all identity fields, so consumers never
resolve an ID to whatever directory happens to hold it later.

`bindingDigest` is SHA-256 of `orkworks.repository-binding.v1` followed by one
LF and recursively key-sorted compact JSON of the binding without that field.
Use #741's serializer, exact path encoding and base64 with standard alphabet
and required padding. No path case folding, Unicode normalization, slash
rewriting or display-text round trip participates in this digest. A platform
resolver must give canonical handle-backed paths and object identifiers;
Windows aliases/case and Unix symlinks need platform fixtures. Reject an
unrepresentable or unverifiable path rather than substituting a hash of a
lossy string. Human display paths are derived separately and confer no authority.

Object identity uses opened-handle native identifiers (Unix device/inode and
Windows volume/file ID) under a versioned platform adapter. These mechanisms are not verified by the
Git layout probe below. Eligibility requires evidence that the selected local
filesystem/platform can supply stable directory identities and canonical
locators. Unsupported filesystems, inconsistent aliases or missing identity
capabilities produce `identity_unsupported`, never a path-only fallback.
Object IDs can be reused after deletion and are not cryptographic repository
incarnations. This contract detects observed changes; it cannot prove that an
unobserved delete/recreate with reused identifiers or same-user tampering never
occurred. Suspected replacement requires explicit retirement/new registration.
Native provenance/confinement claims remain excluded by ADR 0077.

### Synthetic binding-byte fixture

For the exact compact JSON below (one displayed line, no trailing newline),
the canonical object is 308 UTF-8 bytes. The binding digest is
`9031986d5ccb5052a9cc5c654980fd20c5d1f1cf0b5d03a34143705f04f21e74`.
The prefix adds one LF before those bytes. The decoded locator is
`/fixture/main/.git`. IDs and object values are synthetic and authorize no
filesystem operation; the fixture is serializer evidence, not enrollment.

```json
{"canonicalCommonDirectory":{"base64":"L2ZpeHR1cmUvbWFpbi8uZ2l0","encoding":"unix-bytes"},"commonDirectoryObject":{"fileId":"42","mechanismVersion":1,"platform":"unix","volumeId":"7"},"registryEpoch":"00000000000000000000000000000000","repositoryId":"repo-11111111111111111111111111111111","schemaVersion":1}
```

## Resolution and registration

The identity module owns discovery, canonicalization, registration and fresh
binding validation. Consumers use `resolveExisting(cwd)` and
`revalidate(expectedBinding, cwd)`; UI-authorized run preparation additionally
uses `enroll(cwd)`. These are proposed internal operations, not HTTP endpoints.

1. Discover from the explicitly supplied existing cwd. Ignore inherited Git
   routing/configuration variables. Prefer an explicit library repository
   handle; a CLI resolver needs a verified sanitized environment and no shell
   interpolation. Do not trust agent-reported roots, `.git` suffixes, cwd cache
   entries or renderer-supplied repository IDs.
2. Require a non-bare workdir; canonicalize/open the root, private Git directory
   and Git-reported common directory. Verify they describe the same discovered
   repository. A linked worktree must have consistent private/common linkage
   and registration; a `.git` file alone is not evidence of linked status.
   Parse/query Git's real layout, including separate Git directories. Never
   infer the primary checkout by taking the common directory's parent.
3. Acquire supported native directory-object identities. Missing, inaccessible,
   invalid, oversized, unsupported or inconsistent discovery returns an explicit
   failure. Do not scan parent/sibling repositories looking for a convenient match.
4. For resolution, look up one active registration matching both the canonical
   common-directory locator and object identity. Missing registration returns
   `identity_unregistered`; duplicates/corruption return `identity_ambiguous`.
   An ID with mismatched fields returns `identity_drift`. If the same native
   common-directory object is active at another canonical locator, return
   `identity_drift` with that exact registration reference instead of treating
   the relocated family as merely unregistered.
5. Only Electron-authorized run preparation may enroll an unregistered family
   after displaying its local repository locator. Enrollment writes OrkWorks
   metadata only; it grants no permission to create worktrees or launch children.
   Under the registry lock, repeat the lookup and object checks across all active
   registrations before atomically persisting a new registration. A matching
   native object at another locator blocks enrollment pending explicit UI
   retirement of that exact old binding; never publish two active locators for
   the same object or silently retire a binding during enrollment. Two simultaneous
   enrollments of the same binding produce the same registration. Child/parent reports cannot enroll,
   replace, retire or select a historical ID.
6. If the locator is already registered with a different object identity, return
   `identity_drift`. The UI must explicitly retire the old binding before new
   enrollment; do not overwrite it or inherit its history. Retirement fences
   later revalidation even if the object subsequently reappears.

Symlink/junction aliases resolving to the same canonical locator and object
reuse the registration. A move that changes the canonical locator requires
explicit retirement of the old same-object registration, then a new registration
and new run. This rule also applies when moving back: the original retired ID
can never reactivate, and any current same-object registration must be retired
before enrolling the returned locator. A crash between retirement and enrollment
leaves the old binding retired; it does not restore it. Concurrent relocation
attempts serialize under the registry lock and cannot create duplicate active
object bindings. No history reassociation or approval transfer is provided.
A clone cannot adopt the old identity through matching remotes or contents.
A registry copy is data, not origin proof; supported restore and the limits of
direct copying are specified below.
Backing-directory replacement, stale linkage, relocation and conflicting
aliases pause admission and require a new reviewed binding. Git repair/move,
submodule initialization, fetch and filesystem creation are never resolution
side effects.

## Binding approval, allocation and resume

#741 gains required `repositoryBinding` and `sourceWorktreeBinding` in both
`AssignmentConfiguration` and `OrchestratorBootstrapConfiguration`; its
`repositoryId` must equal the repository binding's ID. Include the complete descriptor/digest in their canonical serialization,
2 MiB inclusive bounds and protected snapshot retention. It is also an immutable
input of the run definition and exact plan. Every child and `ParentPlanBinding`
references the same run repository binding digest; #742's input manifests do
not replace it. Mismatches block proposal/approval before any launch grant.
These identity fields are accepted by ADR 0080. The wider #741/#742 contracts,
consumer integration and platform eligibility remain subject to their own
reviews and gates.

`sourceWorktreeBinding` is the complete `ResolvedWorktree` snapshot of the run's
existing selected workspace repository root, not the parent's current reported
cwd and not a future child target. It includes the root/private-directory paths
and object identities as well as the matching repository binding. Its digest
is SHA-256 of `orkworks.worktree-binding.v1` plus one LF and #741-canonical JSON
of the complete snapshot without `worktreeBindingDigest`. Include this snapshot
in the run/bootstrap/child configuration and exact allocation descriptor; each
plan and `ParentPlanBinding` retains `sourceWorktreeBindingDigest`. The
repository fields must exactly equal the containing run's repository binding.
Replacement by another valid worktree in the same family therefore still
changes the source snapshot and blocks mutation/spawn/resume. A linked source
workspace is supported only when its own root/private linkage revalidates.


Revalidate from the run's bound existing workspace at run creation, plan approval,
before allocation intent, immediately before Git mutation, after materialization
and immediately before child spawn/resume. Match registration state, repository identity and the complete approved source
worktree snapshot on fresh discovery; do not use the five-second Git-context
display cache. The
already-live parent's bootstrap cannot be patched to a new repository identity:
fence its run and require explicit UI creation under a new reviewed bootstrap.

A planned child path does not exist yet and therefore has no `ResolvedWorktree`.
The exact approved allocation descriptor still binds its existing canonical
parent, final basename, base commit and branch/path collision rules under #610.
The allocator first revalidates the source repository and approved parent path,
records durable intent/reservation, then creates only the authorized target.
The existing target parent directory also has a path/object snapshot in the
approved allocation descriptor, rechecked before mutation; a non-Git sibling
parent is not mistaken for the source worktree binding.
After creation, persist the resolved root/private-directory object bindings in
the allocation receipt and require their common-directory binding to match the
approved repository. Resume must match that receipt as well as repository
identity, group ownership, capacity, grants, tool/profile and generation.

Same-family membership is insufficient ownership proof. Existing or foreign
worktrees stay unadoptable; retirement does not delete branches/worktrees or
kill children. A detected mismatch pauses new allocation/spawn/resume, retains
intent/artifacts and requires user disposition. A mutation already completed
before drift detection is an interrupted allocation, never silently retried or
removed. Git/filesystem races cannot be made into native confinement by fresh
checks: retain handles where supported, detect observable swaps, and document
remaining same-user/TOCTOU limits in the runtime plan. No atomic filesystem
identity guarantee is claimed.

## Bounded persistence, concurrency and recovery

Proposed registry: `~/.orkworks/repository-identities/v1/registry.json`, protected
by a retained `registry.lock` OS advisory lock. This installation-local mapping
contains no sessions, runtime handles, peer authority or instance ownership.
Each sidecar keeps its existing workspace lease. Lock only registry reads/writes;
never hold it across Git discovery, process launch, workspace locks or model calls.
Use pre-opened identity handles and bounded object rechecks under the lock.

| Limit | Proposed v1 value |
| --- | --- |
| Registry file, including epoch and active/retired records | 8 MiB |
| Registrations, including retired entries | 1,024 |
| Encoded registration | 8 KiB |
| Each native path before base64 | 4,096 bytes |
| Each object identity field | 128 ASCII bytes |
| Repository discovery/linkage traversal | 32 administrative links, 64 KiB total |
| One discovery/revalidation attempt | 5 seconds |
| Registry lock acquisition | 2 seconds |

Limits are inclusive and checked before publication; exhaustion blocks new
enrollment, never evicts or rebinds old identity. Existing valid registrations
remain usable when enrollment capacity is full. Unknown schema, duplicate IDs,
duplicate active locators/object bindings, invalid encodings, missing epoch or
oversized/malformed registry block identity operations with a bounded error.
Empty history does not relax launch identity requirements. I/O failure is not
absence. A published record must be durable before returning its binding;
use atomic replace plus platform-appropriate durable publication fixtures.
A crash before publication returns no new binding; a crash afterward resolves
the persisted record. Temporary/corrupt files are not automatically promoted.

Never delete the retained lock inode or break a lock by age. Metadata restart
loads the registry but confers no volatile run/plan authority. Missing registry
on first use can create a fresh OS-random epoch through authorized enrollment;
old bindings then fail validation, never migrate by matching paths. A corrupt
registry requires an explicit UI reset, not a silent empty-registry fallback.
Reset creates a new epoch and invalidates all former bindings; it must not
remove evidence or worktrees. Supported restoration of an archived/copied
registry uses this reset/new-epoch path and requires new reviewed bindings;
never replay old registration IDs or plan approval through a restore operation.
A manual byte-for-byte copy back onto the same installation with unchanged
paths/object IDs can be indistinguishable from the current registry. It can
also restore an earlier active state after retirement. Such direct filesystem
rollback is outside this protocol's guarantees: there is no authenticated
installation origin or rollback-proof counter in v1. Ordinary startup still
requires current fresh discovery and never restores volatile run bearers/grants.
Suspected rollback blocks use pending explicit UI reset; do not claim that
filesystem identifiers or the epoch alone prevent copied-state replay. UI reset/retirement must expose affected bindings
and warn that existing runs cannot resume under them. Active children continue
ordinary management; no new authority is inferred from their continued existence.

V1 keeps retired records within the same finite limit and offers no automatic
compaction, individual hard deletion or reassociation. Deleting a workspace
never deletes this shared identity mapping. The learning-history forgetting
contract below separately removes protected history without reusing identity
IDs. This early mapping cannot be used as an unbounded history store. Registry
reset is an explicit recovery path with new identity, not routine history
retention or cleanup.

## Repository identity and comparison cohorts

The learning store is installation-local and shared by worktrees that resolve
to the same active `(registryEpoch, repositoryId, bindingDigest)`. Store data is
kept separately from the identity registry at
`~/.orkworks/repositories/<registryEpoch>/<repositoryId>/learning/v1/`; the
registry remains an identity map, never a history database. A sidecar must
freshly resolve and revalidate the complete repository binding before reading
or writing this store. Retired, drifted, reset, missing, or mismatched
bindings cannot access the old history. A new registration never inherits an
old store by path, remote, or content similarity.

The shared repository store is the canonical owner of learning-family identity,
dedupe, owner binding, and dismissal watermarks; it does not duplicate full
Taskmaster recommendation records. Full `ImproveWorkflow` records remain in
their owning workspace's graph. Matching workspaces may receive a read-only,
redacted projection of a learning card.
That projection contains no foreign workspace ID, session ID, bearer, lifecycle
detail, or session-bound action. Only the owning workspace may expose the full
Taskmaster record or route accept, `Fix with AI`, and completion through its
own session-bound API. Dismissal updates the repository-shared watermark under
the shared store transaction from any matching active binding, through the
owner-mediated mutation protocol below. Workspace-specific observation
recommendations remain workspace-local. Retirement forbids normal history
reads, rebinding, and recommendation acceptance.
Electron may expose an exact-binding, deletion-only “forget retired repository
history” operation:
it releases only source dependencies named by exact workspace and subject
references under compare-and-swap, records `cleanup_pending`, and retries
idempotently after a crash. Retirement alone neither forgets nor transfers
history.

Repository family state and workspace Taskmaster graphs are separate stores;
their writes are never described as one graph transaction. Dismissal,
evidence invalidation, forgetting, source expiry, below-eligibility
supersession, and owner deletion use one bounded `pendingCardMutation` fence
per family. Under the repository lock, persist the operation ID, kind, typed
reason, expected family generation, owner binding, card ID and graph digest
before changing either store. That marker immediately suppresses the card from
all matching projections and makes its owner record unavailable to list/detail
actions and accept, dismiss, `Fix with AI`, and completion routes. Every owner
route acquires locks in repository-then-workspace order and rechecks the family
generation and pending marker before reading or mutating the graph.

Only the owning workspace API may mutate its Taskmaster graph. A dismissal
requested from a sibling stores the bounded pending intent; it does not write a
foreign graph. The owner sidecar reconciles pending intents after acquiring its
workspace lease, compare-and-swaps the expected record, and commits the typed
graph transition and immutable watermark/snapshot in its existing recoverable
graph transaction. The repository store then commits the matching watermark or
terminal digest and clears the fence. Evidence invalidation writes the source
fence, removes the contribution from current summaries, and installs fences for
every affected family in one repository transaction before the owner graph is
changed; only after each owner graph transaction commits may the repository
store clear that family's fence. If an owner sidecar is unavailable, the family
remains hidden and non-actionable until that owner resumes and recovery
completes. An accept reservation already in progress must resolve before a
competing mutation can transition the card; a rejected compare-and-swap clears
the pending intent with an explicit conflict and never records a dismissal or
supersession that did not occur. Recovery inspects the graph status and digest
to finish a committed transition idempotently. If either store is unavailable
or the expected graph cannot be reconciled, retain the fence and fail closed;
never expose a stale proposed card or report a partial mutation as complete.
Encode the marker as canonical CBOR schema v1 with fixed field order:
`[1, operation_id, kind_code, reason_code, phase_code, expected_generation,
owner_binding_digest, card_id_digest, graph_digest]`. `operation_id` is 16
random bytes. Encode `kind_code` as u8 (`0=dismiss`, `1=evidence_invalidation`,
`2=forget`, `3=source_expiry`, `4=eligibility_supersession`, `5=owner_loss`);
`reason_code` as u8 (`0=user_dismissed`, `1=evidence_invalidated`,
`2=learning_history_forgotten`, `3=source_expired`, `4=below_eligibility`,
`5=owner_workspace_deleted`); and `phase_code` as u8 (`0=fenced`,
`1=graph_committed`, `2=repository_committed`). Generation is an unsigned
64-bit integer. Each binding,
card, and graph digest is exactly 32 bytes of SHA-256. The authoritative owner
binding remains in the family state and must hash to `owner_binding_digest`;
the marker never carries a foreign workspace/session ID. Canonical CBOR
encoding uses definite-length arrays/byte strings and shortest-form integers
per RFC 8949 deterministic encoding, and is capped at 160 bytes including
schema and array overhead. Only these kind/reason pairs are valid: `dismiss` +
`user_dismissed`, `evidence_invalidation` + `evidence_invalidated`, `forget` +
`learning_history_forgotten`, `source_expiry` + `source_expired`,
`eligibility_supersession` + `below_eligibility`, and `owner_loss` +
`owner_workspace_deleted`. Reject every other pair before writing the fence.
The
serializer asserts that exact bound and the family state's 512-byte control
reservation before every write. Unknown enum/schema values or oversized
encodings fail closed.

As one step in the workspace deletion sequence specified below, each affected
owner card is processed only after the workspace deletion marker and
all-subject fences are durable. The owner-loss protocol records an
idempotent `owner_loss` `pendingCardMutation` by repository-store compare and
swap. The generic pending descriptor, including its bounded
operation ID, typed kind/reason, expected family generation, owner binding,
card ID, graph digest, and phase, serializes to at most 512 bytes; only one
mutation may be pending for a family at a time. Next, the owner's Taskmaster
graph transaction terminally supersedes the nonterminal workspace-local card
with reason `owner_workspace_deleted` and releases any executing reservation.
Before clearing the owner binding, the repository store commits the card's
bounded terminal evidence projection and digest under the same pending marker.
Only after that final commit may workspace metadata be deleted. Recovery
resumes from the durable
phase; if the graph transition already committed, its expected status/digest
makes the retry idempotent. Once the `owner_loss` pending marker is durable, the
deletion
request is committed and recovery proceeds forward; cancellation is permitted
only before that marker. If corruption or I/O failure prevents recovery, keep
the workspace metadata and pending marker, block deletion and card actions,
and expose a recovery error rather than clearing state or forcing deletion. The
typed transition applies to `proposed`,
`executing`, and `accepted` learning cards; it never rewrites `dismissed` or
`completed` history. A pending or unreconciled owner-loss state remains
non-actionable and cannot be adopted by another workspace. The shared finding
and dismissal watermark remain. A later eligible analysis in a surviving
workspace may create a fresh workspace-local successor bound only to that
workspace's current active session, with explicit predecessor lineage; it must
never adopt or replay the deleted workspace's target session.

Workspace deletion first writes a durable `pendingWorkspaceDeletion` marker
into workspace metadata under its workspace lease. The marker contains a
16-byte operation ID and phase; it blocks new source/learning writes and keeps
the metadata available for recovery. Under the repository lock, deletion then
publishes one atomic fence transaction covering every learning subject owned
by that exact workspace binding. First recover any already-pending subject or
family mutation; deletion never overwrites another operation's fence. The
workspace marker rejects new subject mutations until deletion finishes. Each
subject fence carries the operation ID,
workspace-binding digest, expected store generation, and phase in the existing
512-byte correction/fence slot; the complete transaction fits the reserved
512 KiB for at most 1,000 subjects. Repository summaries suppress all fenced
subjects. If the complete fence set cannot be published atomically, keep the
workspace metadata marker and expose no deletion success; no subject source
may be purged. A workspace with no learning subjects needs no repository
fence transaction.

After the all-subject fence commits, tombstone/remove each learning
contribution, recompute affected aggregates, and install each affected
family's `pendingCardMutation` before exposing any updated summary. Owner graph
transitions and terminal projections reconcile next. Only when these commits
finish does the workspace metadata marker advance to `learning_reconciled`.

Next release only source dependencies made unnecessary by that deletion. A
source still required by another current result, reviewer chain, or learning
family remains protected. The #744 custodian must move that source and its
dependency record to durable storage independent of the deleting workspace
metadata before that metadata can be removed; if it cannot, keep the deletion
fence and workspace metadata until the dependency is safely released. Then
purge eligible #743/#744 workspace sources and verify that no protected
dependency was removed before advancing to `sources_reconciled`. Keep each
subject fence until its source purge or protected-source custody transfer is
durably verified; then clear that subject fence. Only after both learning and
source phases are durable and every affected summary/card has reconciled may
the workspace deletion marker clear and metadata be deleted. Recovery resumes
the persisted phases idempotently; every earlier
phase keeps writes/summary reads fenced. This requires an explicit #743/#744
deletion-custody contract update before implementation; until then, workspace
deletion cannot claim atomic learning cleanup or remove its metadata.

This repository-scoped recommendation owner and its cross-workspace projection
are proposed contract changes, not behavior already provided by the current
workspace-local recommendation store. Before implementation, the Taskmaster
spec and storage/API design must explicitly adopt this ownership boundary and
its recovery protocol; until that prerequisite is accepted, repository-shared
recommendations are not implementable under the existing contract.

The repository store keeps a compact `LearningFamilyState`, not copies of
terminal Taskmaster cards. Each family state is at most 12 KiB, and a
repository binding supports at most 128 family states (1.5 MiB total) within
the 2 MiB normal-record budget below; that 1.5 MiB is not additive. A resurfacing
watermark retains exact subject IDs, evaluation digests, revisions, and `runIds`
for up to six represented subjects. Subject and run IDs are ASCII and at most
128 bytes, evaluation digests are 64 lowercase hex bytes, and revisions are
unsigned 64-bit integers. A watermark requiring more than six entries disables
resurfacing for that family and stores only its source-set digest; it is never
truncated. Each serialized subject entry is capped at 416 bytes, including JSON
field names and punctuation. A learning family may create at most four full
`ImproveWorkflow` records in its owning workspace graph over its lifetime,
counting proposed and every terminal outcome. At that lifetime cap,
retain the immutable records, seal the family against successor cards, and
preserve one bounded terminal entry per card in its family state. Each entry
contains a SHA-256 card-ID digest, terminal status/reason, and 64-byte
transition digest (encoded digest at most 256 bytes), plus the evidence
projection below. Never prune or rewrite Taskmaster history. The
lifetime count survives owner loss and workspace deletion. A terminal digest
may carry a privacy-safe `terminalEvidenceProjection` of at most 1,536
canonical CBOR bytes. The projection preserves the bounded typed finding
descriptor: descriptor version, target surface, logical target ID (at most 64
ASCII bytes), finding-class enum, stable criterion/dimension ID (at most 64
ASCII bytes), task-category token (at most 32 ASCII bytes), and up to four
sorted scope tags (each at most 32 ASCII bytes). It also stores the finding
fingerprint, criteria/rubric digests, terminal reason, source-set digest,
support/run counts, and up to three exact recurrence entries. Each entry holds
SHA-256 subject-ID and run-ID digests, subject revision, configuration and
evaluation digests, the #744 overall-result enum, and the applicable
criterion/dimension outcome enum. It contains no source payload,
prompt, secret, or workspace/session ID. This projection is captured from the
sealed Taskmaster snapshot before owner metadata deletion and remains
explanation-only; it never enters active learning. It makes the compact
terminal history inspectable after the owner graph is removed. The family
control fields, including one pending card mutation, are at most 512 serialized
bytes. The worst-case budget is six subject entries (2,496 bytes), four
terminal entries (4 × 1,792 = 7,168 bytes), and control fields (512 bytes),
totaling 10,176 bytes within the 12 KiB cap. The serializer checks the complete encoded size
before every commit and reserves capacity for the next permitted transition.
An invalid field or exhausted reservation makes the family unavailable and
blocks card actions and workspace deletion until recovery; it never drops a
watermark, terminal digest, or pending owner-loss marker. At the 128-family
cap, refuse new learning-family cards with an explicit capacity state while
existing bounded families continue to work. The repository learning store
therefore retains only bounded family state and compact terminal digests; the
full cards stay in their owner workspace graph, with at most four records per
family.

Sibling worktrees may be open through different workspace sidecars at once.
Every read-modify-write therefore takes a retained OS advisory lock for this
repository's learning store, checks the store generation and expected record
digests under lock, then durably publishes an atomic bounded transaction.
Never hold this lock during Git discovery, filesystem traversal, process
launch, UI interaction, or model inference. The global lock serializes only
local learning state; it creates no peer registry, cross-instance attention
authority, workspace lease, or permission to access another worktree. Store
corruption, lock timeout, or publication failure makes learning unavailable;
it is never treated as empty history.

An assignment is the statistical unit, identified by the full immutable #741
assignment identity and exact configuration digest. Multiple reviewer
evaluations, result revisions, resumes, or skill events for one assignment do
not increase its sample count. A learning subject records the repository
binding, run/assignment identity, approved configuration snapshot, selected
skill snapshots, usage-coverage references, result revision, evaluation
revision/digest, rubric and criteria snapshot digests, assignment lifecycle
disposition, and terminal time. It retains references and bounded derived
facts, not transcripts, complete prompts, arbitrary terminal text, or copies
of unbounded evidence.

The default cohort key requires exact equality on all of the following:

- `repositoryId` and `registryEpoch`;
- role ID/version and role-template snapshot digest;
- approved task-category value and approved, normalized scope tags from the
  immutable assignment/task snapshot. Current #741 defines `taskCategory` and
  approved scope bindings but no normalized learning-scope-tag field; until
  #741 adds that immutable field and a canonical mapping, subjects remain
  unmatched rather than deriving tags from prose or inventing a mapping;
- coding-tool identity from #741's harness/adapter binding, including exact
  executable identity and tool version, and the complete canonical #741
  `ModelBinding`. Only pinned model bindings are comparable; tool-managed,
  automatic, unobserved, or unresolved model identities remain unmatched.
  #741 does not separately define model provider/version fields, so do not
  infer or fabricate them;
- effective permission-profile digest;
- a canonical behavior-configuration comparison digest over every other
  behavior-affecting immutable #741 input: the canonical role-template,
  ordered rule, requirement-manifest, rubric, harness/adapter, capability,
  model, permission, and remaining-skill/resource snapshots, including their
  content digests; adapter definition/version, executable/tool identity and
  version, platform, instruction mechanism, `effectiveSettingsDigest`, and
  `adapterGeneration`; plus rendered instruction composition with only the
  candidate skill and its owned resources removed. Task/session/run IDs and
  free-form task prose are excluded; task category and normalized approved
  scope tags above represent task scope. This comparison projection must be
  versioned and canonically serialized, and changing any included field creates
  a different comparison set;
- criteria snapshot digest and rubric ID, version, and rubric snapshot digest;
- all selected non-candidate skill logical IDs, versions, full snapshot/content
  digests, and resource IDs/digests. For one optional candidate skill, the
  comparison-set key omits only that skill; each arm key includes its presence
  and exact version, content digest, and complete resource ID/digest closure.
  Different candidate snapshots are distinct arms, never pooled by a shared
  logical ID or version label.

Display labels, prompt similarity, matching branch names, timestamps, or
repository facts do not relax this key. Different rubric snapshots remain
separate unless #744 later approves an explicit versioned normalization rule.
If any required field is absent, the subject remains visible as unmatched and
cannot influence a ranking. Cohorts never cross repository identities or
installations.

## Reported/observed rates and unknown denominator

Keep `reported_use` and adapter-verified `observed_used` evidence in separate
series. The assignment unit contributes at most one positive unit to each
series for a given skill snapshot, regardless of event count. For
`observed_used`, a complete, finalized adapter stream and a selected,
confirmed-delivered skill define the denominator opportunity. For
`reported_use`, count only terminal assignments with an authenticated, bound
child report stream and a selected, confirmed-delivered skill; report positive
self-report frequency per eligible delivered opportunity, not actual use. A
missing report event is not evidence of non-use and this stream has no
finalization handshake. Missing, partial, unsupported, interrupted, or
corrected/conflicted adapter coverage is **unknown** for `observed_used` and
is excluded from its numerator and denominator. A delivery failure is not a
no-use observation. Self-report never becomes adapter observation, and
adapter evidence never proves more than the verified adapter's declared
coverage.

Show the rate as `positive assignment opportunities / complete opportunities`,
alongside unknown opportunities, assignment count, and coverage. A zero event
count is not a skill-omission signal and cannot be described as proof that an
agent did not use the skill. V1 never recommends removing a skill on non-use
grounds, even when coverage is complete. A user correction invalidates its
target contribution and preserves its provenance. Do not turn repeated events
from one assignment into extra samples.

Rates may be displayed from one complete opportunity as descriptive evidence,
but no rate is called a reliable pattern until it covers at least five distinct
assignments spanning at least two runs. The UI must show counts rather than
presenting a percentage alone. Reported and observed rates cannot be combined
into one score.

## Outcome association and sparse evidence

Only the current result revision with a current, validated, non-invalidated
evaluation can contribute an outcome. One assignment contributes one resolved
outcome after applying #744's evaluator-disagreement rules. `Meets
requirements` is the positive outcome; `Needs rework` is the negative outcome
only when #744 derives it from current uncontested evidence. Preserve required
criterion completeness, rubric quality, findings, and overall result as
separate dimensions; do not invent a weighted scalar or average reviewer
ratings. An `Unassessed` result supplies no positive or negative signal.

Exclude assignments whose lifecycle is blocked, cancelled, interrupted,
unsupported, or partial, even when they contain useful diagnostic details.
Also exclude stale result bytes, invalidated/corrected evidence, stale or
superseded evaluations, conflicting outcomes, mismatched assignment bindings,
unknown rubrics, missing cohort dimensions, and duplicate/replayed subjects.
These records remain explainable history where their source contracts retain
them, but are not selection evidence. Multiple evaluations of one assignment
resolve under #744 before this filter and never count as independent samples.

A single eligible assignment may raise a low-confidence **hypothesis** in the
learning summary with its exact evidence cited. It is not a skill-update card
and cannot change a future configuration preference.
An outcome comparison supports an optional-skill preference only with at least
three eligible assignments in each compared configuration arm, across at
least two distinct runs per arm, using the same exact cohort key. For a
skill-specific comparison, the comparison-set keys must match and the arm keys
must differ only by that optional candidate's presence or exact immutable
snapshot; every other selected skill and configuration dimension stays fixed.
V1 does not learn changes to role settings, task scope, context, model,
permissions, instructions, or other configuration fields.
If the required support is absent, report “insufficient comparable evidence”
and leave task-fit ordering unchanged. These thresholds are conservative v1
defaults, not tunable repository policy.

For one optional logical-skill candidate within an exact comparison set, form
the complete arm set from the skill-absent arm and every exact candidate
snapshot arm. Compare every eligible arm's fraction of assignments whose #744
overall result is `Meets requirements`, using exact integer cross-multiplication
rather than rounded percentages. A learned preference requires the skill-absent
arm and at least one present-skill snapshot arm to meet the per-arm minimum of
three eligible assignments across two runs. If either side of that baseline
comparison is unavailable, learning is neutral. Among all eligible arms,
prefer the arm with a unique highest fraction; if two or more arms share the
highest fraction, leave ordering neutral. Ineligible arms cannot win or break a
tie. `Needs rework` is the non-meeting outcome. `Unassessed` assignments are
excluded from each arm count and denominator, and an arm that then falls below
the threshold cannot win. Do not add criterion, completeness,
quality-dimension, finding-severity, time, cost, or usage values into a
composite or tie-breaker. Those facts remain separate explanatory details.
Thus a mixed criterion/quality vector affects learning only through #744's
derived overall result: an uncontested failure can count as `Needs rework`
even if another dimension is unassessed; without an established failure, the
overall `Unassessed` result supplies no signal.

Outcome association is not causal attribution. A change in score can reflect
task difficulty, reviewer judgment, model behavior, or unmeasured conditions.
Do not claim that an individual skill, model, permission, prompt, or template
caused a result. Never copy an assignment's overall quality score onto every
selected or loaded skill. Mandatory repository skills and mandatory role
instructions are not candidates for removal or bypass.

### Optional elapsed-time and cost observations

The current #744 draft excludes time/cost fields. This section defines units
only for a future compatible amendment; v1 learning does not accept or consume
these observations until #744 explicitly adds a source, provenance, and record
binding for them. If accepted later, elapsed time is an unsigned canonical
`elapsedMilliseconds` integer in `[0, 604800000]` measured by the sidecar's
monotonic clock for the assignment's active runtime; unavailable or
cross-restart intervals are `unknown`, never estimated from timestamps. Cost
is an unsigned canonical `costMicroUsd` integer in `[0, 1000000000000]` with
an exact provider/model identity and bounded `pricingSnapshotId` (1–128 ASCII
bytes). It represents a provider-reported USD amount normalized to one
micro-dollar; unconvertible or unattributed costs are unknown. The upper bound
is USD 1,000,000 per assignment.

If later enabled, time and cost remain separate descriptive dimensions. Cost
may only be compared under the same provider, model, pricing snapshot, and
cohort; neither metric changes a quality/completeness outcome or establishes
skill causality. Missing, unknown, or incomparable values are omitted from
that metric's denominator and shown as unknown counts. #744 review must
explicitly accept or revise this encoding before any producer persists it.

## Versioned history and correction invalidation

Learning records are append-only versioned snapshots. A correction or
invalidation adds provenance and advances the assignment subject revision; it
does not rewrite the earlier evaluation/configuration snapshot. The active
projection removes invalidated contributions immediately and recomputes its
aggregate from remaining eligible subjects under the store lock. If a source
record cannot be resolved, has been purged, or its digest/revision differs,
exclude it and mark the projection degraded; never use a cached summary as a
fallback.

Every aggregate records its schema version, repository binding, cohort-key
digest, exact source subject references/digests, counts by eligibility and
outcome, unknown coverage counts, and derivation version. Aggregates are
reproducible summaries, not authoritative evaluations. Changing normalization,
thresholds, or grouping rules requires a new derivation version; the old
snapshot remains historical until normal expiration or explicit forgetting.
Evaluation corrections, assignment revisions, usage corrections, and
repository evidence-generation changes trigger a locked revalidation before
the next summary is served. A correction cannot resurrect an expired or
forgotten contribution.

The orchestrator receives a bounded summary, never direct access to raw event
logs or reviewer evidence. The summary is capped
at 32 KiB, 64 cohorts, 32 candidates, and five representative evidence
references per candidate. Each candidate includes the exact configuration
delta, positive/negative/unknown counts, coverage, confidence category,
cohort-key digest, evidence references, exclusions, and derivation version.
Order cohorts by bytewise cohort digest and candidates by bytewise candidate
fingerprint, independent of outcomes. If a byte, cohort, or candidate cap
omits any candidate or cohort, set `selectionUsable=false`; no learned
preference from that decision is usable, even for candidates that fit in the
summary. A missing, empty, unavailable, corrupt, or stale summary also
contributes no preference and falls back to task fit plus mandatory rules.

## Bounded summaries supplied to the orchestrator

The summary is an internal read-only input to planning. The orchestrator first
filters configurations by explicit task fit, user constraints, mandatory
skills/instructions, capability support, and effective permissions. Learning
may order only otherwise-eligible optional choices when its evidence threshold
is met. It cannot broaden access, choose an unsupported harness, bypass a
required review, or override user preferences. Missing history behaves
exactly like no learned preference.

Every proposed configuration that used learning includes a concise reason:
which optional skill changed, the exact matched cohort dimensions,
per-arm assignment/run counts, outcome counts, usage coverage/unknown count,
source digests, and why alternatives were not selected. The plan binds the
complete immutable proposed configuration and a `learningSnapshot` binding:
the store generation plus an aggregate/source-set digest over the complete
canonical summary and every supporting reference (not only the displayed
sample). Approval compares that binding under the learning lock; if it changed,
rebuild or withdraw the plan before approval. The normal explicit plan review
and approval is still required. No change reaches an active assignment,
bootstrap, prompt, skill file, or permission profile automatically.

## Explainable future configuration choices

For a supported cohort, compare the absent arm and every eligible exact
optional-skill snapshot arm together under the exact `Meets requirements`
fraction rule above. Only a unique highest arm, with an eligible absent arm
and at least one eligible present arm, may inform ordering. Tied leaders,
missing baseline support, or no qualifying arm leave learning neutral and
ordinary task-fit/user-preference ordering decides. Conflicting outcomes are
`Unassessed` under #744 and do not enter their arm fraction. Usage frequency
alone never ranks configurations. Preserve the reason and matched/unknown
counts with the later proposal so a reviewer can see whether its evidence
still applies. A newly invalidated source causes the proposal to be rebuilt or
withdrawn before approval; it never silently keeps the old ranking.

One run can raise a hypothesis to investigate or a one-off configuration
suggestion to review, explicitly labeled as single-run evidence. Only repeated
comparable outcomes meeting the support threshold above can change the
selection preference. A recurring skill-update card has the stricter
threshold in the following section. This allows a useful prompt to be
surfaced without turning one success or failure into policy.

## Skill-improvement drafts and rejection memory

Recurring learning findings may create a passive `ImproveWorkflow`
recommendation with target surface `skill`, `instructions`, or
`documentation`. Use Taskmaster's existing recommendation record, graph,
status transitions, explicit `Fix with AI` action, and completion reporting;
do not create a second queue, acceptance API, draft-mutation path, or
promotion lifecycle. Extend that record with a typed, bounded learning-evidence
projection referencing exact assignment/evaluation digests and recurrence
counts. Keep these references distinct from `WorkflowObservationEvidence`;
never relabel assignment evaluations as observations. While proposed, the
projection names pinned live source references. Before a terminal transition
releases any source dependency, the owner graph transaction seals a typed,
immutable `learningEvidenceSnapshot` into the terminal `ImproveWorkflow`
record. It contains the canonical finding descriptor and fingerprint, target
surface and exact target snapshot identity, criteria/rubric digests,
derivation version and source-set digest, support and distinct-run counts, and
the exact selected source entries demonstrating recurrence: subject
ID/revision, run ID, assignment configuration and comparison-arm digests,
evaluation digest, overall result, and applicable criterion/dimension ID and
outcome. Include usage summaries only when they are part of the finding. The
canonical serialized snapshot is at most 8 KiB and contains no raw artifacts,
source files, prompts/transcripts, secrets, or foreign workspace/session IDs;
if it cannot fit, fail the transition closed and retain source dependencies.
The sealed snapshot is historical explanation only: exclude it from
eligibility, aggregation, dismissal-watermark, and resurfacing inputs. Explicit
forgetting may remove its explanatory payload while retaining bounded terminal
status and lineage, so a forget request cannot be defeated by graph history.

Learning-finding identity is versioned and server-derived from a canonical
descriptor containing repository epoch/ID, target surface, target logical
skill ID (or `null`), finding class, one stable affected criterion/dimension
ID, task category, and sorted normalized task-scope tags. The v1 `findingClass`
enum is `required_criterion_unsatisfied` or
`quality_dimension_below_standard`; the corresponding stable ID must resolve
to the required criterion or rubric dimension in that assignment's approved
snapshot. The descriptor also contains the exact criteria and rubric snapshot
digests, so a reused label with changed meaning cannot silently join an old
family. Task category is an exact approved nonempty enum token; each scope tag
is an approved lowercase ASCII slug matching `[a-z0-9][a-z0-9._-]{0,63}`, and
the list is unique and byte-sorted. If these typed fields are absent, the
finding is unmatched and cannot produce a recurring learning proposal. Do not
derive them from free-form evaluation prose.

The descriptor excludes generated prose, session IDs, run IDs, timestamps,
and the current skill snapshot/version. The fingerprint is lowercase
SHA-256 over
`orkworks.learning-finding.v1\n` followed by #741-canonical JSON for that
descriptor; store the descriptor and digest together and reject a digest whose
descriptor does not recompute. The logical skill ID keeps an equivalent
proposal grouped across content versions, while every evidence reference still
names the exact skill snapshot and evaluation digest it observed.

Learning-only records use the family key
`improve_workflow:learning:v1:<target-surface>:<learning-finding-fingerprint>`.
The existing observation family
`improve_workflow:v1:<target-surface>:<observation-fingerprint>` remains
unchanged. Learning findings use their own family namespace and remain
separately attributed recommendations in the same canonical Taskmaster graph
and lifecycle. V1 does not merge evaluation findings with workflow
observations, even when their titles or targets look similar: the observation
contract has no shared typed finding descriptor that proves equivalence.
Text similarity, matching titles, or the same target skill is never enough to
coalesce or suppress evidence. Each source family allows at most one proposed
record for its exact key; related cards may appear side by side and are
dismissed independently. This preserves the existing `ImproveWorkflow`
mutation path without inventing a cross-source fingerprint or pretending
assignment evaluations are observations.

The learning evaluator may raise a one-assignment hypothesis, but a recurring
skill-update proposal requires at least three distinct eligible assignment
subjects across at least two runs, with the same normalized finding and target
surface. A related finding supported only by session friction may continue
through the existing observation pipeline and its own eligibility rule. The
two sources do not merge or count toward each other's recurrence threshold.
Learning dedupe uses repository binding, target surface, stable logical skill
identity where applicable, and the normalized learning-finding fingerprint;
the exact skill snapshot/version remains evidence, not family identity. It
never uses generated prose.

The v1 target surface is exactly `skill`, `instructions`, or `documentation`.
Choose the smallest supported target among those three: documentation for a
missing fact/convention, a skill for a reusable procedure with checkpoints,
or instructions for broad repository guidance. Other Taskmaster surfaces such
as `tooling` or `test` are not valid learning targets in v1. The recommendation
contains an edit outline and
evidence, not a ready-to-apply patch or a committed artifact. “Fix with AI”
hands the user-reviewed task to the active session. The user reviews resulting
file changes and decides whether to commit/promote them. No approval edits
repository files by itself.

Dismissal is remembered in the existing immutable recommendation history.
The learning-family watermark contains the sorted assignment-subject IDs,
their evaluation digests and subject revisions, the exact represented
`runIds`, and qualifying recurrence count at dismissal. The existing
observation-family watermark retains its current sequence, observation
IDs/count, impact, and session fields from
`specs/taskmaster.md`; the two watermark schemas remain source-specific.
Learning evidence may create a successor only after at least two newly
eligible assignment subjects qualify, including one from a `runId` absent from
the learning watermark. Observation evidence follows its existing
sequence/impact rule. The successor cites only new evidence plus enough
immutable lineage to explain its predecessor. Time passing, rerunning
analysis, a changed model-generated title, a new skill version without a
materially changed finding, or replaying the same evidence is not new evidence.
A resurfaced successor links the dismissed recommendation and shows exactly
which references crossed its watermark. Dismissing one family does not dismiss
or suppress a related card from the other source. Rejection does not delete
history or retire a skill permanently.

The shared recommendation graph must support a typed `superseded` transition
for proposed learning-family cards when evidence is invalidated, forgotten,
expires, or falls below eligibility. If workspace deletion removes the owner,
the graph must also support this terminal transition from a learning card in
`proposed`, `executing`, or `accepted`, release any execution reservation, and
participate in the ordered owner-loss protocol above before workspace metadata
is deleted. This is a proposed extension to the
current Taskmaster contract, which currently permits `superseded` only for
assessment-derived cards. It requires an explicit Taskmaster spec/API update
before implementation. The transition is distinct from user dismissal
and carries a reason (`evidence_invalidated`, `learning_history_forgotten`,
`source_expired`, `below_eligibility`, or `owner_workspace_deleted`) plus the
replacement or source-set digest when available. Invalidation and supersession
commit together in the owner graph transaction, and a superseded card cannot
be accepted. The pending repository fence spans that graph transaction and
the later repository-store commit; it clears only after both stores reconcile.
Seal terminal-card evidence before releasing its source dependencies. Normal
accepted/executing transitions are serialized by the
canonical store; owner deletion is the explicit typed terminal exception, not
a silent rewrite. The durable pending marker blocks stale reads/actions across
the graph/store boundary until both commits reconcile. Deletion never transfers
session authority or adopts a surviving workspace's session. A fresh
successor requires local reevaluation and a new workspace-bound record, subject
to the four-record family lifetime cap above.

## Retention, forgetting, and concrete examples

The shared repository learning store has hard v1 limits of 1,000 active or
tombstoned assignment subjects, 2 MiB of normal serialized learning records,
and a separate 512 KiB reserved correction/fence budget per repository binding
(2.5 MiB total);
each subject record is at most 8 KiB, each aggregate is at most 64 KiB, and
there are at most 64 live cohort aggregates. Counts are checked before
publication. The reserved budget holds one fixed 512-byte maximum
`SubjectFenceEnvelope` for every possible subject; ordinary contributions
cannot use it. The canonical envelope is `[1, forget_tombstone?, active?]`.
The persistent forget tombstone is at most 192 bytes and retains only subject
ID, subject revision, and forget generation. At most one transient active
fence may coexist with it: either a subject mutation fence (at most 160 bytes)
or a workspace-deletion fence (at most 96 bytes, including its workspace
binding digest, operation ID, expected generation, and phase). Their combined
canonical encoding and envelope overhead must fit 512 bytes, enforced before
every publication. Deletion first recovers any active fence, then atomically
replaces only the transient slot while preserving the forget tombstone. It
clears the tombstone only after the corresponding source is actually purged;
thus an old tombstone cannot consume a second slot or be overwritten by
deletion. If that reserve or any consistency bound is exhausted, fail closed and
serve no summary that could contain stale learning. At normal-capacity
pressure, reject new learning contributions with a visible capacity state;
never evict a retained dependency, lower a threshold, or silently discard the
oldest evidence. These are learning-store caps, separate from source-contract
per-workspace usage/evaluation caps.

Retain a learning subject and its source dependencies for at most 180 days
after its assignment becomes terminal, consistent with #743's aggregate
maximum. At that limit, remove the contribution and recompute affected
aggregates before releasing any #743/#744 dependency. If another current
evaluation, reviewer-credibility chain, or retained learning contribution
still depends on an evaluation/result, preserve it under the source contract
until that dependency is explicitly removed. Workspace deletion releases only
that workspace's learning dependencies. It must not purge a protected source
still needed by another current result, reviewer chain, or learning
dependency. The #744 source custodian retains such sources independently of
deleted workspace metadata; if it cannot, workspace deletion stays pending
until dependency custody is safely transferred or released. Raw skill events still obey
#743's shorter 30-day maximum; a learning contribution whose usage source has
expired becomes unknown for usage and cannot recreate that source from an
aggregate. Never retain a learning aggregate past the source evidence needed
to explain its claim.

An Electron-authorized user may explicitly forget repository learning
history. Under the repository lock, first fence new learning reads/writes,
durably tombstone every learning subject and advance the forget generation
before removing derived aggregates and pending candidates. Tombstones retain
only bounded subject IDs/revisions and the forget generation, never source
payloads; recomputation and crash recovery must exclude tombstoned subjects
even when #744 preserves their source records for another dependency. Tombstones
use the reserved correction/fence budget and remain until the corresponding
source record is actually purged, even if its nominal retention has elapsed
while a protected dependency keeps it. Then transition proposed learning
recommendations to `superseded(learning_history_forgotten)` in the same graph
transaction. Preserve dismissed/completed recommendation history as history
with no active learning input. Release source dependencies that
are no longer needed; #744's exact-subject purge remains conditional on its
subject revision and the absence of current-result, reviewer-chain, or other
learning dependencies. If a dependency remains, report it and leave the
protected source intact while its learning tombstone continues to block reuse.
Forgetting does not reset repository identity, reassign records, or affect
other repository IDs. Workspace deletion removes that workspace's learning
contributions and releases only its source dependencies through the fenced
protocol; it does not erase other worktrees' contributions or purge sources
protected by another dependency. Workspace metadata may be removed only after
protected-source custody is independent of that metadata and every affected
owner graph and repository family state has reconciled. If the cross-store
transaction cannot commit, deletion remains fenced and reports
recovery-required rather than leaving a summary that cites deleted evidence.

Examples:

- A role has one current `Meets requirements` result with a verified skill
  invocation. Taskmaster may show a single-run hypothesis and its source, but
  selects configurations by task fit because neither outcome arm meets the
  three-assignment threshold.
- Three assignments with skill A and three with skill B share the exact role,
  task/scope, tool/model, permission, rubric, criteria, and other-skill
  snapshots across at least two runs per arm. Current uncontested outcomes may
  support a later optional-skill preference; the explanation reports both
  arms' raw outcomes and does not claim the skill caused them.
- Five assignments report use but only two have complete adapter coverage.
  Reported and observed series remain separate; the three unknown adapter
  opportunities are shown separately and excluded from that rate's
  denominator.
- A child assignment is interrupted and has a failing test artifact. Its
  diagnostic remains available under #744, but it cannot count toward a
  learning outcome cohort.
- A reviewer invalidates an evaluation after source bytes change. The next
  locked summary removes the old outcome; any plan based on it is revalidated
  before approval, and an equivalent dismissed suggestion is not resurfaced
  without materially new evidence.
- Two linked worktrees contribute concurrently. Their sidecars serialize on
  the repository learning lock and publish distinct assignment subjects; a
  clone with a different repository ID cannot read either contribution.
- A workspace is deleted while its assignments contributed to a repository
  cohort. Its learning contributions are removed in the same recoverable
  transaction; the remaining worktrees' evidence continues to form the
  aggregate.

When storage is missing or empty, use task fit and mandatory setup without
learning. When storage is malformed, locked, over capacity, or its identity
cannot be revalidated, mark learning unavailable and expose the reason; never
silently initialize a fresh store or fall back to stale cached advice.

## Verification cases

- Same active binding across linked worktrees yields the same repository
  history; clone, nested repository, retired binding, new registry epoch, or
  other installation yields no matching history.
- Concurrent readers/writers serialize; stale expected digests, lock timeout,
  crash before/after atomic publication, malformed records, duplicate subject
  IDs, and capacity exhaustion preserve the prior valid store without
  fabricated empty history.
- Exact cohort equality accepts only the approved dimensions; unknown or
  tool-managed model, missing normalized #741 task-scope tags, changed
  permissions, different rubric/criteria digest, changed non-candidate skill,
  different scoped rules, harness/adapter definition or version, executable,
  tool version, instruction mechanism, effective settings, capability
  evidence, `adapterGeneration`, or unsupported tool remains unmatched.
  Candidate snapshots with identical logical ID/version but different
  content/resource digests remain separate arms.
- Multiple evaluation revisions for one assignment count once; corrected,
  invalidated, conflicting, stale, replayed, blocked, interrupted, partial,
  unsupported, or unassessed subjects do not become positive or negative
  learning outcomes.
- Usage rates keep reported/observed series apart; unknown, incomplete,
  unsupported, failed-delivery, and conflicting coverage is excluded and
  visibly counted; one assignment cannot add multiple numerator units.
- One eligible assignment can raise only a labeled hypothesis; fewer than
  three eligible subjects per arm or fewer than two runs per arm cannot
  change selection preference; exact threshold cases pass.
- Across the absent arm and multiple present snapshot arms, a unique highest
  eligible exact fraction wins; ties among any top arms are neutral, and an
  unavailable or under-supported absent baseline prevents a preference.
- For eligible absent, snapshot-v1, and snapshot-v2 arms, verify each global
  outcome: v1 uniquely highest selects v1; absent uniquely highest selects
  absent; tied v1/v2 leaders are neutral. An under-threshold v2 arm cannot
  win or break a tie. Results do not depend on pairwise comparison order.
- Compare all eligible arms in one exact comparison set, never by outcome-
  ordered pairwise traversal; an under-supported present arm cannot win or
  break a tie.
- Selection compares exact `Meets requirements` fractions: higher wins,
  equal fractions are neutral, `Unassessed` is excluded, and mixed quality /
  completeness dimensions never become an undocumented tie-breaker.
- At three eligible assignments per arm, `2/3` Meets versus `1/3` prefers the
  first arm; `2/3` versus `2/3` is neutral. An `Unassessed` subject excluded
  from an arm that then has only two assignments makes the comparison
  ineligible, regardless of its criterion/quality details.
- Mandatory skills cannot be removed; usage-only evidence cannot rank skills;
  only optional-skill presence/snapshot arms may change. Role settings, task
  scope, context, model, permissions, and instructions remain task-fit-neutral.
  Plans show matched dimensions, both-arm counts, unknown coverage, source
  digests, and deterministic fallback on missing/corrupt history.
- Skill proposals require three distinct eligible assignments across two
  runs, dedupe across the same normalized target, use the existing
  `ImproveWorkflow` lifecycle, suppress unchanged dismissed evidence, and
  expose the precise evidence delta on a valid successor.
- Learning fingerprints are stable across worktrees and skill content
  versions for the same logical target/finding with matching criteria/rubric
  snapshot digests, distinct across different repositories, target skills,
  finding classes, scopes, or rubric snapshots, and source-separated from
  existing observation keys. The same repo/finding from two worktrees shares
  its learning family. Similar-looking learning and observation findings
  remain distinct and use independent dismissal/resurfacing rules in the
  shared lifecycle; dismissed evidence cannot cross-advance either watermark.
- Forgetting, workspace deletion, source expiry, explicit #744 purge, and
  repository retirement do not leave stale summaries or remove protected
  current evaluation/reviewer dependencies; durable forget tombstones prevent
  protected source records from recreating forgotten contributions even after
  nominal retention elapses; tombstones clear only after actual source purge.
  Recovery after an interrupted cross-store transaction is idempotent.
- A linked worktree's learning projection contains no foreign workspace/session
  identifiers, lifecycle details, or session-bound actions; accept, `Fix with
  AI`, and completion route only through the owning workspace API, while
  dismissal updates shared repository rejection memory. Injected crashes at
  each owner-loss phase keep the card unavailable until recovery; completion
  terminally supersedes the old card, releases executing reservations, and
  permits only a freshly bound successor after local reevaluation, never
  adoption of the old target session. Four lifetime cards seal a family while
  preserving their bounded terminal projections and digests after owner graph
  deletion. I/O failure after the `owner_loss` pending marker keeps workspace
  metadata and blocks deletion until
  forward recovery; cancellation is accepted only before the marker.
- Inject crashes after workspace-deletion fence publication, learning-subject
  tombstoning, affected-card graph commits, protected-source custody transfer,
  eligible source purge, and final fence clear. At every point, workspace
  metadata remains until recovery completes; no summary cites purged evidence,
  and an evaluation/reviewer source with another live dependency remains
  protected under independent #744 custody.
- At maximum subject capacity with retained forget tombstones, workspace
  deletion atomically composes each tombstone with its transient deletion
  fence inside the 512-byte envelope; no tombstone is overwritten and the
  operation does not require an unavailable second fence slot. Injected
  envelope overflow fails before publication and leaves all sources intact.
- An interrupted sibling dismissal, invalidation, forgetting, expiry, or
  below-eligibility transition leaves the affected family hidden until owner
  recovery commits the expected graph transition and repository finalization;
  a compare-and-swap conflict does not create a false terminal status.
- Workspace deletion releases only its own source dependencies. A source with
  another current-result/reviewer dependency survives deletion only when #744
  custody is independent of deleted metadata; otherwise deletion stays
  pending. Deleted learning contributions cannot rehydrate from preserved
  sources.
- A terminal card's bounded immutable learning snapshot retains exact
  recurrence evidence after ordinary source expiry/purge; release sources only
  after sealing succeeds. Snapshot overflow fails closed, and terminal
  snapshots never re-enter active learning. After owner graph deletion, the
  retained projection still names the typed finding class, affected criterion
  or dimension, task category/scope, and each cited assignment's overall and
  finding outcome; the maximum descriptor plus three entries fits 1,536 bytes.
- Dismissed learning watermarks retain exact run IDs; when six-source watermark
  bounds, the four-card lifetime cap, or the 128-family state cap is reached,
  resurfacing is disabled for that family or new learning-family cards are
  refused visibly, never by truncating exact history. Maximum encoded entry,
  terminal projection, digest, and control-field sizes fit within 12 KiB;
  injected overflow fails closed without dropping retained data or completing
  a card action. Canonical CBOR pending mutations fit within 160 bytes for
  every enum variant, use shortest-form integers and definite lengths, accept
  only the six listed kind/reason pairs, and reject invalid cross-pairs before
  any store write.
- Learning-card target validation accepts exactly `skill`, `instructions`, and
  `documentation`; `tooling`, `test`, and unknown future values are rejected.
- Optional time/cost inputs are rejected by current #744 unless its reviewed
  contract adopts the defined units; unknown or incomparable future values do
  not enter quality or selection scores.

## Execution plan and implementation gate

This document is a proposed learning/history contract, not an execution plan.
Before any runtime plan or code, obtain written review of this contract and
reconcile it with the final #743/#744 contracts, #741 assignment/configuration
identities, #740 adapter capability evidence, #742 report authority, #610
workspace/child constraints, ADRs 0060/0077/0080, and the current
`ImproveWorkflow` record/store invariants. In particular, review the
installation-local shared store, cross-store workspace-deletion transaction,
learning-evidence recommendation projection, numerical thresholds/quotas,
and time/cost amendment boundary as consequential choices. #741 must add the
immutable normalized learning-scope tags required by these cohorts; until then,
subjects without them remain unmatched. Use only #741's exact harness,
adapter, and pinned `ModelBinding` fields; tool-managed models remain unmatched.

The later scoped plan must name concrete types, lock/transaction order, source
reference resolvers, recovery behavior, deterministic summary encoding,
recommendation graph updates, and tests for every verification case above.
No route, producer, adapter, history store, chooser, skill editor, or promotion
capability is authorized by this specification alone. Upstream reviews,
platform evidence, and explicit implementation approval remain prerequisites.

## Evidence and concrete cases

A disposable local fixture on 2026-10-04 used Git 2.54.0 (Apple Git-157), no
model/agent probes. It created a repository, linked worktree, clone, submodule,
symlink alias and separate Git directory. With inherited `GIT_*` routing removed
and global/system config disabled, each directory was queried using
`git rev-parse --path-format=absolute --git-common-dir`, `--absolute-git-dir`
and `--show-toplevel`, then filesystem-canonicalized. The fixture initially
compared canonical `/private/var` paths against an uncanonicalized `/var` temp
root; canonicalizing that root fixed the measurement harness. All six final
layout assertions passed. These are Git discovery facts, not platform registry,
permission, crash-recovery or admission evidence. Primary option semantics:
[git-rev-parse](https://git-scm.com/docs/git-rev-parse).

| Fixture | Common directory relative to fixture root | Expected identity |
| --- | --- | --- |
| Primary `main` | `main/.git` | R1 after enrollment |
| Linked `linked` | `main/.git` | R1, different worktree binding |
| Symlink `alias` to primary | `main/.git` | R1 after canonicalization |
| Clone of primary | `clone/.git` | New registration; never R1 |
| Submodule `main/nested` | `main/.git/modules/nested` | Separate repository |
| `external` with separate Git dir | `admin` | Its own registration; no parent-path inference |

Required contract fixtures before implementation eligibility:

- Primary/subdirectory/linked alias resolve the same registered repository;
  worktree paths/private directories remain distinct. Separate clone, nested
  repo, submodule and alternates never merge identity.
- Bare input/non-Git input, broken `.git`/`commondir`, stale registrations,
  unsupported filesystem, contradictory linkages and injected Git routing
  fail with explicit reasons. A bare-backed working tree is checked separately.
- Unix native-byte paths and Windows UTF-16, case/junction aliases, long paths
  and invalid encodings have shared byte/digest fixtures and platform receipts.
- Observable common-directory replacement at the same locator, worktree
  replacement, relocation and post-approval drift block admission/resume.
  Matching remotes/commits cannot heal a mismatch. Enrollment of a moved
  same-object family blocks until its exact prior registration is explicitly
  retired. Moving away and back cannot reactivate either retired ID or resume
  its old run, even if root/private-directory object IDs match again; cover
  concurrent enrollment and a crash between retirement and new enrollment.
- Concurrent enrollment across sidecars returns one ID; retirement versus
  validation, timeout, full registry, crash before/after publication, lock-file
  retention, malformed registry and explicit reset preserve their dispositions.
- Source discovery/allocation/materialization/spawn races retain interrupted
  artifacts and never adopt a foreign worktree or automatically retry. Root,
  private-directory and repository binding are all rechecked on exact resume.
- Lost/reset registry denies old configurations even when paths still exist;
  ordinary sessions remain ordinary. Registry state cannot approve or launch.
- Same-family replacement of the approved source worktree or its private Git
  directory, a swapped target-parent directory and linked-source replacement
  fail even when repository identity still matches. All source snapshot fields
  are included in approval/configuration/plan digests and fresh resume checks.
- Supported archive restore uses a new epoch and denies old bindings; document
  direct-copy/rollback limitations rather than claiming copy resistance.
- Configuration/bootstrap/plan digests include the binding; mismatched fields,
  missing snapshots and attempted rebind require a newly reviewed run.

## Review and scoped execution handoff

The [documentation task plan](https://github.com/Rambolarsen/orkworks/blob/main/docs/superpowers/plans/2026-10-04-repository-identity-specification.md)
records the accepted repository-identity contract review. The learning/history
extension above is proposed for written review under #745. The accepted identity
decision is recorded in ADR 0080 and its registry substrate has a separate
[#810 implementation issue](https://github.com/Rambolarsen/orkworks/issues/810)
and a gated execution plan in this repository.
That plan consumes existing Git discovery and native filesystem/persistence
mechanisms, and separates registry resolution/persistence from allocator
integration. Native identity/durability evidence and approval of that exact
plan are required before implementation. No interface or evidence here makes
#610 units 1–4 ready automatically.
