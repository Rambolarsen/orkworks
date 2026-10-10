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
- approved task-category value and relevant scope tags from the immutable
  assignment/task snapshot (unknown or free-form-only scope forms its own
  unmatched cohort);
- coding-tool identity/version and selected model provider/model/version, or
  the same explicit unknown value with its producer/adapter generation and
  unknown reason (unknown never pools with a known value);
- effective permission-profile digest;
- criteria snapshot digest and rubric ID, version, and rubric snapshot digest;
- all selected skill identities and versions, except the one candidate skill
  whose presence/version is the sole permitted variable for that comparison.

Display labels, prompt similarity, matching branch names, timestamps, or
repository facts do not relax this key. Different rubric snapshots remain
separate unless #744 later approves an explicit versioned normalization rule.
If any required field is absent, the subject remains visible as unmatched and
cannot influence a ranking. Cohorts never cross repository identities or
installations.

## Reported/observed rates and unknown denominator

Keep `reported_use` and adapter-verified `observed_used` evidence in separate
series. The assignment unit contributes at most one positive unit to each
series for a given skill snapshot, regardless of event count. A complete,
finalized usage stream with a selected and confirmed-delivered skill is the
denominator opportunity for that series. If the stream is missing, partial,
unsupported, interrupted, not finalized, or corrected/conflicted, classify
that assignment-skill opportunity as **unknown**: show its count and coverage,
and exclude it from both numerator and denominator. A delivery failure is not
a no-use observation. Self-report never becomes adapter observation, and
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
An outcome comparison supports a preference only with at least three eligible
assignments in each compared configuration arm, across at least two distinct
runs per arm, using the same exact cohort key. For a skill-specific
comparison, the arms must differ only by that candidate skill's presence or
version; every other selected skill and configuration dimension stays fixed.
If the required support is absent, report “insufficient comparable evidence”
and leave task-fit ordering unchanged. These thresholds are conservative v1
defaults, not tunable repository policy.

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
logs or reviewer evidence. The summary is sorted deterministically and capped
at 32 KiB, 64 cohorts, 32 candidates, and five representative evidence
references per candidate. Each candidate includes the exact configuration
delta, positive/negative/unknown counts, coverage, confidence category,
cohort-key digest, evidence references, exclusions, and derivation version.
If any cap is reached, report truncation and retain deterministic ordering;
never silently select a favorable subset. A missing, empty, unavailable,
corrupt, stale, or truncated-without-the-candidate summary contributes no
preference and falls back to task fit plus mandatory rules.

## Bounded summaries supplied to the orchestrator

The summary is an internal read-only input to planning. The orchestrator first
filters configurations by explicit task fit, user constraints, mandatory
skills/instructions, capability support, and effective permissions. Learning
may order only otherwise-eligible optional choices when its evidence threshold
is met. It cannot broaden access, choose an unsupported harness, bypass a
required review, or override user preferences. Missing history behaves
exactly like no learned preference.

Every proposed configuration that used learning includes a concise reason:
which optional skill/settings changed, the exact matched cohort dimensions,
per-arm assignment/run counts, outcome counts, usage coverage/unknown count,
source digests, and why alternatives were not selected. The plan binds the
complete immutable proposed configuration. The normal explicit plan review
and approval is still required. No change reaches an active assignment,
bootstrap, prompt, skill file, or permission profile automatically.

## Explainable future configuration choices

For a supported cohort, prefer only the configuration arm with stronger
eligible outcomes under the thresholds above. If outcomes tie, conflict, or
remain unassessed, learning is neutral and ordinary task-fit/user preference
ordering decides. Usage frequency alone never ranks configurations. Preserve
the reason and matched/unknown counts with the later proposal so a reviewer
can see whether its evidence still applies. A newly invalidated source causes
the proposal to be rebuilt or withdrawn before approval; it never silently
keeps the old ranking.

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
never relabel assignment evaluations as observations.

The learning evaluator may raise a one-assignment hypothesis, but a recurring
skill-update proposal requires at least three distinct eligible assignment
subjects across at least two runs, with the same normalized finding and target
surface. A related finding supported only by session friction may continue
through the existing observation pipeline and its own eligibility rule. If
both pipelines identify the same target and normalized fingerprint, merge the
visible evidence in the same `ImproveWorkflow` family instead of creating two
cards. Stable dedupe identity uses repository binding, target surface, stable
skill identity/current snapshot where applicable, and normalized finding
fingerprint; it never uses generated prose.

Choose the smallest plausible target: docs for a missing fact/convention, a
skill for a reusable procedure with checkpoints, instructions for broad
repository guidance, or another existing surface only when its current
contract supports it. The recommendation contains an edit outline and
evidence, not a ready-to-apply patch or a committed artifact. “Fix with AI”
hands the user-reviewed task to the active session. The user reviews resulting
file changes and decides whether to commit/promote them. No approval edits
repository files by itself.

Dismissal is remembered in the existing immutable recommendation history.
Equivalent evidence is suppressed until materially new eligible subjects
change the normalized evidence fingerprint or increase qualifying recurrence
by at least two assignments, including one from a run absent from the prior
watermark. Time passing, rerunning analysis, a changed model-generated title,
or replaying the same evidence is not new evidence. A resurfaced successor
links the dismissed recommendation and shows exactly which assignments,
outcomes, coverage changes, or normalized target changes crossed the
watermark. Rejection does not delete history or retire a skill permanently.

## Retention, forgetting, and concrete examples

The shared repository learning store has hard v1 limits of 1,000 assignment
subjects and 2 MiB total serialized learning records per repository binding;
each subject record is at most 8 KiB, each aggregate is at most 64 KiB, and
there are at most 64 live cohort aggregates. Counts are checked before
publication. At pressure, reject new learning contributions with a visible
capacity state; never evict a retained dependency, lower a threshold, or
silently discard the oldest evidence. These are learning-store caps, separate
from and no larger than the source contracts' per-workspace usage/evaluation
caps.

Retain a learning subject and its source dependencies for at most 180 days
after its assignment becomes terminal, consistent with #743's aggregate
maximum. At that limit, remove the contribution and recompute affected
aggregates before releasing any #743/#744 dependency. If another current
evaluation, reviewer-credibility chain, or retained learning contribution
still depends on an evaluation/result, preserve it under the source contract
until that dependency is explicitly removed. Raw skill events still obey
#743's shorter 30-day maximum; a learning contribution whose usage source has
expired becomes unknown for usage and cannot recreate that source from an
aggregate. Never retain a learning aggregate past the source evidence needed
to explain its claim.

An Electron-authorized user may explicitly forget repository learning
history. Under the repository lock, first fence new learning reads/writes,
remove derived aggregates and pending candidates, and withdraw proposed
recommendations that depend on the cleared history using their existing
terminal lifecycle. Preserve dismissed/completed recommendation history as
history with no active learning input. Then release source dependencies that
are no longer needed; #744's exact-subject purge remains conditional on its
subject revision and the absence of current-result, reviewer-chain, or other
learning dependencies. If a dependency remains, report it and leave the
protected source intact. Forgetting does not reset repository identity,
reassign records, or affect other repository IDs. Workspace deletion purges
that workspace's assignment/usage sources and atomically removes its learning
contributions from the shared repository store; it does not erase other
worktrees' contributions. If the cross-store transaction cannot commit,
deletion remains fenced and reports recovery-required rather than leaving a
summary that cites deleted evidence.

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
- Exact cohort equality accepts only the approved dimensions; unknown model,
  missing task scope, changed permissions, different rubric/criteria digest,
  changed other skill, or unsupported tool remains unmatched.
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
- Mandatory skills cannot be removed; usage-only evidence cannot rank skills;
  plans show matched dimensions, both-arm counts, unknown coverage, source
  digests, and deterministic fallback on missing/corrupt history.
- Skill proposals require three distinct eligible assignments across two
  runs, dedupe across the same normalized target, use the existing
  `ImproveWorkflow` lifecycle, suppress unchanged dismissed evidence, and
  expose the precise evidence delta on a valid successor.
- Forgetting, workspace deletion, source expiry, explicit #744 purge, and
  repository retirement do not leave stale summaries or remove protected
  current evaluation/reviewer dependencies; recovery after an interrupted
  cross-store transaction is idempotent.
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
and time/cost amendment boundary as consequential choices.

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
