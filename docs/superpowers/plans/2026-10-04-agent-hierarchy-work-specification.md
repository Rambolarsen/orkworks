# Agent Hierarchy Specification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to execute the specification tasks with review checkpoints. Steps use checkbox (`- [ ]`) syntax for tracking. Each task requires an owned worktree; this plan does not start agents or authorize runtime implementation.

**Goal:** Produce reviewed, scoped work specifications for the agent hierarchy, role configuration, preparation, evidence, evaluation, and learning.

**Architecture:** Extend the existing proposed ordinary-child-session workflow and preserve its approval/runtime boundaries. Separate configuration, reporting, evaluation/history, and renderer presentation into focused modules with reviewed interfaces before creating code execution plans. Reuse the existing data-only coordinator foundation and session runtime only after authoritative scope alignment.

**Tech Stack:** Markdown specifications, GitHub issues, VitePress documentation, and source investigation of the existing Rust/Axum sidecar and Electron/React/TypeScript app.

**Spec:** [Agent hierarchy and configuration learning](../specs/2026-10-04-agent-hierarchy-and-configuration-learning-design.md); [ordinary child-session launch baseline](../specs/2026-09-26-taskmaster-orchestrated-child-sessions-design.md).

**Tracking:** [#738](https://github.com/Rambolarsen/orkworks/issues/738); continuing [draft PR #739](https://github.com/Rambolarsen/orkworks/pull/739).

## First specification batch: 2026-10-04

Task 1 has a [capability evidence register](../../validation/agent-role-capabilities.md)
and Task 2 has a [role configuration draft](../specs/2026-10-04-agent-role-configuration-design.md).
They are proposed review artifacts, not completed reviewed handoffs. Copilot CLI
1.0.90 exposes candidate role/tool controls, but exact delivery and enforcement
profiles remain unverified; no supported launch slice is claimed. The installed
loader package version also differs from the executable's reported version, so
launch identity must pin the actual resolved executable chain.

- [x] Investigate sources and produce the first configuration/evidence drafts.
- [ ] Obtain review of the concrete role contract and capability methodology.
- [ ] In a separately authorized probe session, establish version-specific
  delivery/permission fixtures or retain an explicit no-go outcome.
- [ ] Finalize downstream contract dependencies and reviewed execution handoffs.

Task 3 now has a [preparation lifecycle draft](../specs/2026-10-04-orchestrator-preparation-design.md): the user chose a continuously running parent with automatic execution-plan preparation. Run planning authority, exact-plan grants, event continuation and run-wide capacity are proposed contracts awaiting review. Tasks 4–7 remain queued. Authoritative scope alignment is accepted in PR #747. Code execution planning remains behind written contract review and capability evidence gates.

## Work-picking priority

The user made this initiative and directly related bugs the first priority when
picking work. Follow the root `AGENTS.md` priority and its
[development workflow reference](../../agents/development-workflow.md#issue-prioritization-and-scope).
Within this focus, address related bugs/stabilization and ready prerequisites
before dependent feature work. Link related issues to #738 and name the affected
contract/component. Existing spec, capability, approval and dependency gates
still apply; priority is not runtime authorization.

## Global Constraints

- Product scope is approved for work specification; detailed contracts and execution plans require their own review before code starts.
- Parent-only delegation, ordered batches, one attempt per task, and the approved live-child concurrency limit remain in force.
- Research children and plan-owned worktrees require exact-plan approval; research completion never authorizes execution.
- Independent task chains use separate worktrees. Dependent reuse requires a terminal predecessor and user quiescence acknowledgement.
- Ordinary sessions and the single selected terminal remain the runtime/UX foundation.
- Use coding-tool-enforced permission profiles. Unsupported or unverified profiles block launch; no OS confinement is claimed.
- Mandatory repository instructions/skills stay binding. Skills, scores, and reports cannot expand authority.
- User acceptance, merges, and manual code integration remain separate from evaluation and coordination reports.
- Skill edits are reviewable proposals. Learning affects later approved configurations, never active sessions.
- Keep history local to the repository for the first version; preserve version/provenance and bounded records.
- Use pnpm for Node package tasks and RTK for shell commands. Respect both scoped AGENTS files when planning cross-component work.
- Only this documentation/backlog work is in execution now. The current source has no implemented orchestrated-child launch or skill usage feature.

## Scope and plan granularity

This is the specification-stage plan. Its independently reviewable outputs are
component contracts, a capability evidence register, interaction mockups, and
subsequent scoped execution plans/issues. It does not invent a runnable Rust
or TypeScript interface before the contracts and tool capabilities are verified.

The broad product design spans several subsystems. Split the subsequent code
plans by the deliverables below rather than extending the old runner plan into
one large PR. Each code plan must name exact files and interfaces, carry
meaningful failing/passing verification steps, and preserve the relevant
specification and approval gates.

## Investigated uncertainty checkpoint

**What am I least confident about?** Which coding tools can enforce the requested
role permissions, prove skill delivery, and expose recognizable usage. The
current `HarnessDefinition` declares launch/model/resume/integration capabilities
but no role-profile or skill-delivery contract. `CreateSessionCommand` currently
carries only harness, model, and initial prompt. The integration register does
not define portable skill-use events. Package #740 investigates exact supported versions;
unsupported outcomes remain valid findings and must block eligibility.

**What is the biggest missing dependency?** The coordinator foundation is
explicitly data-only: `taskmaster/coordinator.rs` and `coordinator_store.rs`
state that runtime authority is absent. #604/#606 are closed foundation work;
#610 now tracks the accepted ordinary-child scope alignment in PR #747.
Detailed component contracts, exact capability evidence and scoped runtime
implementation plans remain prerequisites; launch capability is not implemented.

A further lifecycle gap is concrete: the launch proposal revokes the plan
capability on completion, while the new preparation flow can finish research
before its execution assignments are known. Package #742 must define same-parent
multi-plan authority before that transition is implemented. The user chose
to retain the same live parent: the draft separates a run planning bearer from
server-held exact-plan grants and requires verified event continuation. No
expired plan bearer is reused; prior-plan children still count against the run cap.

The existing baseline plan's stale ADR 0066 reference has been replaced by
accepted [ADR 0077](../../adr/0077-taskmaster-orchestrated-child-sessions.md).
The [#610 alignment draft](../specs/2026-10-04-taskmaster-orchestration-scope-design.md)
reconciles ordinary-child scope and run/grant authority; written scope acceptance
is recorded on PR #747, #610 criteria are aligned and #617 is retired. ADR 0066 stays hook-owned attention.

Resolved scope: coding-tool controls are sufficient for this product slice;
recursive delegation, native confinement, automatic retries/integration, and
activity-based rewards remain excluded. Unresolved tool/version support,
reporting/storage bounds, and the preparation capability transition are
reviewed specification deliverables.

## Repository identity draft: 2026-10-04

The early #745 input has a [proposed repository-identity contract](../specs/2026-10-04-configuration-learning-design.md)
and [documentation task plan](2026-10-04-repository-identity-specification.md).
It specifies local common-directory registration, replacement/relocation
behavior, exact configuration/approval/resume binding and finite persistence.
Written contract review, filesystem/platform evidence and executable code
planning remain open. It does not complete #745's downstream learning/history
scope or satisfy baseline unit 4's reviewed identity prerequisite yet.

## Application-shell prerequisite: #755

The [proposed shell/navigation contract](../specs/2026-10-05-application-shell-navigation-design.md)
and six synthetic mockups are drafted for written review. #755 precedes final
shell-dependent #746 placement/navigation acceptance. It compares central tree
and timeline views, removes panel dragging/tab management, defines compact
session switching and central Review/Terminal returns, and stages the required
replacement ADR for ADR 0002/0011/0013, the ADR 0060 context-clause
replacement pointer, ADR 0034 placement amendment, authoritative MVP/Taskmaster/
Session Plan Review/multi-workspace reconciliation, and layout migration.
No shell direction, replacement library or runtime execution plan is accepted
by this link. Existing #746 evidence/identity contracts remain separately gated.

## Work packages and dependency order

| Package | Tracker | Drafting inputs / final dependencies | Deliverable |
| --- | --- | --- | --- |
| Orchestration scope alignment | [#610](https://github.com/Rambolarsen/orkworks/issues/610), existing baseline-plan Task 1 | Product design and existing authoritative specs/ADRs | Consistent launch scope and reviewed baseline execution plan |
| Research: verify coding-tool role profiles and skill delivery | [#740](https://github.com/Rambolarsen/orkworks/issues/740) | Draft independently; finalize eligibility with sibling research/configuration contract | `docs/validation/agent-role-capabilities.md` |
| Spec: agent role configuration and coding-tool permissions | [#741](https://github.com/Rambolarsen/orkworks/issues/741) | Draft independently; finalize eligibility with sibling research/configuration contract | `docs/superpowers/specs/2026-10-04-agent-role-configuration-design.md` |
| Spec: clarification and research-to-execution approval lifecycle | [#742](https://github.com/Rambolarsen/orkworks/issues/742) | #741, #740 | `docs/superpowers/specs/2026-10-04-orchestrator-preparation-design.md` |
| Spec: skill delivery and usage evidence protocol | [#743](https://github.com/Rambolarsen/orkworks/issues/743) | #741, #740 | `docs/superpowers/specs/2026-10-04-skill-usage-evidence-design.md` |
| Spec: assignment quality and completeness evaluation | [#744](https://github.com/Rambolarsen/orkworks/issues/744) | #741 | `docs/superpowers/specs/2026-10-04-assignment-evaluation-design.md` |
| Spec: repository-scoped configuration learning and skill improvements | [#745](https://github.com/Rambolarsen/orkworks/issues/745) | #741, #743, #744 | `docs/superpowers/specs/2026-10-04-configuration-learning-design.md` |
| Spec: application shell and workflow navigation | [#755](https://github.com/Rambolarsen/orkworks/issues/755) | Draft from current shell and #746 interaction proposal; final shell direction and authoritative reconciliation require written acceptance | `docs/superpowers/specs/2026-10-05-application-shell-navigation-design.md` and six synthetic shell mockups |
| Spec: agent hierarchy interaction and visual presentation | [#746](https://github.com/Rambolarsen/orkworks/issues/746) | Draft independently; finalize projection after #741, #743, #744; final placement/navigation also requires accepted #755 shell direction and authoritative reconciliation | `docs/superpowers/specs/2026-10-04-agent-hierarchy-ui-design.md` |

#745's repository-identity-across-worktrees portion is an early reviewed
input to #741 and baseline worktree/child admission; draft it with #741 before
baseline unit 4. It does not depend on usage/evaluation history. The remaining
learning/cohort/history contract still consumes #743/#744 downstream. Unresolved
repository identity blocks launch; it cannot wait for the complete learning unit.

Parallel drafting lanes:

1. Scope alignment, capability research, role-contract drafting, and hierarchy
   interaction/mockup drafting and #755 shell drafting can proceed independently
   from the agreed product direction; their final navigation must agree.
2. After configuration/eligibility decisions, preparation, usage evidence, and
   evaluation contracts can proceed independently with shared identity terms.
3. Learning consumes the approved usage/evaluation contracts. The hierarchy
   projection is finalized against those contracts; final #746 placement/navigation
   also waits for #755 shell acceptance and authoritative reconciliation.
4. Only then create/review scoped runtime plans and implementation issues.
   Foundation approval and tool eligibility remain launch gates.

Do not treat all of #610's runtime as a prerequisite to writing contracts.
Reuse its existing scope-alignment task instead of creating a competing runner
issue. Conversely, a completed specification issue does not close #610 or
authorize its runtime.

## Existing scope-alignment task: #610

**Files to reconcile:** `specs/taskmaster.md`, `specs/orkworks-mvp.md`,
`specs/multi-workspace.md`, `AGENTS.md`, `README.md`,
`docs/agents/product-boundaries.md`, `docs/agents/architecture.md`,
`docs/adr/0060-independent-workspace-instances.md`,
`docs/adr/0064-bounded-taskmaster-coordinator.md`, `docs/adr/README.md`,
and the 2026-09-25/26 runner/launch proposals and baseline implementation plan.
Preserve the historical confinement validation record.

**Consumes:** The current newer launch proposal, this product extension,
#610's old acceptance criteria, #617's native-boundary scope, and accepted
session/workspace decisions.

**Produces:** An accepted product/ADR boundary for ordinary
orchestrated children, non-colliding ADR 0077, and a baseline handoff split into
runtime planning units. The [alignment draft](../specs/2026-10-04-taskmaster-orchestration-scope-design.md)
was accepted on 2026-10-04 for PR #747 after documentation validation.
Live issue disposition is aligned; capability evidence and reviewed executable
plans/implementation issues remain separate gates.

- [x] Compare the launch proposal with every conflicting root/spec/ADR statement;
  classify amendments versus supersessions under the ADR change sequence.
- [x] Write the actual scoped documentation changes and update the baseline
  plan's stale ADR reference using the next unoccupied index entry.
- [x] Update #610's criteria after the scope decision is accepted. Resolve
  #617's status explicitly against the ordinary-session design; do not silently
  retain native confinement as this slice's launch prerequisite.
- [x] Run documentation drift/link/build checks and obtain written review.
- [x] Break the baseline runtime into independently testable PR-sized execution
  work before coding; keep this extension's modules out of unrelated runtime PRs.

## Task 1: Research: verify coding-tool role profiles and skill delivery

**Tracker:** [#740](https://github.com/Rambolarsen/orkworks/issues/740)

**Files and ownership:**
- Create: `docs/validation/agent-role-capabilities.md`.
- Own its reviewed specification/research handoff and subsequent scoped execution
  plan. Other tasks own their own artifacts; do not overwrite their edits or
  independently change shared configuration/projection terms.
- Read: `docs/agents/harness-integration-contracts.md`.
- Read: `crates/orkworksd/src/harness/definition.rs`.
- Read: `crates/orkworksd/src/harness/registry.rs`.
- Read: `crates/orkworksd/src/harness/integrations/copilot.rs`.
- Read: `crates/orkworksd/src/harness/integrations/codex.rs`.
- Read: `crates/orkworksd/src/harness/integrations/claude.rs`.
- Read: `crates/orkworksd/src/harness/integrations/opencode.rs`.

**Interfaces:**
- Consumes: Existing coding-tool definitions/integration contracts and the product's role/permission requirements.
- Produces: The evidence register `docs/validation/agent-role-capabilities.md`: one row per exact tool/version × role/profile with prompt delivery mechanism, skill delivery confirmation, permission enforcement evidence, invocation coverage, support status, and limitations.

**Steps:**

- [ ] **Step 1: Establish context and prerequisites.** Read both linked product
  specifications, the tracker and its dependency issues, applicable scoped
  instructions, and the listed source files. Record facts, proposed contract
  choices, and remaining eligibility questions in the artifact.
- [ ] **Step 2: Write the concrete artifact.** Use these sections and fill each
  with explicit decisions, identities, examples, limits, and failure behavior:

```markdown
# verify coding-tool role profiles and skill delivery
## Scope and status
## Evidence methodology and exact versions
## Role/prompt delivery
## Skill delivery confirmation
## Permission enforcement and bypass checks
## Skill invocation coverage
## Eligibility matrix and negative findings
## Verified initial slice and handoff
## Verification cases
## Execution plan and implementation gate
```

- [ ] **Step 3: Check the task's full acceptance criteria.**

- [ ] Record exact tool versions, primary documentation, reproducible bounded probes/fixtures, and separate verified/limited/unsupported results; authentication, installation, or a prompt instruction alone is not permission enforcement.
- [ ] Verify native system/agent instructions versus startup-context delivery, repository instruction inheritance, required-skill inclusion, and confirmation that each selected skill's content was delivered.
- [ ] For each role, check tools, command execution, filesystem scope, temporary test output, network access, MCP/connectors, and ways broad inherited access could bypass the intended profile.
- [ ] Define eligibility when a role or requested scope is unsupported; never silently widen access, change provider, or describe prompt-only restrictions as enforced.
- [ ] Specify which recognized skill invocations can be observed, which usage is agent-reported only, and which is unknown; preserve source and coverage.
- [ ] Record a supported initial slice or an explicit no-go finding without expanding scope. Use bounded disposable probes; do not start live implementation agents.
- [ ] Deliver the evidence register and a reviewed handoff to the configuration, preparation, and telemetry specifications.

- [ ] **Step 4: Review through the consuming interfaces.** Check the artifact
  against the source design and dependency contracts; include concrete
  accepted/denied, missing-evidence, stale/replayed, and relevant recovery cases.
  Obtain written review of consequential decisions. Evidence gaps cannot become
  an assumed supported runtime capability.
- [ ] **Step 5: Validate and commit the specification.** Run the shared
  documentation commands below from the owned checkout, inspect their output,
  and commit the artifact on its branch:

```bash
rtk git add docs/validation/agent-role-capabilities.md
rtk git diff --cached --check
rtk proxy bash scripts/doc-check.sh
rtk proxy pnpm --dir docs docs:build
rtk git commit -m "docs: specify coding-tool role capability evidence"
```

- [ ] **Step 6: Prepare the scoped execution handoff.** After its reviewed contract
  and upstream gates are satisfied, write a separate execution plan with actual
  types/interfaces, source/test files, and meaningful verification commands.
  Create implementation issues only for authoritative spec-covered work and
  link them to #740. Do not start code as part of this specification task.

## Task 2: Spec: agent role configuration and coding-tool permissions

**Tracker:** [#741](https://github.com/Rambolarsen/orkworks/issues/741)

**Files and ownership:**
- Create: `docs/superpowers/specs/2026-10-04-agent-role-configuration-design.md`.
- Own its reviewed specification/research handoff and subsequent scoped execution
  plan. Other tasks own their own artifacts; do not overwrite their edits or
  independently change shared configuration/projection terms.
- Read: `crates/orkworksd/src/taskmaster/coordinator.rs`.
- Read: `crates/orkworksd/src/taskmaster/coordinator_store.rs`.
- Read: `crates/orkworksd/src/session_application.rs`.
- Read: `crates/orkworksd/src/harness/definition.rs`.
- Read: `crates/orkworksd/src/harness/registry.rs`.

**Interfaces:**
- Consumes: The product configuration requirements, existing immutable coordinator records, and the capability register's verified/unsupported outcomes.
- Produces: A reviewed configuration contract defining identity/version/digest, role-template snapshot, task category and scope, mandatory/optional skill snapshots, harness/model binding, requested/effective permission settings, and approval/launch validation. These named concepts are the shared inputs to preparation, usage, evaluation, learning, and presentation.

**Steps:**

- [ ] **Step 1: Establish context and prerequisites.** Read both linked product
  specifications, the tracker and its dependency issues, applicable scoped
  instructions, and the listed source files. Record facts, proposed contract
  choices, and remaining eligibility questions in the artifact.
- [ ] **Step 2: Write the concrete artifact.** Use these sections and fill each
  with explicit decisions, identities, examples, limits, and failure behavior:

```markdown
# agent role configuration and coding-tool permissions
## Scope and status
## Configuration identity and canonical digest
## Repository/role/assignment composition
## Role templates and mandatory skills
## Skill identity and delivery requirements
## Requested/effective permission profiles
## Adapter eligibility and drift
## Approval binding and access escalation
## Bounds, compatibility, and examples
## Verification cases
## Execution plan and implementation gate
```

- [ ] **Step 3: Check the task's full acceptance criteria.**

- [ ] Define versioned logical/wire records for role templates, assignment/configuration identity, skill content/version snapshots, requested/effective permissions, coding tool/model binding, and canonical configuration digest.
- [ ] Specify deterministic composition of shared repository rules, role instructions, task inputs, selected skills, and rubric; preserve mandatory rules and document conflicts that block launch.
- [ ] Define orchestrator, research, implementation, review, verification, and remediation role defaults. Verification includes explicitly declared generated/test output; unrestricted shell access does not qualify as read-only.
- [ ] Specify approval and launch revalidation of effective instructions, content identities, paths/actions, tool/model generation, and effective permission settings; drift produces a new proposal.
- [ ] Define adapter eligibility/support reasons, denial behavior, and access-escalation requests; no skill grants tools and no child widens its own profile.
- [ ] Define finite size/collection limits, provenance, template/skill retirement and updates, and backward-compatible handling of ordinary sessions.
- [ ] Include accepted/denied examples and contract-level verification cases. Use the capability register to finalize initial supported roles/tools before writing a runtime execution plan.

- [ ] **Step 4: Review through the consuming interfaces.** Check the artifact
  against the source design and dependency contracts; include concrete
  accepted/denied, missing-evidence, stale/replayed, and relevant recovery cases.
  Obtain written review of consequential decisions. Evidence gaps cannot become
  an assumed supported runtime capability.
- [ ] **Step 5: Validate and commit the specification.** Run the shared
  documentation commands below from the owned checkout, inspect their output,
  and commit the artifact on its branch:

```bash
rtk git add docs/superpowers/specs/2026-10-04-agent-role-configuration-design.md
rtk git diff --cached --check
rtk proxy bash scripts/doc-check.sh
rtk proxy pnpm --dir docs docs:build
rtk git commit -m "docs: specify agent role configuration and coding-tool permissions"
```

- [ ] **Step 6: Prepare the scoped execution handoff.** After its reviewed contract
  and upstream gates are satisfied, write a separate execution plan with actual
  types/interfaces, source/test files, and meaningful verification commands.
  Create implementation issues only for authoritative spec-covered work and
  link them to #741. Do not start code as part of this specification task.

## Task 3: Spec: clarification and research-to-execution approval lifecycle

**Tracker:** [#742](https://github.com/Rambolarsen/orkworks/issues/742)

**Files and ownership:**
- Create: `docs/superpowers/specs/2026-10-04-orchestrator-preparation-design.md`.
- Own its reviewed specification/research handoff and subsequent scoped execution
  plan. Other tasks own their own artifacts; do not overwrite their edits or
  independently change shared configuration/projection terms.
- Read: `docs/superpowers/specs/2026-09-26-taskmaster-orchestrated-child-sessions-design.md`.
- Read: `docs/superpowers/plans/2026-09-26-taskmaster-orchestrated-child-sessions.md`.
- Read: `crates/orkworksd/src/taskmaster/coordinator.rs`.
- Read: `crates/orkworksd/src/taskmaster/coordinator_store.rs`.
- Read: `crates/orkworksd/src/session_application.rs`.

**Interfaces:**
- Consumes: The role/configuration contract and capability eligibility matrix; the baseline's ordered batches, exact-plan approval, revocation, capacity, and worktree rules.
- Produces: A reviewed state/transition contract for clarification, research, result synthesis, proposal of execution, and same-parent authority renewal. It defines how plan/assignment identities and research result references cross stage boundaries without implicitly launching work.

**Steps:**

- [ ] **Step 1: Establish context and prerequisites.** Read both linked product
  specifications, the tracker and its dependency issues, applicable scoped
  instructions, and the listed source files. Record facts, proposed contract
  choices, and remaining eligibility questions in the artifact.
- [ ] **Step 2: Write the concrete artifact.** Use these sections and fill each
  with explicit decisions, identities, examples, limits, and failure behavior:

```markdown
# clarification and research-to-execution approval lifecycle
## Scope and status
## Assessment and skip decisions
## Material clarification lifecycle
## Research question/task contracts
## Research approval and result collection
## Same-parent research-to-execution transition
## Planning authority and capability renewal
## Revision, cancellation, and restart transitions
## Bounded reports and example workflows
## Verification cases
## Execution plan and implementation gate
```

- [ ] **Step 3: Check the task's full acceptance criteria.**

- [ ] Define assessment inputs and skip criteria: assessment remains coordination; codebase/external investigation is delegated; missing material requirements trigger clarification.
- [ ] Define bounded research question/task contracts, independent question decomposition, output/evidence references, and allocation within the existing approved concurrency limit. Bind orchestrating-task-graphs as required planning guidance for parallel plans; define result dependencies, one artifact owner, explicit ordered batches/cap, and independent correctness/completeness review questions without changing runtime authority or manual integration.
- [ ] Specify exact-plan approval before research launches, subsequent execution approval when assignments were not already fixed, and binding of reviewed research outputs into the next proposal.
- [ ] Define same-parent multi-plan identity with a continuously running orchestrator, automatic report collection/synthesis/execution proposal, verified event continuation, run-bound planning authority, separate server-held exact-plan grants, generation fencing and final revocation. Count prior-plan live children/reservations against the run ceiling; never reuse an expired plan bearer or infer execution approval from research completion.
- [ ] Preserve ordered batches, parent-only delegation, one attempt per task, quiescent worktree reuse, manual integration, capacity accounting, and no implied launch authority from research completion.
- [ ] Define cancellation, missing/conflicting research, late reports, restart/resume, rejected execution proposals, and changed scope without automatic relaunch or retries.
- [ ] Provide an explicit lifecycle/transition table and examples for a small direct task, research followed by revised execution, and a material clarification blocker. Show a short research summary alongside execution approval with sources/evidence in details.

- [ ] **Step 4: Review through the consuming interfaces.** Check the artifact
  against the source design and dependency contracts; include concrete
  accepted/denied, missing-evidence, stale/replayed, and relevant recovery cases.
  Obtain written review of consequential decisions. Evidence gaps cannot become
  an assumed supported runtime capability.
- [ ] **Step 5: Validate and commit the specification.** Run the shared
  documentation commands below from the owned checkout, inspect their output,
  and commit the artifact on its branch:

```bash
rtk git add docs/superpowers/specs/2026-10-04-orchestrator-preparation-design.md
rtk git diff --cached --check
rtk proxy bash scripts/doc-check.sh
rtk proxy pnpm --dir docs docs:build
rtk git commit -m "docs: specify clarification and research-to-execution approval lifecycle"
```

- [ ] **Step 6: Prepare the scoped execution handoff.** After its reviewed contract
  and upstream gates are satisfied, write a separate execution plan with actual
  types/interfaces, source/test files, and meaningful verification commands.
  Create implementation issues only for authoritative spec-covered work and
  link them to #742. Do not start code as part of this specification task.

## Task 4: Spec: skill delivery and usage evidence protocol

**Tracker:** [#743](https://github.com/Rambolarsen/orkworks/issues/743)

**Files and ownership:**
- Create: `docs/superpowers/specs/2026-10-04-skill-usage-evidence-design.md`.
- Own its reviewed specification/research handoff and subsequent scoped execution
  plan. Other tasks own their own artifacts; do not overwrite their edits or
  independently change shared configuration/projection terms.
- Read: `crates/orkworksd/src/http/workflow_observation_handlers.rs`.
- Read: `crates/orkworksd/src/workflow_observations.rs`.
- Read: `crates/orkworksd/src/runtime/terminal_runtime.rs`.
- Read: `crates/orkworksd/src/harness/integrations/mod.rs`.
- Read: `crates/orkworksd/scripts/report-harness-event.sh`.
- Read: `crates/orkworksd/scripts/report-harness-event.ps1`.

**Interfaces:**
- Consumes: Assignment/configuration/skill-version identities from the role contract and source/coverage guarantees from the capability register.
- Produces: A reviewed evidence contract for selected, loaded, reported-used, observed-used, and unknown states, including producer identity, coverage, event IDs, strict bounded reporting, correction/deletion, and projections. It supplies usage summaries to learning and badges to presentation.

**Steps:**

- [ ] **Step 1: Establish context and prerequisites.** Read both linked product
  specifications, the tracker and its dependency issues, applicable scoped
  instructions, and the listed source files. Record facts, proposed contract
  choices, and remaining eligibility questions in the artifact.
- [ ] **Step 2: Write the concrete artifact.** Use these sections and fill each
  with explicit decisions, identities, examples, limits, and failure behavior:

```markdown
# skill delivery and usage evidence protocol
## Scope and status
## Record identity and producers
## Selected versus confirmed-loaded delivery
## Reported/observed/unknown usage and coverage
## Authenticated requests and task-version validation
## Idempotency, conflicts, and replay
## Numerical request/rate/storage bounds
## Retention, deletion, and recovery
## Wire examples and renderer projection
## Verification cases
## Execution plan and implementation gate
```

- [ ] **Step 3: Check the task's full acceptance criteria.**

- [ ] Define selected, loaded, reported-used, observed-used, and unknown with explicit delivery/coverage semantics; source claims cannot promote reported or inferred usage into a native observation.
- [ ] Bind records to workspace/repository, plan revision, task/assignment, launched child, configuration digest, and skill version; derive authorized reporter identity from the session capability.
- [ ] Specify strict authenticated HTTP/report contracts, generation and task-version validation, stable event IDs, idempotent replay/conflict behavior, and permissions for each producer.
- [ ] Define numeric request/field/collection limits, pre-persistence rate limits, bounded aggregate history, tombstone/replay behavior, retention/deletion, and startup recovery.
- [ ] Use only verified adapters from the capability register. Terminal mentions are not invocation events; lack of observation stays unknown and does not become zero usage.
- [ ] Specify bounded evidence references/redaction and forbid storing secret tokens, hidden reasoning, full prompts, or complete transcripts as telemetry.
- [ ] Include JSON examples, status/error cases, retry/deletion/recovery cases, and renderer projections that distinguish selected from confirmed-loaded badges.

- [ ] **Step 4: Review through the consuming interfaces.** Check the artifact
  against the source design and dependency contracts; include concrete
  accepted/denied, missing-evidence, stale/replayed, and relevant recovery cases.
  Obtain written review of consequential decisions. Evidence gaps cannot become
  an assumed supported runtime capability.
- [ ] **Step 5: Validate and commit the specification.** Run the shared
  documentation commands below from the owned checkout, inspect their output,
  and commit the artifact on its branch:

```bash
rtk git add docs/superpowers/specs/2026-10-04-skill-usage-evidence-design.md
rtk git diff --cached --check
rtk proxy bash scripts/doc-check.sh
rtk proxy pnpm --dir docs docs:build
rtk git commit -m "docs: specify skill delivery and usage evidence protocol"
```

- [ ] **Step 6: Prepare the scoped execution handoff.** After its reviewed contract
  and upstream gates are satisfied, write a separate execution plan with actual
  types/interfaces, source/test files, and meaningful verification commands.
  Create implementation issues only for authoritative spec-covered work and
  link them to #743. Do not start code as part of this specification task.

## Task 5: Spec: assignment quality and completeness evaluation

**Tracker:** [#744](https://github.com/Rambolarsen/orkworks/issues/744)

**Files and ownership:**
- Create: `docs/superpowers/specs/2026-10-04-assignment-evaluation-design.md`.
- Own its reviewed specification/research handoff and subsequent scoped execution
  plan. Other tasks own their own artifacts; do not overwrite their edits or
  independently change shared configuration/projection terms.
- Read: `crates/orkworksd/src/taskmaster/coordinator.rs`.
- Read: `crates/orkworksd/src/taskmaster/completion.rs`.
- Read: `crates/orkworksd/src/taskmaster/completion_tests.rs`.
- Read: `crates/orkworksd/src/http/taskmaster_handlers.rs`.
- Read: `crates/orkworksd/src/session_application.rs`.

**Interfaces:**
- Consumes: Immutable assignment/configuration identity, approved acceptance criteria and rubric, reviewer identity, and the inspected result subject.
- Produces: A reviewed evaluation contract containing criterion outcomes, completeness score or unassessed state, quality rating or unassessed state, evidence, blockers, reviewer/source, freshness binding, and correction/invalidation history. It supplies outcome summaries to learning and presentation.

**Steps:**

- [ ] **Step 1: Establish context and prerequisites.** Read both linked product
  specifications, the tracker and its dependency issues, applicable scoped
  instructions, and the listed source files. Record facts, proposed contract
  choices, and remaining eligibility questions in the artifact.
- [ ] **Step 2: Write the concrete artifact.** Use these sections and fill each
  with explicit decisions, identities, examples, limits, and failure behavior:

```markdown
# assignment quality and completeness evaluation
## Scope and status
## Approved criteria and role rubrics
## Completeness calculation and unassessed state
## Quality rating and evidence requirements
## Reviewer identity and self-assessment separation
## Result revision binding and freshness
## Conflicts, corrections, and invalidation
## Numerical bounds and record lifecycle
## Coordination and user-acceptance separation
## Verification cases
## Execution plan and implementation gate
```

- [ ] **Step 3: Check the task's full acceptance criteria.**

- [ ] Define approved acceptance criteria and a versioned role-specific quality rubric, criteria identities, evaluation record, reviewer identity, artifact/result subject, and configuration binding.
- [ ] Define one overall result (Meets requirements, Needs rework, Unassessed), with current evidence and missing/stale/conflicting states; completeness and quality stay in details. Resolve score details: completeness satisfied/required percentage only when all required criteria are assessed; quality 0 unusable, 1 major rework, 2 limited rework, 3 meets standard; otherwise unassessed.
- [ ] Define required-criterion and quality-failure behavior, partial output, blockers, cancellation/interruption, missing evidence, and available cost/time fields without inventing a universal agent ability.
- [ ] Use independent declared reviewer/verification children or explicit user review. Worker self-assessments stay separate; reviewer evaluation cannot be self-awarded.
- [ ] Bind evaluations to the inspected result revision; stale or changed subjects cannot claim current review. Preserve credible conflicting evaluations and escalate instead of averaging them.
- [ ] Define authenticated reports, finite bounds, corrections/invalidation with provenance, stale/replayed/conflicting writes, deletion and retention; invalidated evidence stops influencing selection.
- [ ] Keep scoring unable to launch work, widen permission, advance undeclared dependencies, or replace user acceptance/merge approval. Include concrete contract examples and verification cases.

- [ ] **Step 4: Review through the consuming interfaces.** Check the artifact
  against the source design and dependency contracts; include concrete
  accepted/denied, missing-evidence, stale/replayed, and relevant recovery cases.
  Obtain written review of consequential decisions. Evidence gaps cannot become
  an assumed supported runtime capability.
- [ ] **Step 5: Validate and commit the specification.** Run the shared
  documentation commands below from the owned checkout, inspect their output,
  and commit the artifact on its branch:

```bash
rtk git add docs/superpowers/specs/2026-10-04-assignment-evaluation-design.md
rtk git diff --cached --check
rtk proxy bash scripts/doc-check.sh
rtk proxy pnpm --dir docs docs:build
rtk git commit -m "docs: specify assignment quality and completeness evaluation"
```

- [ ] **Step 6: Prepare the scoped execution handoff.** After its reviewed contract
  and upstream gates are satisfied, write a separate execution plan with actual
  types/interfaces, source/test files, and meaningful verification commands.
  Create implementation issues only for authoritative spec-covered work and
  link them to #744. Do not start code as part of this specification task.

## Task 6: Spec: repository-scoped configuration learning and skill improvements

**Tracker:** [#745](https://github.com/Rambolarsen/orkworks/issues/745)

**Files and ownership:**
- Create: `docs/superpowers/specs/2026-10-04-configuration-learning-design.md`.
- Own its reviewed specification/research handoff and subsequent scoped execution
  plan. Other tasks own their own artifacts; do not overwrite their edits or
  independently change shared configuration/projection terms.
- Read: `crates/orkworksd/src/taskmaster/context.rs`.
- Read: `crates/orkworksd/src/taskmaster/evaluator.rs`.
- Read: `crates/orkworksd/src/taskmaster/store.rs`.
- Read: `docs/superpowers/specs/2026-06-23-repo-learning-loop-design.md`.
- Read: `specs/taskmaster.md`.

**Interfaces:**
- Consumes: Versioned configuration identities, usage evidence and coverage, independent evaluations and invalidations, and the existing reviewable improvement workflow.
- Produces: A reviewed repository-scoped history/summary contract and deterministic eligibility rules for selection evidence. It defines explainable configuration advice, mandatory-skill preservation, skill-update draft triggers, rejection memory, and local retention/deletion.

**Steps:**

- [ ] **Step 1: Establish context and prerequisites.** Read both linked product
  specifications, the tracker and its dependency issues, applicable scoped
  instructions, and the listed source files. Record facts, proposed contract
  choices, and remaining eligibility questions in the artifact.
- [ ] **Step 2: Write the concrete artifact.** Use these sections and fill each
  with explicit decisions, identities, examples, limits, and failure behavior:

```markdown
# repository-scoped configuration learning and skill improvements
## Scope and status
## Repository identity and comparison cohorts
## Reported/observed rates and unknown denominator
## Outcome association and sparse evidence
## Versioned history and correction invalidation
## Bounded summaries supplied to the orchestrator
## Explainable future configuration choices
## Skill-improvement drafts and rejection memory
## Retention, forgetting, and concrete examples
## Verification cases
## Execution plan and implementation gate
```

- [ ] **Step 3: Check the task's full acceptance criteria.**

- [ ] Define canonical repository identity across its worktrees, comparison cohorts by role, task/configuration, coding tool/model, rubric ID/version, and effective permission profile/version. Cross-rubric comparisons require an explicit normalization rule in the reviewed evaluation contract; cross-repository pooling is excluded.
- [ ] Define reported-versus-observed usage rates, denominator and unknown counts, sample/coverage visibility, and optional-skill outcome comparisons that hold other relevant configuration dimensions fixed while allowing the candidate skill to vary; do not infer causality from association.
- [ ] Define inclusion/exclusion for stale, invalidated, blocked, interrupted, conflicting, and sparse evidence; skill/template updates do not rewrite historical scores.
- [ ] Specify a bounded local summary/read interface for the orchestrator and explainable selection reasons; mandatory repository skills and task fit remain binding.
- [ ] Changes only appear in a later proposed configuration and approval. Learning does not mutate active prompts, grant permissions, launch agents, or autonomously edit skills.
- [ ] Define recurrence thresholds, draft artifact/target selection, promotion approval, and compatibility with existing improve_workflow recommendations without duplicating their mutation path. Single runs may suggest; repeated comparable evidence supports future configuration changes. Suppress equivalent rejected suggestions until materially new relevant evidence appears and show that evidence when resurfacing.
- [ ] Specify numerical history/aggregate retention limits, forgetting and workspace deletion, missing-history fallback, and examples for configuration selection versus a proposed skill update.

- [ ] **Step 4: Review through the consuming interfaces.** Check the artifact
  against the source design and dependency contracts; include concrete
  accepted/denied, missing-evidence, stale/replayed, and relevant recovery cases.
  Obtain written review of consequential decisions. Evidence gaps cannot become
  an assumed supported runtime capability.
- [ ] **Step 5: Validate and commit the specification.** Run the shared
  documentation commands below from the owned checkout, inspect their output,
  and commit the artifact on its branch:

```bash
rtk git add docs/superpowers/specs/2026-10-04-configuration-learning-design.md
rtk git diff --cached --check
rtk proxy bash scripts/doc-check.sh
rtk proxy pnpm --dir docs docs:build
rtk git commit -m "docs: specify repository-scoped configuration learning and skill improvements"
```

- [ ] **Step 6: Prepare the scoped execution handoff.** After its reviewed contract
  and upstream gates are satisfied, write a separate execution plan with actual
  types/interfaces, source/test files, and meaningful verification commands.
  Create implementation issues only for authoritative spec-covered work and
  link them to #745. Do not start code as part of this specification task.

## Task 7: Spec: agent hierarchy interaction and visual presentation

**Tracker:** [#746](https://github.com/Rambolarsen/orkworks/issues/746)

**Files and ownership:**
- Create: `docs/superpowers/specs/2026-10-04-agent-hierarchy-ui-design.md`.
- Own its reviewed specification/research handoff and subsequent scoped execution
  plan. Other tasks own their own artifacts; do not overwrite their edits or
  independently change shared configuration/projection terms.
- Read: `apps/desktop/src/components/SessionListPanel.tsx`.
- Read: `apps/desktop/src/components/SessionDetailPanel.tsx`.
- Read: `apps/desktop/src/components/HarnessIcon.tsx`.
- Read: `apps/desktop/src/sessionGroups.ts`.
- Read: `apps/desktop/src/sessionSort.ts`.
- Read: `apps/desktop/src/workspaceSessionController.ts`.
- Read: `apps/desktop/src/api.ts`.
- Read: `apps/desktop/src/domain/session.ts`.

**Interfaces:**
- Consumes: Existing session lifecycle/selection/unread state and the product node design; final field agreement consumes configuration, usage, and evaluation contracts; final placement/navigation consumes accepted #755 shell direction and authoritative reconciliation.
- Produces: A reviewed interaction/projection contract and mockup fixtures covering planned/live/remembered nodes, skills, blockers, selection, collapse, accessible names, reduced motion, and secondary permission/evaluation detail. It introduces no privileged renderer control or second terminal.

**Steps:**

- [ ] **Step 1: Establish context and prerequisites.** Read both linked product
  specifications, the tracker and its dependency issues, applicable scoped
  instructions, and the listed source files. Record facts, proposed contract
  choices, and remaining eligibility questions in the artifact.
- [ ] **Step 2: Write the concrete artifact.** Use these sections and fill each
  with explicit decisions, identities, examples, limits, and failure behavior:

```markdown
# agent hierarchy interaction and visual presentation
## Scope and status
## Node identity and planned/live/remembered state
## Portrait, role, task, status, and skill badge priority
## Stable placement and collapse behavior
## Blocked-child and required-action discoverability
## Dependency labels distinct from parentage
## Single terminal and ordinary-session fallback
## Keyboard, accessibility, and reduced motion
## Contract fixtures and low/high-density mockups
## Verification cases
## Execution plan and implementation gate
```

- [ ] **Step 3: Check the task's full acceptance criteria.**

- [ ] Define root/child and planned/live/remembered states using supported OrkWorks identities; do not create separately controlled nodes for uncorrelated coding-tool-native subagents.
- [ ] Specify node content priority: portrait, role, short task, ordinary status and skill badges. Planned/selected badges are outlined, confirmed-loaded badges filled, recorded usage briefly highlighted, and unconfirmed delivery outlined with an accessible label. Preserve independent reported/observed usage evidence; neither selection nor a usage report proves loading, and missing usage is not unused.
- [ ] Define stable ordering/positions, automatic collapse after completed result summaries are viewed, manual reopening without repeated recollapse, focus/selection preservation, collapsed blocker and required-user-attention rollup, and explicit dependency/wait labels distinct from parentage.
- [ ] Preserve one selected terminal and existing lifecycle/unread/attention semantics, selection restoration, ordinary-session fallback, and workspace-switch stale-result fencing.
- [ ] Define keyboard navigation, focus behavior, screen-reader names/structure, non-color status communication, reduced-motion behavior, and labels accessible without hover.
- [ ] Provide restrained low/high-density mockups and cases for many tasks, long labels/skill sets, unknown telemetry, failed children, completed groups, and missing parents. Show one overall result with scores/permission details secondary; recorded usage gives a brief soft skill-badge highlight with source labels, replay deduplication, and a static reduced-motion indicator.
- [ ] Define renderer projection/interface fixtures with contract-owner agreement and a usability check centered on finding blockers/required actions. Preserve Electron-main authority and independent preload/renderer types.

- [ ] **Step 4: Review through the consuming interfaces.** Check the artifact
  against the source design and dependency contracts; include concrete
  accepted/denied, missing-evidence, stale/replayed, and relevant recovery cases.
  Obtain written review of consequential decisions. Evidence gaps cannot become
  an assumed supported runtime capability.
- [ ] **Step 5: Validate and commit the specification.** Run the shared
  documentation commands below from the owned checkout, inspect their output,
  and commit the artifact on its branch:

```bash
rtk git add docs/superpowers/specs/2026-10-04-agent-hierarchy-ui-design.md
rtk git diff --cached --check
rtk proxy bash scripts/doc-check.sh
rtk proxy pnpm --dir docs docs:build
rtk git commit -m "docs: specify agent hierarchy interaction and visual presentation"
```

- [ ] **Step 6: Prepare the scoped execution handoff.** After its reviewed contract
  and upstream gates are satisfied, write a separate execution plan with actual
  types/interfaces, source/test files, and meaningful verification commands.
  Create implementation issues only for authoritative spec-covered work and
  link them to #746. Do not start code as part of this specification task.

## Shared validation and completion

For every documentation task, use the owned worktree and installed locked docs
dependencies. Do not install packages or build in another task's checkout.

```bash
rtk proxy pnpm --dir docs install --frozen-lockfile
rtk git diff --check
rtk proxy bash scripts/doc-check.sh
rtk proxy pnpm --dir docs docs:build
rtk proxy bash .claude/hooks/worktree-check.sh
```

The specification review checks coverage, field/identity consistency, finite
limits, examples, unsupported combinations, review/approval boundaries, and
source links. Reuse current observation/reporting patterns only where their
semantics fit; source authentication does not prove a report's quality.

This planning session validates documentation and backlog records. It does not
run application tests, create runtime fixtures, start coding harnesses, or
claim that the product features are implemented. Later code plans must follow
the repository's applicable TDD/review/CI requirements.

## Spec coverage and readiness

| Product requirement | Owner |
| --- | --- |
| Clarify, skip preparation when appropriate, research before execution | #742; research contracts/configuration consume #741/#740 |
| Parent-only delegation, batches, capacity, approval, worktrees | Existing #610 scope alignment and baseline runtime |
| Task-specific role instructions, skill snapshots, permission profiles | #741; verified support #740 |
| Selected/loaded/reported/observed/unknown skills | #743; support evidence #740 |
| Quality/completeness, independent review, freshness and correction | #744 |
| Comparable repository-local history and future configuration choices | #745 |
| Reviewable skill updates and rejection memory | #745, existing improvement workflow |
| Portraits, tasks/status, badges, stable hierarchy, collapsed attention | #746 |
| Fixed shell, central navigation, layout migration and authoritative reconciliation | #755; final #746 placement/navigation consumes it |
| Single terminal, accessibility, reduced motion, secondary details | #746 and shell integration #755 |
| Bounded authenticated reports and history; ordinary-session compatibility | #741/#743/#744/#745, then baseline integration |

Tasks 1–3 have proposed draft artifacts: the capability register, role
configuration and preparation lifecycle. Tasks 4–7 remain queued. Existing
drafts still need written contract review and eligibility evidence; they are not
completed implementation handoffs. Continue the remaining drafting lanes and
review the concrete contracts before code execution planning becomes ready.
