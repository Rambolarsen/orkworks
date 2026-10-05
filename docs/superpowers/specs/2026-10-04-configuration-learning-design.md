# Repository-scoped configuration learning and skill improvements

- Status: proposed repository-identity contract; written review pending
- Date: 2026-10-04
- Tracking: [#745](https://github.com/Rambolarsen/orkworks/issues/745), initiative [#738](https://github.com/Rambolarsen/orkworks/issues/738)
- Consumers: [role configuration](2026-10-04-agent-role-configuration-design.md), [ordinary-child admission](https://github.com/Rambolarsen/orkworks/blob/main/docs/superpowers/plans/2026-09-26-taskmaster-orchestrated-child-sessions.md)
- Product direction: [hierarchy and configuration learning](2026-10-04-agent-hierarchy-and-configuration-learning-design.md)

## Scope and status

This first portion of #745 defines one local repository identity across its
Git worktrees. It is an early input to #741 and #610 worktree/child admission,
independent of skill usage and assignment evaluation. It does not complete
#745's comparison cohorts, learning summaries, outcome association, skill
proposals, rejection memory or learning-history retention contracts. Those
consume #743/#744 and remain separate specification work.

The ordinary-child scope is accepted under [ADR 0077](../../adr/0077-taskmaster-orchestrated-child-sessions.md).
This algorithm, registration defaults and configuration amendment are proposed.
There is no runtime implementation, new endpoint, eligible adapter or approved
code execution plan. Written contract acceptance and a scoped implementation
plan are required before source changes; an ADR must record the accepted
identity/persistence decision before implementation.

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
| Local random registration bound to canonical common directory and filesystem object identity | Needs bounded local persistence and platform evidence; detects observable replacement without writing to Git metadata | Proposed initial contract |

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

Object identity is proposed as Unix device/inode and Windows volume/file ID
under a versioned platform adapter. These mechanisms are not verified by the
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
These are proposed amendments requiring the consuming contracts' written review.

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
never deletes this shared identity mapping. Future learning-history forgetting
must separately define protected evidence and user deletion; it may remove
history without reusing identity IDs. This early mapping cannot be used as an
unbounded history store. Registry reset is an explicit recovery path with new
identity, not routine history retention or cleanup.

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
tracks this narrow deliverable. Review the proposed local registration strategy,
replacement/relocation behavior, platform eligibility, persistence limits and
configuration amendments jointly with #741/#610 before claiming a reviewed
identity prerequisite. #745 remains open for this review and its downstream
learning/cohort/history scope.

Only after written acceptance: record the persistence/identity decision in an
ADR, finalize #741/#742 consumer descriptors, create a scoped implementation
issue linked to #745/#610/#738, and approve its exact code plan. Candidate seam
is a repository-identity module, consuming existing Git discovery and native
filesystem/persistence mechanisms, with dedicated platform fixtures. The plan
must separate registry resolution/persistence from allocator integration and
carry the error/recovery/race cases above. No interface or evidence here makes
#610 units 1–4 ready automatically.
