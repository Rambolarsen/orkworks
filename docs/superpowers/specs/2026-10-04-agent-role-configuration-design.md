# Agent role configuration and coding-tool permissions

- Status: proposed contract; drafting authorized, written contract review pending
- Date: 2026-10-04
- Tracking: [#741](https://github.com/Rambolarsen/orkworks/issues/741)
- Product direction: [Agent hierarchy and configuration learning](2026-10-04-agent-hierarchy-and-configuration-learning-design.md)
- Launch baseline: [Orchestrated child sessions](2026-09-26-taskmaster-orchestrated-child-sessions-design.md)
- Eligibility evidence: [Coding-tool role capabilities](../../validation/agent-role-capabilities.md), [#740](https://github.com/Rambolarsen/orkworks/issues/740)

## Scope and status

Define the configuration selected by an orchestrator for one assignment:
role instructions, task inputs, skills, rubric, coding tool/model, and effective
coding-tool permissions. Configuration is immutable once approved. Evidence
about delivery, usage, and outcomes lives in separate records.

This is a proposed component contract, not an implemented API or a declaration
that a coding tool is eligible. The authoritative ordinary-child scope is aligned
under accepted ADR 0077 and PR #747; #610 remains open for gated runtime work.
The capability
register currently supplies no verified launch slice. Review of this document,
the upstream launch scope, and a scoped execution plan is required before code.
No accepted ADR is amended or superseded by this draft.

### Current source and intended seam

| Source | Established behavior | Proposed extension |
| --- | --- | --- |
| `crates/orkworksd/src/taskmaster/coordinator.rs:181` | Data-only `PlanNode` records contain role, context digest, criteria, scope, tools, and provider choices | Bind a separate versioned configuration into the approved definition; do not reinterpret existing tool lists as enforcement |
| `crates/orkworksd/src/taskmaster/coordinator_store.rs:50,76` | `StoredPlan` and `CoordinatorStore` retain durable definitions and approval state, with no runtime authority | Persist immutable configuration snapshots with definitions and mutable delivery evidence separately |
| `crates/orkworksd/src/session_application.rs:216` | `CreateSessionCommand` carries harness, model, initial prompt | A separately reviewed orchestration launch path consumes a validated configuration; ordinary creation remains unchanged |
| `crates/orkworksd/src/harness/definition.rs:12` | Harness definition declares launch/resume/integration capabilities | Versioned adapter eligibility evidence and deterministic role/permission mapping |
| `crates/orkworksd/src/harness/registry.rs:44` | Resolved definition and effective capabilities snapshot | Pin that definition plus executable/version/configuration evidence; registry membership alone is insufficient |

These are source investigation references at branch base
`18936e0f83d37ae9bf16ee672e961e14b646a11c`, rechecked against the current
`origin/main`; they are not new production types. A later implementation plan
must choose exact modules and wire routes after contract review.

## Configuration identity and canonical digest

### Approved logical records

Wire names below use camelCase. Version 1 rejects unknown fields, duplicate JSON
keys, duplicate IDs, unsupported enum values, non-finite numbers, and oversized
input before persistence or process creation. There are no numeric floats in
the definition. IDs are nonempty ASCII `[A-Za-z0-9._-]`, at most 128 bytes.
Content digests are lowercase SHA-256 hex, exactly 64 characters.

| Record | Required fields and meaning |
| --- | --- |
| `RoleTemplateSnapshot` | `id`, `version` (positive integer), `role`, `instructions`, `instructionsDigest`, `contentDigest`, `provenance` |
| `SkillSnapshot` | `id`, `version` (nonempty UTF-8 label, at most 128 bytes), `content`, `contentDigest`, `requirement` (`mandatory` or `optional`), `selectionReason`, `requirementSources`, `resourceIds`, `provenance`; referenced instruction resources use the separate records below |
| `SkillResourceSnapshot` | `id`, `skillId`, `sourceReference`, `content`, `contentDigest`, `provenance`; an instruction resource owned by exactly one selected skill, with no independent rule authority |
| `RuleSnapshot` | `id`, `sourcePath`, `content`, `contentDigest`, `precedence` (unique integer `0..31`), `authority` (always `binding`), `provenance`; scoped applicable instruction files are explicit inputs |
| `RequirementManifest` | `sourceId`, `sourceDigest`, `taskId`, `requiredSkillIds`, `sourceLocations`, `applicabilityReason`; resolved requirement annotations separate from source snapshot content |
| `CriterionSnapshot` | `id`, `requirement` (`required` or `optional`), `description` |
| `RubricSnapshot` | `id`, `version` (positive integer), `dimensions` (ID/description pairs), `evaluatorRole`; rating/calculation belongs to #744 |
| `AdapterBinding` | `harnessId`, `definitionDigest`, `adapterId`, `adapterVersion`, `executableIdentity`, `toolVersion`, `platform`, `instructionMechanism`, `effectiveSettingsDigest`, `evidenceId`, `evidenceDigest` |
| `ModelBinding` | `schemaVersion` (1), `mode` (`pinned` or `tool-managed`), `modelId` (required for pinned, null for tool-managed), `policyId` (adapter-recognized policy), `policyDigest`, `adapterGeneration` (opaque adapter/capability identity, stable across observer restarts); part of the immutable configuration |
| `AssignmentConfiguration` | `schemaVersion`, `configurationId`, `repositoryId`, `repositoryBinding`, `sourceWorktreeBinding`, `workspaceId`, `parentSessionId`, `planId`, `planRevision`, `taskId`, `assignmentKind`, `roleTemplate`, `taskCategory`, `assignment`, `rules`, `requirementManifests`, `skills`, `skillResources`, `rubric`, `harness`, `capabilityEvidence`, `model`, `permissions`, `renderedInstructions`, `renderedInstructionsDigest`, `configurationDigest` |
| `VersionRetirement` | Separate source-catalog tombstone, never part of an assignment digest: `schemaVersion`, `sourceIdentity`, `artifactKind`, `artifactId`, `version`, `contentDigest`, `retiredAt`, `reason` |

`configurationId` is stable only within the immutable plan revision. A changed
configuration gets a new ID and digest. Repository/workspace/plan/task identities
are those supplied by the approved baseline, never guessed from a display name.
The [repository-identity portion of #745](2026-10-04-configuration-learning-design.md)
is now a proposed review input; an unresolved binding prevents comparison and
launch. Its proposed amendment adds required `repositoryBinding` and `sourceWorktreeBinding`
to both child and bootstrap configurations and binds its digest into the run and every exact
plan/parent binding. The complete descriptor participates in canonical digests,
inclusive byte bounds and protected snapshot retention; `repositoryId` must
match it. The source worktree snapshot binds the existing root and private Git
directory objects; another same-family worktree cannot replace that source.
Its digest is also retained by every plan/parent binding. Existing records without the reviewed binding cannot become eligible
through path/name inference. This amendment and platform evidence remain gated.

`assignmentKind` is `child` for the plan-bound `AssignmentConfiguration`. The
parent uses the separate run bootstrap and plan binding below. `assignment` includes a short
label, description, input references/content digests, expected output contract,
criteria, dependencies, and approved worktree-group/base/scope bindings. A child
has the baseline task identity. The parent has one immutable run assignment
across its research and execution plans; per-plan identity is coordination data,
not a replacement startup instruction set. The [preparation contract](2026-10-04-orchestrator-preparation-design.md) defines the proposed authority lifecycle.

`AdapterBinding.executableIdentity` records canonical executable path and the
content identity of the executable/runtime package actually invoked. Interpreter
or wrapper-based tools must pin the resolved wrapper, interpreter and package
manifest/content identities; a path or version label alone is insufficient.
`effectiveSettingsDigest` covers the enumerated launch flags and nonsecret
applicable startup settings. It does not include credential values.

`provenance` records `sourceKind` (`repository`, `bundled`, or `user-approved`),
a stable `sourceIdentity` for the owning catalog, `sourceRevision`, and, for a
local file, its canonical source path. It contains no credential. Skill ID denotes
a logical skill; version plus content digest identifies the content actually
approved. Same name/version with different bytes is drift, not the same historical
configuration.

`model` is a `ModelBinding`, never an implicit default. In `pinned` mode, the
adapter must establish the requested model ID; if it cannot, that combination
is unverified and cannot launch as pinned. In `tool-managed` mode, `policyId`
names an explicit approved adapter policy (for example that tool's `auto`), and
`policyDigest` binds its nonsecret settings and supported semantics. It does
not promise a fixed inference backend. `adapterGeneration` is the opaque,
bounded ASCII identity for the exact adapter-capability snapshot resolved for
this configuration, including its adapter/tool binding, profile/evidence
identity, and supported model policy. It is the immutable adapter/capability
identity consumed by #743, not a model observation or a process counter. It
stays unchanged when an observer process restarts; #743 uses
`producerStreamId` for that producer-process incarnation, while
`launchGeneration` identifies the child runtime. Changing the adapter, tool
version, effective profile, evidence snapshot, or supported model policy
requires a new configuration and `adapterGeneration`. A model observation
within an unchanged approved tool-managed policy does not change this identity.

A separate version-1 `ModelObservation` records `configurationDigest`,
`sessionId`, `launchGeneration`, `observationId`, `observedAt` (UTC), `state`
(`known` or `unknown`), `modelId` (required only for known), `source`
(`adapter-reported`, `agent-reported`, or `unavailable`), and bounded `reason`.
Each observation is at most 4 KiB, has IDs/field limits below, and contains no
provider credential. Unknown has null `modelId` and a reason; `unavailable` source
requires unknown state. Only adapter evidence can establish pinned delivery; an
agent report cannot promote an unverified model into a verified binding. Unknown is explicit; tool-managed does not establish which
model was used. Agent claims stay reported. These mutable observations do not
change the configuration digest; #743/#745 define bounded aggregate persistence.

Launch/resume revalidates the binding's ID/policy, capability generation and
settings. A changed pinned model, changed policy/settings or incompatible
capability generation needs a revised proposal. An observed model change within
the verified semantics of an approved tool-managed policy stays in that policy,
with separate observation provenance; it does not silently become pinned or
cross into another provider policy. An unknown actual model under tool-managed
is allowed only if unknown visibility was part of approval; under pinned it
blocks verified delivery. Evaluations/learning retain approved binding and
observations separately, never pooling unknown into a known-model cohort.

Each root/child assignment contains 1–32 criterion snapshots, including at least
one `required` criterion. Optional-only or empty sets fail validation before
approval; optional criteria cannot become a zero-denominator completeness rubric.

### Referenced skill instruction resources

Both child and bootstrap configurations contain `skillResources`, an inline
array of `SkillResourceSnapshot` records (empty when none are needed).
`SkillSnapshot.resourceIds` names that skill's complete context-relevant
resource closure, including transitive references, sorted by resource ID.
Every resource has one selected `skillId`; its ID must occur exactly once in
that skill's list. Reject dangling, duplicate, unowned or cross-skill references.
The same source needed by two skills gets distinct owner-bound resource IDs.
Resolve the declared closure before approval; cycles are visited once, and an
unresolved reference or excess closure blocks approval. A runtime reference
cannot fetch additional instruction bytes or broaden access without a revised
configuration. Non-instruction code/data inputs use the declared assignment
input/permission contract and cannot serve as hidden instruction resources.

`sourceReference` identifies the reviewed locator relative to the skill source
or an explicitly approved source URI; `provenance` retains its resolved source
identity. These references do not grant permission to read or fetch a source.
`content` is the exact retained UTF-8 instruction text; `contentDigest` hashes
those exact bytes without a prefix or normalization, as for skill content.
Resources inherit the containing skill's composition position and conflict
checks; they cannot become binding repository rules by being placed in `rules`.
Render a skill followed by its resources in resource-ID order. Include every
resource descriptor and byte in canonical configuration/bootstrap serialization,
rendered-context and inclusive descriptor limits, source drift revalidation,
delivery receipts and protected snapshot retention. A changed resource byte or
source binding changes the configuration digest and needs new approval.

### Canonicalization

1. Validate the complete descriptor and each referenced snapshot. Hash snapshot
   content as exact UTF-8 bytes; preserve Unicode, line endings, and whitespace.
   `instructionsDigest` is SHA-256 of the exact `RoleTemplateSnapshot.instructions`
   UTF-8 bytes, with no prefix, newline insertion, normalization or JSON quoting.
   `renderedInstructionsDigest` likewise hashes exact rendered UTF-8 bytes.
   Resolve files once before approval and retain their exact bytes.
2. Build the definition object without `configurationDigest`. No bearer tokens,
   launch reservations, child IDs, timestamps of execution, telemetry, evaluator
   outcomes, or mutable support state are included.
3. Normalize only collections declared as sets (skills and skillResources by ID,
   each skill resourceIds list by ID; tool/action and
   permission path entries by their full validated identity) before rendering.
   Reject duplicate entries. Preserve meaningful instruction precedence,
   command argv, criteria and dependency ordering; do not silently reorder them.
   Snapshot `contentDigest` for skills/rules/resources is the digest of exact content bytes;
   a role template's `contentDigest` hashes its canonical descriptor excluding
   that field. Profile digests hash their normalized canonical profile bytes.
4. Serialize recursively: object keys sorted by UTF-8 byte order; arrays retain
   their validated order; compact JSON with UTF-8 strings and deterministic JSON
   escaping; integers decimal, no leading zero. This matches the existing
   coordinator's recursively sorted JSON approach without implying a generic
   floating-point canonicalization standard. Version 1 pins the serializer;
   cross-language implementations need shared byte fixtures.
5. Compute SHA-256 of `orkworks.assignment-configuration.v1\n` followed by those
   bytes. The prefix is a literal UTF-8 newline, not backslash-n in the hashed
   stream. Include this digest and the referenced snapshots in the immutable
   plan definition, so the existing exact-plan approval covers them.

### Run-bound orchestrator bootstrap

`OrchestratorBootstrapConfiguration` uses `schemaVersion` (1), `bootstrapId`,
`repositoryId`, `repositoryBinding`, `sourceWorktreeBinding`, `workspaceId`, `parentSessionId`, `runId`, `rootAssignmentId`,
`goalDigest`, `roleTemplate`, `taskCategory`, `assignment`, `rules`,
`requirementManifests`, `skills`, `skillResources`, `rubric`, `harness`,
`capabilityEvidence`, `model`, `permissions`,
`renderedInstructions`, `renderedInstructionsDigest`, and
`bootstrapConfigurationDigest`. It has no future plan/task revision or bearer.
The assignment describes the immutable user goal and coordination outputs,
including clarification, research synthesis and preparation of execution.
Its requirement manifests use `rootAssignmentId` as their `taskId`; it cannot
be mistaken for a launchable child task. Role must be `orchestrator`.

Apply the composition, byte/digest and bounds rules above, excluding the
bootstrap digest field before canonical serialization. Hash
`orkworks.orchestrator-bootstrap.v1\n` (one literal LF after `v1`) plus those
canonical bytes. The run definition binds this digest; the UI approves the
bootstrap, goal and run ceiling at parent creation. A child configuration keeps
the assignment prefix and exact plan/task bindings above.

Each `ParentPlanBinding` includes `runId`, `parentSessionId`, `planId`,
`planRevision`, `runDefinitionDigest`, `bootstrapConfigurationDigest`,
`repositoryBindingDigest`, `sourceWorktreeBindingDigest`, `preparationRevision`, and `inputDigest`. It is part of that plan's immutable
approved definition. It reuses the original startup bytes, permission profile,
skills and model policy; reports and approved tasks are dynamic coordination
inputs, not system-prompt/configuration replacement. Both the 2 MiB plan limit
and retained bootstrap validation apply to its referenced snapshots.

An already running parent cannot change its instructions, loaded skills or
permissions through a new binding. A change fences/cancels that run and requires UI-authorized creation of a new
run under a reviewed bootstrap; no old plan approval transfers. Exact resume
of the current run requires unchanged verified bytes/settings, exact delivery
and fresh runtime identity. No child receives coordination rights merely from a role name.
The preparation contract's run bearer is planning authority; a matching current
exact-plan execution grant is separately required to launch a declared child.

### Instruction-byte fixture

For instruction text exactly `Delegate.` followed by one LF, the UTF-8 bytes
are `44 65 6c 65 67 61 74 65 2e 0a`, length 10. Its `instructionsDigest` is
`52bd270085f34b76516831f685fe272ee7b36f45ff3ff6a37abfbdd66b4942d3`.
Removing that LF changes the digest. The fixture is a specification value,
not a runtime test or a skill invocation. Full descriptor/serializer fixtures
are required in the later scoped implementation plan.

## Repository, role, and assignment composition

The sidecar renders OrkWorks-supplied instructions deterministically, in this
order, with labeled section boundaries and content identities:

1. Coordination/reporting boundaries and applicable repository instruction
   snapshots in their declared precedence order, including scoped rules.
2. Selected role template.
3. Assignment inputs, scope, dependencies, acceptance criteria, output contract,
   rubric, and access-escalation/reporting requirements.
4. Mandatory skills, then optional skills, each ordered by ID; referenced
   instruction resources come from the explicit `skillResources` array and are
   rendered immediately after their owning skill.

Every `RuleSnapshot` is binding; advisory research/context belongs in assignment
inputs, not a downgraded rule snapshot. `authority` must equal `binding`. Discover
applicable repository rules from the approved root router and scoped instruction
paths for the assignment, recording each resolved source and precedence. Version 1
defines no rule-override operation: scoped rules add constraints to inherited
rules, and a contradiction between simultaneously applicable binding rules blocks
approval with the conflicting source IDs. A later section or deeper path cannot
silently erase or weaken an inherited rule. Resolving a conflict requires changing
the source rules and proposing a new configuration. The stored `precedence` is a
unique zero-based render-order index assigned from the root router's declared
order, followed by applicable scoped sources from shallowest to deepest; sources
at the same scope without a declared order are sorted by normalized source path in
UTF-8 byte order. This order is digest-bound but never resolves a contradiction.

Each applicable rule and role template has one reviewed `RequirementManifest`
for this assignment. Its `sourceId`/`sourceDigest` bind the exact source snapshot;
its `requiredSkillIds` may be empty only with an explicit applicability reason.
The mandatory set is the union of those manifests, resolved
to exact selected skill snapshots. Each mandatory skill's `requirementSources`
references all contributing rule/template IDs and bounded source locations;
optional skills have no requirement source. The validator rejects a missing
mandatory ID, an optional label for a required skill, an unresolved source, or
an unapproved manifest change. Manifests and precedence are digest-bound configuration inputs. Assignment-specific
manifest resolution does not mutate a reusable source template or its content digest.

Determining applicability/conditional requirements in natural-language source
rules is a planning responsibility proposed by the orchestrator and reviewed
with the configuration. Structural validation does not prove that an agent
correctly interpreted arbitrary prose. A requirement that cannot be resolved
with a source-backed manifest is a clarification blocker, not an optional skill.

Order is not permission to override mandatory instructions. A conflicting role,
skill, or assignment is rejected with the conflicting source IDs and explanation;
the system does not delete a binding repository rule to make a launch possible.
Report substantive conflicts to the user for a revised proposal. Optional skill
selection cannot waive a mandatory skill.

Pin the instruction files that the selected tool automatically discovers, in
addition to supplied sections. If a tool independently loads additional global,
ancestor, nested, agent, or skill instructions, the adapter must enumerate their
identities and precedence or mark delivery/launch unverified. Unknown private
vendor system instructions are outside the snapshot; preserve the vendor's
policies and never claim to capture or replace them.

`instructionMechanism` is `native-agent` or `startup-context`. UI details state
which was used. A startup prompt is not described as a native system prompt.
The adapter supplies approved content before substantive assignment work; a
failed/unknown required delivery prevents execution. Optional skill delivery is
also required when the skill is in the approved configuration; dropping it needs
a revised configuration. Automatic tool-side compaction or context recall is
not proof that the skill remains loaded for the entire run.

Configuration snapshots may contain local source/task instruction text and are
private approval artifacts, governed by bounded local storage. They are not
usage telemetry and are not copied into usage/evaluation reports. Secret values,
full conversation transcripts, hidden reasoning, and private vendor instructions
are forbidden. References to credentials state the required access mechanism,
never the secret. Reject inputs containing a known secret reference that cannot
be represented without its value; do not promise perfect secret detection.

## Role templates and mandatory skills

Templates are reusable and versioned. Their task-specific fields are filled
from the approved assignment. Role is independent of visual portrait and skill
usage/evaluation; no role accumulates permanent XP.

| Role | Required output and stopping conditions | Default capability ceiling |
| --- | --- | --- |
| `orchestrator` | Material questions, bounded proposals, configuration choices, approved coordination, synthesis of declared child reports; stop for changed scope or missing authority | Declared coordination metadata/report reads and approved plan operations only; no codebase investigation, product editing, shell, external research, or review work |
| `research` | Answer declared questions with source references, findings, uncertainty; stop at unknowns that need broader access | Declared repository/source reads and, if explicitly approved, external search/fetch; no edits, arbitrary shell or native agent delegation |
| `implementation` | Deliver declared changes and evidence against acceptance criteria; report missing permission rather than widening it | Declared file reads/edits and approved development commands with stated effects; no Git mutation or native agent delegation |
| `review` | Findings with locations, severity, evidence, and result revision; never evaluate itself | Declared artifact/evidence reads; no product edits, shell by default, or native agent delegation |
| `verification` | Declared check result, inspected revision, evidence and limitations; missing checks stay unassessed | Declared checks and explicitly approved generated/build/test output; no unrelated product edits or native agent delegation |
| `remediation` | Resolve exactly the declared finding scope and provide evidence; new findings/access need escalation | Same controls as implementation narrowed to the finding; no additional undeclared repair task |

For a parent proposing parallel work, the orchestrator template's required-skill
manifest includes `orchestrating-task-graphs`, with its version/content digest
and selection reason. Include it in the reviewed bootstrap configuration for
such a parent. Its product-facing instruction composition preserves parent-only
delegation, ordered batches, exact approval, coding-tool permissions and manual
integration. Guidance to synthesize reports is coordination; guidance cannot
grant rights to merge code or perform child implementation/review work.
A conflicting raw skill instruction must be resolved in a reviewed role-compatible
version before launch, not treated as broader authority.

Mandatory skills come from the applicable repository instructions and approved
role template. The orchestrator may add optional task-fit skills and record
selection reasons. A skill can supply guidance and referenced data; invoking its
script is a command request subject to the permissions profile. It cannot add
shell, connectors, native subagents, or broader filesystem access.

For a role whose required repository workflow entails forbidden effects, reject
the combination until a reviewed role-specific workflow or revised assignment
resolves the conflict. For example, a verification role requiring build output
cannot be represented as a profile that denies all writes. A repository mandate
to commit from a child is incompatible with the launch proposal's no-Git-mutation
contract and must be resolved before launch, not silently ignored.

## Requested and effective permission profiles

`permissions` contains `requested` and `effective` profiles. A requested profile
expresses the assignment's intended actions/resources. The adapter produces the
effective profile, concrete tool settings, and evidence of their mapping before
approval. Requested and effective values may differ only by narrowing that
still satisfies the assignment; unresolved loss of required functionality is
ineligible. Both are visible in approval details.

| Profile field | Contract |
| --- | --- |
| `tools` | Allowlist of adapter-recognized action IDs; unknown or wildcard tool entries rejected |
| `readPaths`, `writePaths` | Exact file or explicit subtree entries, bound to an approved repository/worktree root; canonical paths shown in approval |
| `commands` | Exact executable identity, argv, cwd, declared output/effects, and a digest-bound `environmentPolicy`; no wildcard shell string or ambient environment inheritance |
| `externalSources` | Explicit source/host and action policy for search/fetch; redirected destinations must satisfy the same policy |
| `connectors` | Exact server/connector identity and approved read/action scope; absent means denied, not inherit unrestricted MCP |
| `coordinationActions` | Only declared parent-plan operations; child reports have assignment-scoped identity, not launch rights |
| `nativeDelegation` | `denied` in version 1, including skill-triggered or tool-native subagent execution |
| `accessChanges` | `request-only`; no child modifies approval, permission settings, profile, tool/model choice, or its own effective instructions |

### Command environment policy

Every command-enabled requested/effective profile includes an explicit
`CommandEnvironmentSnapshot`: `mode` (exactly `explicit`), `nonsecretBindings`
(name/exact value pairs), `credentialBindings` (name, stable `slotId`,
`sourceIdentity`, allowed consumer/scope and `rotationPolicyId`), and
`policyDigest`. Hash its canonical descriptor excluding `policyDigest`; include
that digest and descriptor in the approved command/profile/configuration.
A credential slot identifies an approved source and use, never a secret value.
Secret bytes are resolved at execution and excluded from digests, reports and
logs; a slot/source/scope/rotation-policy change requires a revised approval.
Rotation within its explicitly approved policy does not expose secret bytes.

### Safe permission defaults

Role capability ceilings above are maximums, not defaults or grants. The
version-1 default requested profile for every role has empty allowlists for
tools, filesystem paths, commands, external sources, connectors, and coordination
actions; `nativeDelegation` is `denied` and `accessChanges` is `request-only`.
A proposal may add only the task-specific resources and actions needed to satisfy
the declared assignment, and must name them explicitly in the requested profile
before approval. For example, a research proposal names its exact repository
paths or external sources; an implementation proposal names exact read/write
paths and command invocations; a review proposal names the artifact/evidence
paths; a verification proposal names each allowed check and its declared
outputs; and a remediation proposal names the finding-scoped paths and commands.
The orchestrator may name only
coordination metadata/report reads and plan operations needed for the declared
parent assignment. These additions remain within the role ceiling, are subject
to capability evidence, and require user approval. Missing task-specific scope
stays denied; neither role selection nor task text creates ambient access.

The verified adapter constructs each command environment from this snapshot,
starting empty. Unlisted ambient variables are removed rather than inherited.
Required platform variables, working-directory/settings/search-path values and
behavior-changing bindings must be explicit nonsecret approved values. In
particular, `BASH_ENV`, `ENV`, `PYTHONPATH`, `NODE_OPTIONS`, `RUSTC_WRAPPER` and
loader variables cannot arrive through ordinary-session inheritance. If needed,
they require exact approved values plus content identities/scope for referenced
startup scripts, modules or executable wrappers. A credential slot cannot be
used to hide a behavior-changing nonsecret setting. The adapter validates its
recognized credential-slot semantics; unknown slots are ineligible.

Validate actual environment construction immediately before every command.
An adapter unable to remove unlisted ambient variables or bind effective values
cannot offer a command-enabled restricted profile. Exact executable/argv alone
is insufficient; no prompt-only claim or OS confinement is substituted. This
narrows coding-tool command execution, while ordinary session creation and the
baseline's provider-login environment remain unchanged. Parent inference/login
credentials do not implicitly authorize forwarding them to task commands.

Provider inference traffic inherent in the approved coding tool is described
in adapter evidence, separate from agent-requested external search, shell
networking, and connector actions. This contract does not proxy credentials or
control the provider's internal infrastructure. Inherited ordinary-session
credentials remain governed by the baseline; they do not grant connector or
agent-requested network actions in the effective profile.

Permission path entries are tagged descriptors: `{ "kind": "worktree-root" }`
means the entire exact approved worktree/repository root and has no `path` field;
`{ "kind": "file" | "subtree", "path": "..." }` names a descendant.
Reject unknown fields/kinds. Show the canonical absolute bound root in approval;
the root descriptor cannot mean another worktree, a home directory or the
filesystem root. Descendant paths are root-relative, use `/`, and reject absolute,
empty, `.`/`..`, NUL, and drive-qualified components. Canonical launch paths are
resolved against the approved exact worktree root and recorded separately.
Reject symlink/reparse-point escape from the approved scope, including a missing
write target whose nearest existing ancestor resolves outside it. Case and
filesystem identity follow the launch platform. Canonicalization does not create
an OS sandbox: after-launch race/effect coverage must be stated in the adapter
matrix. If an assignment requires a guarantee that the tool cannot enforce,
it is unavailable under that tool/profile.

Do not infer command effects from executable names. Arbitrary shell can write,
read credentials, call network endpoints or start processes. Denying edit tools
while leaving arbitrary shell is not read-only. A check command can execute
repository-authored scripts and produce generated files: its intended effects
must be declared and the coding-tool mapping must prove the requested controls
or mark the profile ineligible. No server broker, OS process isolation, or
hard CPU/token/cost ceilings are added by this contract.

Coding-tool controls apply to the selected tool's actions. They do not protect
against a same-user process directly calling ordinary sidecar endpoints, reading
another process's environment, or spoofing a same-session report. UI permission
details state that boundary. Authenticated reports prove session association,
not instruction adherence or work quality.

## Adapter eligibility and drift

Eligibility is a persisted evidence decision, not a checkbox inferred from
login, a resume recipe, model availability, tool installation, or working hooks.
Key it by exact tool version/executable identity, adapter version, platform,
instruction mechanism, effective settings digest, role, and profile digest.
`verified` requires primary schema/reference, exact version evidence, and a
reproducible bounded capability fixture for the exact combination. It is the only
decision that can make that exact profile eligible for user approval; approval and
the launch-time checks below are still required. `limited` means evidence covers
only a narrower capability set than the requested profile; it is ineligible, and
any narrowed proposal requires new profile/configuration digests and its own
evidence decision. `unverified` means evidence is absent, incomplete, ambiguous,
or stale. `unsupported` means evidence establishes that a required behavior for
the exact profile cannot be enforced. Neither `limited`, `unverified`, nor
`unsupported` can launch a restricted role.

### Immutable capability evidence snapshot

`capabilityEvidence` is one inline `CapabilityEvidenceSnapshot`, separate from
mutable delivery/usage observations. Required fields are `schemaVersion` (1),
`evidenceId`, `harnessId`, `definitionDigest`, `adapterId`, `adapterVersion`,
`executableIdentity`, `toolVersion`, `platform`, `instructionMechanism`,
`effectiveSettingsDigest`, `role`, `requestedProfileDigest`,
`effectiveProfileDigest`, `decision` (`verified`, `limited`, `unverified`, or
`unsupported`), `checks`, `references`, `assessedAt` (UTC), `assessorIdentity`,
and `evidenceDigest`. The executable/settings fields use exactly the same
representations as `AdapterBinding`; profile digests use the canonical normalized
requested/effective permission profiles defined above. `evidenceId` equals the
binding's `evidenceId`, and every shared key plus the role/profile digests must
match the configuration. Do not include a configuration digest in this snapshot:
that would create a circular hash dependency.

Each check contains `surface`, `result` (`passed`, `failed`, `unverified`, or
`not-applicable`), `reason`, and unique `referenceIds`. Required unique surfaces
are `instructions`, `skills`, `tools`, `commands`, `filesystem`, `network`,
`connectors`, `delegation`, `startup`, and `model-policy`. A passed check requires
retained reproducible fixture evidence and a primary reference. Not-applicable
requires a source-backed reason and fixture confirming the surface is absent or
denied; it is not a way to omit an untested control. A failed or unverified check
requires a nonempty bounded explanation and source references for the observed
limitation or evidence gap. For a non-verified decision, the support reasons are
the failed/unverified check reasons in surface order; consumers must preserve each
surface and its reference IDs rather than inventing a single summary or dropping
the reason. If no exact snapshot exists, the proposal reports `unverified` with a
bounded reason that cites the capability register or other reviewed source, and
cannot advance to approval. `verified` requires all surfaces passed or legitimately
not-applicable; failed/unverified coverage cannot be authenticated as verified
merely by hashing it. Native usage observation is optional and separate from the
mandatory skill-delivery check.

Each reference contains `id`, `kind` (`primary-reference` or `fixture`),
`sourceReference`, and `contentDigest` (SHA-256 of exact retained artifact bytes).
Checks may only name references in this snapshot. Artifact content is retained
locally and protected with the referencing evidence/configuration; URLs or file
names alone are insufficient. No secrets, hidden reasoning or full production
transcripts are retained. The capability register is source evidence for a
future unverified snapshot; it is not itself a verified permission fixture.

Reject unknown fields, duplicate surfaces/references and dangling reference IDs.
Sort checks by surface, references by ID and referenceIds by ID; use the same
recursive canonical JSON serializer defined above. Exclude only evidenceDigest
and hash `orkworks.capability-evidence.v1\n` (one literal LF) plus those bytes.
`AdapterBinding.evidenceDigest` must equal the recomputed snapshot digest.
Include the complete snapshot in the approved configuration/bootstrap digest.
At approval and launch/resume, recompute the digest, check exact binding/profile
coverage, verify retained reference content and current evidence eligibility.
Changed evidence yields a new ID/digest and proposal, never an in-place patch.
Revocation/invalidation is separate mutable support state checked on every
launch; the immutable historical verified decision cannot override it.

Evidence inventories all applicable startup configuration, tool allow/deny
behavior, shell effects, filesystem access, network actions, connectors, native
subagents, and permission-changing controls. A flag's existence proves only that
its parser/help exposes it. Current hooks prove neither skill delivery nor
invocation coverage. The evidence register currently records a **no-go** for
all six Copilot role profiles; other tools are not verified as substitutes.

A native skill invocation observer is optional for eligibility if instructions
and permissions are verified: lack of observation remains unknown usage.
Content delivery confirmation is mandatory for every selected skill. An adapter
may confirm supplied startup content without claiming native skill invocation.
Skill state projections belong to #743 and must retain that distinction.

## Approval binding, launch validation, and escalation

1. Resolve applicable rules/template/skill content and adapter capabilities.
   Validate requested/effective profiles and produce rendered bytes/digests.
2. Show role, task, selected mandatory/optional skills, instruction mechanism,
   coding tool/model policy, exact paths/actions, effective permissions, support
   limitations, and configuration digest with the plan proposal.
3. Parent creation first requires Electron-authorized approval of the immutable
   bootstrap/goal/run ceiling; later plans bind the unchanged bootstrap.
   Accept only Electron-authorized approval of the exact immutable plan revision
   and its configuration references. An agent report or terminal text cannot
   approve any snapshot. Do not create a worktree or launch a research child
   earlier.
4. Immediately before the existing reserved launch, re-resolve executable and
   version, definition/adapter/config sources, scoped repository instructions,
   template/skill bytes and requirement manifests, approved model binding/capability
   generation, path/base/worktree bindings, requested/effective
   settings, approval generation, and current evidence eligibility. Resume
   performs the same checks; a resumed tool must not reuse an old broad profile.
5. A mismatch, revoked/expired approval, or unknown mandatory delivery blocks
   launch with a bounded reason. Changed content/settings/tool/model/scope
   produces a new proposal; no silent provider or profile substitution.
6. Apply the approved profile before task content is delivered. Record a delivery
   receipt linked to configuration digest, launch reservation, session, and
   generation; retain each supplied skill/content digest. Failed/unknown delivery
   stops substantive work and becomes a blocker, never an automatic retry.
7. During execution, detected settings/instruction drift invalidates eligibility
   for further coordination/launches and marks the affected assignment as
   requiring user attention. Do not pretend detecting drift guarantees immediate
   process quiescence. Ordinary session stop/lifecycle behavior remains the
   baseline's; no new process-tree termination guarantee is implied.

A child needing access sends a bounded blocker containing assignment/configuration
identity, missing action/resource, reason, and evidence reference. It does not
include a credential value. The orchestrator may synthesize a revised proposal;
only the user approves the new revision. Existing children do not have their
instructions or access changed in place. A replacement/retry follows the baseline's
new-revision, terminal-child and quiescence gates. A prompt asking the user to
allow a broader coding-tool action is not OrkWorks plan approval.

A denied or interactive permission prompt leaves the assignment blocked under
its current profile. If the tool lets a prompt permanently broaden its own
policy, that control must be disabled/proven unavailable for eligibility.
User acceptance of a result, a high score, and reported completion do not waive
these checks or authorize additional tasks.

## Bounds, compatibility, and persistence

Proposed version-1 limits, measured as UTF-8 bytes unless stated otherwise:

| Subject | Limit / behavior |
| --- | --- |
| IDs/digests | 128-byte IDs; 64-character digests; `schemaVersion` exactly 1 |
| Generation identities | `adapterGeneration` is a nonempty ASCII ID within 128 bytes; runtime `launchGeneration` follows the preparation contract's opaque generation encoding, never a JSON numeric counter |
| Numeric revisions | `planRevision` and `preparationRevision` are integers 1–64, matching #742; `RoleTemplateSnapshot.version`, `RubricSnapshot.version`, and `taskVersion` are integers 1 through 2^31−1; source revision identities retain their declared string representation |
| Version labels | `SkillSnapshot.version` is a nonempty UTF-8 string at most 128 bytes, compared byte-for-byte without normalization; `AdapterBinding.adapterVersion` and `toolVersion` are nonempty labels within the 512-byte label bound, not numeric revisions |
| Labels/reasons | 512 bytes per label; 2 KiB per reason/source reference |
| Skills/rules/inputs | 16 skills, 32 rule/resource snapshots combined (including skillResources), at most 32 resourceIds per skill, and 32 input references per configuration; 33 requirement manifests, at most 16 requiredSkillIds and 32 sourceLocations/requirementSources per record, each source location at most 512 bytes; unique IDs within each namespace |
| Criteria/dimensions | 32 acceptance criteria and 16 rubric dimensions; 2 KiB per description and output-contract text, 4 KiB assignment description |
| Paths/tools/actions | 64 read paths, 64 write paths, 64 tools, 32 commands, 32 source policies, 16 connectors, 16 coordination actions; path/reference 2 KiB, policy text 2 KiB; dependencies at most 128 unique task IDs |
| Commands | 64 argv elements, 2 KiB per element, 32 environment bindings per command, 128-byte names, 2 KiB nonsecret values; credential slot/source/scope/policy references at most 512 bytes each; no credential values retained |
| Supplied instructions | 256 KiB total rendered context, including all selected instruction content; individual skill/rule/template cannot exceed this total |
| Configuration | 1 MiB serialized descriptor plus its inline snapshots; rendered content counts within that bound |
| Plan | 128 tasks and 2 MiB total approved definition, inclusive of every configuration; lower existing/upstream limit always wins |
| Template catalog | 64 role-template versions per workspace, including pinned historical versions; 1 MiB total |
| Version retirement ledger | 4,096 role-template/skill retirement entries per workspace and 1 MiB serialized; never evict tombstones |
| Capability evidence | One inline snapshot, at most 64 KiB included in the 1 MiB configuration; ten surface checks, 16 references, 2 KiB per reason/source reference; fixture artifacts at most 64 KiB each / 1 MiB total per snapshot, retained separately; no raw production transcripts |
| Blocker/delivery records | At most 16 KiB per record, 16 evidence references and 16 skill delivery entries; idempotency/rate/aggregate retention belong to #742/#743 |
| Admission | Reject oversized/unsupported input before writing or launching; never truncate mandatory content or evict referenced snapshots |

The 256 KiB rendered-context ceiling accommodates the checked-in mandatory
rules: at reviewed commit `e111fd6c`, root `AGENTS.md` is 48,894 bytes, desktop
rules 3,248 and sidecar rules 2,698 (54,840 combined), before role/assignment/skill
content. Include every applicable byte; never truncate rules to fit. A complete
configuration includes snapshots and rendered bytes within its 1 MiB cap.
The 2 MiB plan ceiling still applies to all configurations together, so the
maximum task count is not a promise that every maximum-sized descriptor fits.

These are record/admission bounds, not coding-tool context guarantees or runtime
resource ceilings. A tool with a lower verified startup/context limit lowers
eligibility. Large required skills need a reviewed composition adjustment or
cannot launch; do not silently summarize away mandatory instructions.

Keep immutable configurations with their plan definitions under the existing
workspace metadata owner/lease. Content-addressed deduplication may share bytes,
but changing or deleting a source file does not rewrite an approved snapshot.
Ordinary sessions without orchestration metadata keep their current behavior;
absence of a configuration never opts them into orchestration or restricted roles.
Existing dormant coordinator records are read under their original version and
are never auto-promoted into approved role configurations.

Editing a template/skill creates a new version/digest. Separate, bounded
`VersionRetirement` tombstones use a discriminated union with `schemaVersion` 1,
`sourceIdentity`, `artifactKind`, `artifactId`, `version`, `contentDigest`,
`retiredAt` (UTC), and a bounded non-secret `reason`. For
`artifactKind: role-template`, `version` is an integer from 1 through
2^31−1, matching `RoleTemplateSnapshot.version`. For `artifactKind: skill`,
`version` is a nonempty UTF-8 string of at most 128 bytes, matching
`SkillSnapshot.version`; compare it byte-for-byte without normalization. Reject
any variant whose kind and version type/range do not match. The retirement
identity is the tuple `(sourceIdentity, artifactKind, artifactId, version,
contentDigest)`; a same-named version with different content is a separate
identity. The stable `sourceIdentity` is the catalog identity in snapshot
provenance, not its changing source revision or file path. A tombstone is written
only through an Electron-authorized user action; callers cannot supply or claim
retirement authority. Tombstones are durable and monotonic: a retired identity
cannot be reactivated; corrected or replacement content needs a new version/digest. The
catalog/source identity is taken from snapshot provenance, so retirement does not
affect a same-named skill from another source. Missing or unreadable catalog state
blocks new proposals that depend on it and is reported as an unavailable
retirement check; it is never treated as active.

Retirement blocks selection in new proposals. It does not rewrite or erase an
approved configuration or its history; an existing launch/resume still requires
the exact retained bytes, approval, and current capability evidence to validate.
A reviewed capability-evidence invalidation can independently make an old version
ineligible for resume. Retired versions remain available for interpreting retained
history. The catalog owner is the source identified by provenance, with state
stored under the existing workspace metadata owner/lease; restart cannot restore
an active version from a stale source file. When the retirement ledger reaches its
bound, reject the retirement action without changing catalog state; report the
failure and never evict tombstones or referenced snapshots.
Referenced snapshots survive normal session retention while their plan/allocation
records remain protected. On explicit eligible plan deletion, remove unreferenced
configuration blobs after ownership checks; never remove plan-owned worktrees
or branches through snapshot deletion. Storage exhaustion with pinned records
blocks admission, not proof retention. Broader learning-history retention and
repository identity across worktrees belong to #745.

## Contract examples and verification cases

### Permission projection example

The following is a proposed fragment, not a callable endpoint. Snapshot digests
and full definition are supplied separately; this fragment cannot authorize a
launch. It illustrates a review assignment that inspects only declared files:

```json
{
  "schemaVersion": 1,
  "configurationId": "review-config-1",
  "assignmentKind": "child",
  "role": "review",
  "permissions": {
    "requested": {
      "tools": ["read-file"],
      "readPaths": [{"kind": "file", "path": "apps/desktop/src/sessionSort.ts"}],
      "writePaths": [],
      "commands": [],
      "externalSources": [],
      "connectors": [],
      "coordinationActions": [],
      "nativeDelegation": "denied",
      "accessChanges": "request-only"
    },
    "effective": null
  },
  "eligibility": "unverified"
}
```

`effective: null` is allowed only in a draft projection. The approved full record
requires a complete effective profile and verified binding; this example is
**denied** for approval/launch. A hypothetical verified read-file adapter can
supply a matching effective profile, skills, and complete digest-bound definition;
that configuration is accepted only after exact-plan approval. This is a contract
example, not evidence that such an adapter exists.

### Accepted contract path (synthetic fixture only)

This scenario pins the positive eligibility path in the contract; it does not
describe a real supported coding tool. Given a disposable fixture adapter for a
review assignment, accept it as **eligible for exact-plan approval** only when:

1. Its version, executable identity, platform, instruction mechanism, effective
   settings, role, and requested/effective profile digests exactly match the
   immutable configuration.
2. Every required evidence surface is `passed` or has a source-backed,
   fixture-confirmed `not-applicable` result, with retained referenced bytes
   matching their recorded digests.
3. The snapshot decision is `verified`, the snapshot and current support state
   validate, no revocation or invalidation is present, and all approved rule,
   skill, resource, model, and path bindings still match.
4. These checks make the assignment eligible for approval; they do not approve
   or launch it. The user must still approve the exact plan revision before a
   launch grant exists. A missing or unknown delivery receipt after launch
   blocks substantive work; approval does not waive the delivery gate.

The accepted result is limited to that synthetic fixture and exact profile.
The capability register currently marks all six Copilot roles unavailable and
does not establish a supported profile for another tool. This example tests the
contract's acceptance conditions only; it cannot populate the register or
authorize a production launch.

| Case | Required outcome |
| --- | --- |
| Root/scoped rules larger than former 16 KiB cap | Full applicable bytes admitted only within the new bounds and verified adapter context capacity; never truncated |
| Whole worktree scope | Explicit `worktree-root` descriptor resolves to that approved canonical root, with normal escape/adapter checks |
| Empty/optional-only acceptance criteria | Reject approval; at least one required criterion needed |
| Skill version `v6.3.0` | Accept as a label within 128 UTF-8 bytes; reject empty or oversized labels and non-string versions |
| Template/rubric/task numeric version | Reject non-integers and values outside 1 through 2^31−1 |
| Version-retirement discriminator/type mismatch | Reject a string role-template version, numeric skill version, empty/oversized skill label, or out-of-range template version |
| Role profile with unspecified task scope | Keep every permission denied; role ceiling alone grants no paths, commands, sources, or tools |
| Referenced resource missing, changed or placed in rules | Reject missing/invalid closure or drift; preserve skill ownership and include exact resource bytes in digest/delivery |
| Evidence digest valid but fixture missing, binding different or revoked | Reject approval/launch; a matching hash alone cannot establish verified eligibility |
| Same logical skill/version, different bytes | Different configuration; old approval cannot launch new bytes |
| Reordered object keys | Same canonical bytes/digest; duplicate keys rejected |
| Reordered skills in input | Validate canonical skill ordering before rendering; equivalent sorted definition produces same digest |
| Changed task criteria, model policy or permissions | New definition/digest and approval required |
| Advisory input conflicts with binding repository rule | Reject the conflicting assignment; cannot relabel binding authority as advisory |
| Mandatory manifest ID missing or labeled optional | Reject before approval; preserve source references |
| Changed pinned model or model policy/generation | Revalidate; changed approved identity/settings require new proposal |
| Tool-managed model changes within approved policy | Preserve known/reported/unknown observation; no change to immutable policy and no known-model cohort inferred |
| `planRevision`/`preparationRevision` boundaries | Accept 1 and 64; reject 0 and 65, matching #742's shared identity bound |
| Binding repository rules conflict at root and scoped path | Block the configuration with both source IDs; version 1 has no scoped override operation |
| Retired skill/template version selected for a new proposal | Reject selection with the retirement reason; preserve existing approved snapshot/history |
| Missing mandatory skill, conflicting repository rule, oversized context | Reject; no automatic skill removal or truncation |
| Selected optional skill supplied in startup context | Loaded receipt only after adapter confirms delivery; no native invocation claim |
| Hook records a terminal mention or unknown tool event | Unknown/reported usage; cannot create a native observed-use record |
| Unlisted ambient command environment or unbound startup/wrapper value | Strip unlisted variables; command-enabled profile ineligible unless exact effective environment is verified |
| Edit tool denied, unrestricted shell available | Ineligible for research/review; do not label read-only |
| Undeclared MCP/server, external redirect or native subagent | Deny; if controls cannot guarantee this, ineligible |
| Test output needed but write policy empty | Invalid verification configuration; declare generated paths/commands and obtain approval |
| Approved worktree subtree resolves outside via symlink | Reject canonical path binding; do not widen root |
| Executable/configuration/support evidence changes before launch | Block and propose reviewed revision; no substitution |
| Same config resumed after sidecar restart | Revalidate, rotate baseline capability, pause/reapprove; no automatic launch |
| Child requests additional write path or approves a tool-side broadening prompt | Block; user-owned revised plan required |
| Ordinary session has no config | Ordinary behavior; not a restricted agent and no configuration score invented |
| Retired snapshot or corrected history | Preserve historical identity; use only eligible versions in future proposals |

## Review handoff — 2026-10-08

The author-level correctness/completeness pass checked this contract against
#741, the product direction, accepted ADR 0077, the #610 scope alignment, and
the listed source seams at the current `origin/main` base above. The source
inspection confirms that the current coordinator, session creation, and
harness registry do not enforce role profiles or skill delivery; the draft
defines a proposed seam and does not claim that these runtime capabilities
exist.

The capability-independent audit against the #741 acceptance criteria and the
consuming #742/#743 contracts aligned `planRevision` to #742's `1..64` range,
removed the undefined version-1 scoped-rule override, and defined durable,
source-scoped retirement plus support-reason projection. These are contract
clarifications only; they add no capability evidence and do not change the #740
no-go disposition.

The merged #740 capability register records the current capability-status and
evidence disposition for this handoff: all six Copilot roles are unavailable,
and no other tool has a verified substitute profile. Eligibility still
requires an exact-profile immutable `CapabilityEvidenceSnapshot` plus current
support-state validation. Draft PR #765 contains additional Probe 0 transport
observations, but remains a draft and does not establish complete content
delivery, response exclusion, or permission enforcement. Do not promote those
observations into eligibility until their research handoff is reviewed and the
required snapshot and support checks pass.

Usage contract #743 merged in PR #785. Its report binding consumes
`ModelBinding.adapterGeneration` as the stable adapter/capability identity;
observer restarts instead receive a new `producerStreamId`. The two contracts
now state this distinction explicitly. This alignment adds no capability
evidence: the #740 register still marks all six Copilot roles unavailable and
does not verify a substitute tool profile.

No runtime execution plan is ready to write. The next gates are written owner
review of this proposed contract and the reviewed #740 evidence disposition. If
that evidence still yields no eligible profile, preserve the no-go and do not
create a runtime launch plan. This handoff is not approval of an API, runtime
implementation, or coding-tool profile.

## Consumer interfaces and implementation gate

Preparation #742 consumes role/assignment/configuration identities and
eligibility, and defines multi-plan parent authority. Usage #743 consumes
configuration and skill digests, the immutable `adapterGeneration`, and
adapter delivery/observation coverage; producer-process restarts are fenced by
`producerStreamId`. Evidence records are not part of the immutable
configuration. Evaluation #744 consumes approved
criteria/rubric and result identity, not just role labels. Learning #745 consumes
versioned configuration plus independently assessed outcomes. UI #746 consumes
bounded descriptors, permission reasons, and actual delivery/evaluation evidence;
it never grants permissions or decides adapter eligibility.

This draft supplies the contract for their written review. It does not mark
those consumers reviewed or implemented. #610's authoritative scope/ADR
alignment is accepted in PR #747. Before a runtime plan can be written,
review this contract and #740's
capability evidence, and establish a verified initial role/tool/profile slice.
The current no-go result is valid research, but cannot be converted into
permission-free fallback implementation. Keep #740/#741 open until their reviewed
handoff and prerequisite gates are satisfied.
