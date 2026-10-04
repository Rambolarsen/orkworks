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
that a coding tool is eligible. The authoritative Taskmaster/MVP specs and #610
still need alignment with the ordinary-child-session proposal. The capability
register currently supplies no verified launch slice. Review of this document,
the upstream launch scope, and a scoped execution plan is required before code.
No accepted ADR is amended or superseded by this draft.

### Current source and intended seam

| Source | Established behavior | Proposed extension |
| --- | --- | --- |
| `crates/orkworksd/src/taskmaster/coordinator.rs:1` | Data-only records; `PlanNode` contains role, context digest, criteria, scope, tools, provider choices | Bind a separate versioned configuration into the approved definition; do not reinterpret existing tool lists as enforcement |
| `crates/orkworksd/src/taskmaster/coordinator_store.rs:1` | Durable definitions and approval state, no runtime authority | Persist immutable configuration snapshots with definitions and mutable delivery evidence separately |
| `crates/orkworksd/src/session_application.rs:216` | `CreateSessionCommand` carries harness, model, initial prompt | A separately reviewed orchestration launch path consumes a validated configuration; ordinary creation remains unchanged |
| `crates/orkworksd/src/harness/definition.rs:12` | Harness definition declares launch/resume/integration capabilities | Versioned adapter eligibility evidence and deterministic role/permission mapping |
| `crates/orkworksd/src/harness/registry.rs:44` | Resolved definition and effective capabilities snapshot | Pin that definition plus executable/version/configuration evidence; registry membership alone is insufficient |

These are source investigation references at branch base `5779d237`, not new
production types. A later implementation plan must choose exact modules and
wire routes after contract review.

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
| `SkillSnapshot` | `id`, `version` (nonempty label), `content`, `contentDigest`, `requirement` (`mandatory` or `optional`), `selectionReason`, `requirementSources`, `provenance`; any context-relevant referenced resource is separately snapshotted |
| `RuleSnapshot` | `id`, `sourcePath`, `content`, `contentDigest`, `precedence`, `authority` (always `binding`), `provenance`; scoped applicable instruction files are explicit inputs |
| `RequirementManifest` | `sourceId`, `sourceDigest`, `taskId`, `requiredSkillIds`, `sourceLocations`, `applicabilityReason`; resolved requirement annotations separate from source snapshot content |
| `CriterionSnapshot` | `id`, `requirement` (`required` or `optional`), `description` |
| `RubricSnapshot` | `id`, `version`, `dimensions` (ID/description pairs), `evaluatorRole`; rating/calculation belongs to #744 |
| `AdapterBinding` | `harnessId`, `definitionDigest`, `adapterId`, `adapterVersion`, `executableIdentity`, `toolVersion`, `platform`, `instructionMechanism`, `effectiveSettingsDigest`, `evidenceId`, `evidenceDigest` |
| `ModelBinding` | `schemaVersion` (1), `mode` (`pinned` or `tool-managed`), `modelId` (required for pinned, null for tool-managed), `policyId` (adapter-recognized policy), `policyDigest`, `adapterGeneration`; part of the immutable configuration |
| `AssignmentConfiguration` | `schemaVersion`, `configurationId`, `repositoryId`, `workspaceId`, `parentSessionId`, `planId`, `planRevision`, `taskId`, `assignmentKind`, `roleTemplate`, `taskCategory`, `assignment`, `rules`, `requirementManifests`, `skills`, `rubric`, `harness`, `model`, `permissions`, `renderedInstructions`, `renderedInstructionsDigest`, `configurationDigest` |

`configurationId` is stable only within the immutable plan revision. A changed
configuration gets a new ID and digest. Repository/workspace/plan/task identities
are those supplied by the approved baseline, never guessed from a display name.
The repository identity across worktrees remains a reviewed #745 input; an
unresolved repository binding prevents comparison and launch.

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

`provenance` records source kind (`repository`, `bundled`, or `user-approved`),
source identity/revision and, for a local file, its canonical source path.
It contains no credential. Skill ID denotes a logical skill; version plus content
digest identifies the content actually approved. Same name/version with different
bytes is drift, not the same historical configuration.

`model` is a `ModelBinding`, never an implicit default. In `pinned` mode, the
adapter must establish the requested model ID; if it cannot, that combination
is unverified and cannot launch as pinned. In `tool-managed` mode, `policyId`
names an explicit approved adapter policy (for example that tool's `auto`), and
`policyDigest` binds its nonsecret settings and supported semantics. It does
not promise a fixed inference backend. `adapterGeneration` pins the resolved
model-capability snapshot used for the proposal.

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
3. Normalize only collections declared as sets (skills by ID; tool/action and
   permission path entries by their full validated identity) before rendering.
   Reject duplicate entries. Preserve meaningful instruction precedence,
   command argv, criteria and dependency ordering; do not silently reorder them.
   Snapshot `contentDigest` for skills/rules is the digest of exact content bytes;
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
`repositoryId`, `workspaceId`, `parentSessionId`, `runId`, `rootAssignmentId`,
`goalDigest`, `roleTemplate`, `taskCategory`, `assignment`, `rules`,
`requirementManifests`, `skills`, `rubric`, `harness`, `model`, `permissions`,
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
`preparationRevision`, and `inputDigest`. It is part of that plan's immutable
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
   instruction resources are included in the same snapshot manifest.

Every `RuleSnapshot` is binding; advisory research/context belongs in assignment
inputs, not a downgraded rule snapshot. `authority` must equal `binding`. Discover
applicable repository rules from the approved root router and scoped instruction
paths for the assignment, recording each resolved source and precedence. A
nested rule supplements inherited rules except where the source explicitly
specifies an authorized scoped override. Unknown applicability or an unresolved
contradiction blocks approval; a later section cannot silently erase a binding
root instruction.

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
| `commands` | Exact executable identity, argv, cwd, declared output/effects, and any permitted environment variable names; no wildcard shell string |
| `externalSources` | Explicit source/host and action policy for search/fetch; redirected destinations must satisfy the same policy |
| `connectors` | Exact server/connector identity and approved read/action scope; absent means denied, not inherit unrestricted MCP |
| `coordinationActions` | Only declared parent-plan operations; child reports have assignment-scoped identity, not launch rights |
| `nativeDelegation` | `denied` in version 1, including skill-triggered or tool-native subagent execution |
| `accessChanges` | `request-only`; no child modifies approval, permission settings, profile, tool/model choice, or its own effective instructions |

Provider inference traffic inherent in the approved coding tool is described
in adapter evidence, separate from agent-requested external search, shell
networking, and connector actions. This contract does not proxy credentials or
control the provider's internal infrastructure. Inherited ordinary-session
credentials remain governed by the baseline; they do not grant connector or
agent-requested network actions in the effective profile.

Repository path descriptors are root-relative, use `/`, and reject absolute,
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
reproducible bounded capability fixture for the relevant combination. `limited`,
`unverified`, and `unsupported` combinations cannot launch restricted roles.

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
| IDs/digests | 128-byte IDs; 64-character digests; schemaVersion exactly 1; revision/version integers 1 through 2^31−1 |
| Labels/reasons | 512 bytes per label; 2 KiB per reason/source reference |
| Skills/rules/inputs | 16 skills, 32 rule/resource snapshots, and 32 input references per configuration; 33 requirement manifests, at most 16 requiredSkillIds and 32 sourceLocations/requirementSources per record, each source location at most 512 bytes; unique IDs within each namespace |
| Criteria/dimensions | 32 acceptance criteria and 16 rubric dimensions; 2 KiB per description and output-contract text, 4 KiB assignment description |
| Paths/tools/actions | 64 read paths, 64 write paths, 64 tools, 32 commands, 32 source policies, 16 connectors, 16 coordination actions; path/reference 2 KiB, policy text 2 KiB; dependencies at most 128 unique task IDs |
| Commands | 64 argv elements, 2 KiB per element, 32 environment variable names; values excluded except bounded nonsecret approved literals |
| Supplied instructions | 16 KiB total rendered context, including all selected instruction content; individual skill/rule/template cannot exceed this total |
| Configuration | 64 KiB serialized descriptor plus its inline snapshots; rendered content counts within that bound |
| Plan | 128 tasks and 2 MiB total approved definition, inclusive of every configuration; lower existing/upstream limit always wins |
| Template catalog | 64 role-template versions per workspace, including pinned historical versions; 1 MiB total |
| Evidence references | 16 per configuration, each at most 2 KiB; no raw event transcripts |
| Blocker/delivery records | At most 16 KiB per record, 16 evidence references and 16 skill delivery entries; idempotency/rate/aggregate retention belong to #742/#743 |
| Admission | Reject oversized/unsupported input before writing or launching; never truncate mandatory content or evict referenced snapshots |

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

Editing a template/skill creates a new version/digest. Retired versions cannot
be selected for new proposals but remain available for historical interpretation.
A reviewed evidence invalidation can make an old version ineligible for resume.
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
      "readPaths": [{"kind": "file", "path": "apps/desktop/src/domain/session.ts"}],
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

| Case | Required outcome |
| --- | --- |
| Same logical skill/version, different bytes | Different configuration; old approval cannot launch new bytes |
| Reordered object keys | Same canonical bytes/digest; duplicate keys rejected |
| Reordered skills in input | Validate canonical skill ordering before rendering; equivalent sorted definition produces same digest |
| Changed task criteria, model policy or permissions | New definition/digest and approval required |
| Advisory input conflicts with binding repository rule | Reject the conflicting assignment; cannot relabel binding authority as advisory |
| Mandatory manifest ID missing or labeled optional | Reject before approval; preserve source references |
| Changed pinned model or model policy/generation | Revalidate; changed approved identity/settings require new proposal |
| Tool-managed model changes within approved policy | Preserve known/reported/unknown observation; no change to immutable policy and no known-model cohort inferred |
| Missing mandatory skill, conflicting repository rule, oversized context | Reject; no automatic skill removal or truncation |
| Selected optional skill supplied in startup context | Loaded receipt only after adapter confirms delivery; no native invocation claim |
| Hook records a terminal mention or unknown tool event | Unknown/reported usage; cannot create a native observed-use record |
| Edit tool denied, unrestricted shell available | Ineligible for research/review; do not label read-only |
| Undeclared MCP/server, external redirect or native subagent | Deny; if controls cannot guarantee this, ineligible |
| Test output needed but write policy empty | Invalid verification configuration; declare generated paths/commands and obtain approval |
| Approved worktree subtree resolves outside via symlink | Reject canonical path binding; do not widen root |
| Executable/configuration/support evidence changes before launch | Block and propose reviewed revision; no substitution |
| Same config resumed after sidecar restart | Revalidate, rotate baseline capability, pause/reapprove; no automatic launch |
| Child requests additional write path or approves a tool-side broadening prompt | Block; user-owned revised plan required |
| Ordinary session has no config | Ordinary behavior; not a restricted agent and no configuration score invented |
| Retired snapshot or corrected history | Preserve historical identity; use only eligible versions in future proposals |

## Consumer interfaces and implementation gate

Preparation #742 consumes role/assignment/configuration identities and eligibility,
and defines multi-plan parent authority. Usage #743 consumes configuration and
skill digests plus adapter delivery/observation coverage; evidence records are
not part of the immutable configuration. Evaluation #744 consumes approved
criteria/rubric and result identity, not just role labels. Learning #745 consumes
versioned configuration plus independently assessed outcomes. UI #746 consumes
bounded descriptors, permission reasons, and actual delivery/evaluation evidence;
it never grants permissions or decides adapter eligibility.

This draft supplies the contract for their written review. It does not mark
those consumers reviewed or implemented. Before a runtime plan can be written:
reconcile #610 and authoritative scope/ADRs, review this contract and #740's
capability evidence, and establish a verified initial role/tool/profile slice.
The current no-go result is valid research, but cannot be converted into
permission-free fallback implementation. Keep #740/#741 open until their reviewed
handoff and prerequisite gates are satisfied.
