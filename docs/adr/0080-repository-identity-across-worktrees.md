# ADR 0080: Repository identity across Git worktrees

- Status: accepted
- Deciders: owner
- Date: 2026-10-10
- Tracking: [#810](https://github.com/Rambolarsen/orkworks/issues/810), prerequisite to [#610](https://github.com/Rambolarsen/orkworks/issues/610) under [#745](https://github.com/Rambolarsen/orkworks/issues/745) and [#738](https://github.com/Rambolarsen/orkworks/issues/738)
- Contract: [repository-scoped configuration learning](../superpowers/specs/2026-10-04-configuration-learning-design.md)

## Context

The proposed ordinary-child orchestration uses separate Git worktrees for
independent task chains. Existing Git context identifies a checkout root, while
the advisory plan-path resolver compares common directories for one file lookup.
Neither provides a durable, approved repository identity for later configuration
records, allocation, or resume. A path alone can be replaced; a remote, branch,
commit, shared object database, or matching content can join unrelated clones.

The accepted [ADR 0077](0077-taskmaster-orchestrated-child-sessions.md) keeps
one selected workspace, sidecar, metadata store, and lease, and leaves Git
integration manual. [ADR 0060](0060-independent-workspace-instances.md) keeps
instances independent and workspace history path-only. Repository identity must
not weaken either boundary.

## Decision

Define one repository family as a Git common-directory object at one canonical
native locator, enrolled in an installation-local registry epoch. The primary
checkout and valid linked worktrees share a `repositoryId`; each has a separate
complete `ResolvedWorktree` binding. Separate clones remain separate even when
their remotes or contents match. Submodules and nested repositories use their
own common-directory identities. A bare path and non-Git directory cannot
enroll for orchestrated use; ordinary sessions keep their existing behavior.

The registration binds the exact canonical common-directory path encoding and
a versioned native directory-object identity obtained through an opened handle.
The repository and worktree binding digests use the accepted canonical JSON
fixtures in the contract. Resolution uses Git's repository API from an explicit
existing working directory; it does not infer a common directory from a
`.git` suffix, trust agent/renderer IDs, use lossy display paths, or scan
neighboring repositories. Unsupported or unverifiable identity returns a
typed failure; no path-only fallback is allowed.

Keep these identities distinct:

- `workspaceId` remains the existing path-derived workspace metadata identity
  and workspace lease.
- `repositoryId` identifies only the locally registered Git worktree family.
- `sourceWorktreeBinding` snapshots the exact selected workspace root, private
  Git directory, their native object identities, and the repository binding.
- Future child worktree allocations have their own exact approved descriptors
  and receipts; family membership is not ownership or adoption proof.

Persist registrations separately from workspace history at
`~/.orkworks/repository-identities/v1/registry.json`, protected by a retained
OS advisory lock file. The registry is bounded to 8 MiB and 1,024 records; one
encoded registration is at most 8 KiB, each native path at most 4,096 bytes
before base64, and each object-identity field at most 128 ASCII bytes. Keep
active and retired registrations; exhaustion blocks enrollment and never
evicts or rebinds an identity. Use verified durable atomic publication.

Only Electron-authorized run preparation may enroll a new repository after
showing its local locator. Enrollment writes OrkWorks metadata only. Child or
parent reports cannot enroll, replace, retire, or select a historical ID.
Concurrent enrollment serializes to one registration. A changed object at a
registered locator, a relocated registered object, corrupt state, or ambiguous
aliases blocks admission. Relocation requires explicit retirement of the exact
old binding before new enrollment; a retired ID never reactivates, including
after moving away and back. Registry reset or supported restore creates a new
random epoch and invalidates old bindings.

Bind the complete repository and source-worktree snapshots into the immutable
orchestrator bootstrap and child assignment. Bind their digests into the run
definition, every exact `ParentPlanBinding`, and later allocation descriptor
and receipt. `repositoryId` must equal the ID in its binding. Preparation input
manifests remain scoped to mutable task/input revisions and cannot replace
these bindings. Missing or mismatched bindings deny the affected orchestration
operation; they never migrate by path/name inference or transfer approval.
Consumers must rediscover and revalidate the registration and full source
worktree snapshot at run creation, plan approval, allocation intent, immediately
before Git mutation, after materialization, and before child spawn or exact
resume. The five-second Git display cache and the plan-path resolver are not
identity authority.

The registry is identity metadata, not installation workspace history, a peer
instance registry, process/port/health state, a workspace lease, or a permission
grant. It contains no session history, remote URL, credential, Git config,
transcript, or active-instance data. It does not allow OrkWorks to discover,
adopt, focus, monitor, or control another instance or an existing worktree.
It does not establish native filesystem/process confinement or prevent direct
same-user registry rollback. Those limits remain explicit.

## Consequences

Orchestrated work can compare exact repository-family and source-worktree
bindings without replacing the selected workspace's path identity or lease.
The identity substrate is a prerequisite, not launch authority. The sidecar
registry and its native identity/durability mechanisms are tracked by #810;
later #610 work must separately bind and revalidate them at every admission
boundary. Neither this ADR nor #810 implements or enables ordinary-child
orchestration.

Platform eligibility remains evidence-gated. Git layout fixtures establish
discovery semantics only; they do not verify native object identity, filesystem
alias handling, durable publication, races, or any production OS/filesystem
combination. Unsupported combinations remain unavailable until those checks
pass. Same-user replacement races, identifier reuse, and direct byte-for-byte
registry rollback cannot be ruled out by this protocol.

Repository-local learning may later use this identity only with separately
approved cohort and retention rules under #745. This decision creates no
learning-history read/write interface and does not authorize cross-repository
pooling.
