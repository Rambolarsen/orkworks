# Agent hierarchy and configuration learning

- Status: proposed; work specification authorized, detailed contracts awaiting review
- Date: 2026-10-04
- Tracking issue: [#738](https://github.com/Rambolarsen/orkworks/issues/738)
- Launch baseline: [Taskmaster orchestrated child sessions](2026-09-26-taskmaster-orchestrated-child-sessions-design.md)
- Related learning proposal: [Repo learning loop](2026-06-23-repo-learning-loop-design.md)
- Existing observation model: [Workflow observation feedback loop](2026-08-14-workflow-observation-feedback-loop-design.md)
- Work specification plan: `docs/superpowers/plans/2026-10-04-agent-hierarchy-work-specification.md` (repository-only; plans are excluded from the published docs site).

## Scope alignment routing

The [#610 alignment](2026-10-04-taskmaster-orchestration-scope-design.md) and
[ADR 0077](../../adr/0077-taskmaster-orchestrated-child-sessions.md) carry the
ordinary-child/run-grant direction into the proposed authoritative extension.
Scope acceptance is recorded on PR #747; the detailed #740–#746
evidence/contract gates remain open. This is neither runtime implementation nor a supported tool profile.

## Purpose and status

Make an orchestrated workflow visible as a hierarchy of agents. Each agent has
a recognizable portrait, an assignment, a status, and a set of loaded skills.
The orchestrator chooses each child's configuration to suit its role and
work. Evidence from completed assignments informs future configurations and
reviewable skill improvements.

This document records the product direction agreed in the design conversation
and proposes concrete evaluation defaults for written review. It extends the
2026-09-26 launch proposal and preserves its approval, batch and worktree
contracts. The preparation contract proposes run-level planning authority
across completed research plans, separately gated execution grants and run-wide
capacity accounting. Baseline/authority scope alignment is accepted in PR #747; the detailed
component designs remain proposed.
Neither this document nor its tracking issue authorizes runtime work. The
authoritative specifications, architecture decisions, tracking issues, and
implementation plan must agree before implementation starts.

## Agreed product decisions

- The orchestrator assesses, clarifies, plans, selects agent configurations,
  delegates, and tracks. It delegates research, implementation, verification,
  and review work to children.
- A small, clear task can skip clarification and research. Research is used
  when specific unknowns could change the approach or assignment.
- Preserve parent-only delegation, ordered batches, and the approved live-child
  concurrency limit. Children do not spawn grandchildren through this workflow.
- Use recognizable agent portraits and compact skill badges with restrained
  animation. Task clarity and required user attention lead the visual hierarchy.
- The orchestrator selects skills for each role and task; loading is confirmed
  separately by adapter delivery evidence. The user
  reviews the workflow rather than manually equipping every child.
- Role-specific instructions and permissions accompany the assignment. Use
  verified coding-tool controls; OS-enforced confinement is outside this scope.
- Record skill usage and evaluate each assignment's work quality and completeness.
  Use the evidence to improve future configurations and propose skill updates.

## Preparation and approved execution

The orchestrator first reads the user's goal and available coordination
metadata, identifies uncertainty, and asks questions that could change the
scope, desired result, or approach. The assessment is a planning pass;
investigating the codebase or external sources is assigned to research children.
Missing material requirements trigger clarification. Silence is not an answer.

When research is needed, the orchestrator proposes a bounded research plan.
Each researcher has a distinct question, expected evidence, scope, and output
contract. The number of researchers follows independent questions and the
approved capacity limit. The user approves that exact plan before any research
child or plan-owned worktree starts. Reports identify sources, findings,
uncertainty, and recommendations; the orchestrator synthesizes those reports.

After research, the orchestrator proposes the execution plan using the findings.
Show a short research summary alongside the execution proposal: findings that
affect the approach, remaining uncertainty, and the resulting assignment choices.
Supporting sources and evidence remain available in research-agent details.
Keep the same orchestrator running: it automatically collects reports, prepares
the summary and proposes execution without a mandatory pause/resume step.
Verified same-parent event delivery is required; a live terminal alone does not
prove automated continuation. Research completion revokes that plan’s execution
grant while preserving run-bound planning authority. A fresh execution grant
requires approval of its exact proposal; final completion revokes run authority.
Count older research children against the run-wide live-child ceiling until
their sessions end. The [preparation contract](2026-10-04-orchestrator-preparation-design.md) defines this proposed capability amendment.
This is a separate approval when the execution assignments were not already
fixed in an approved plan. Completing research does not authorize execution.
The execution proposal includes required result-review tasks and all intended
implementation, verification, and remediation assignments. Small, clear work
can go directly to this proposal without a research plan.

### Planning parallel work

Use `orchestrating-task-graphs` as a required orchestrator planning skill when
proposing parallel work. Snapshot its selected version/content in the parent
configuration; its planning guidance does not grant runtime authority.
A parent expected to propose parallel work includes this skill in its reviewed
bootstrap configuration. Missing required content is a configuration blocker,
not permission to mutate the running parent silently.

Use the skill's independence test: two jobs run in parallel only when neither
requires the other's result. Add a dependency when one job consumes another's
output. Declare each job's question/assignment, inputs, expected artifact,
acceptance criteria, role/configuration, one artifact owner, dependencies and
agent cap before delegation. Convert the graph into the baseline's explicit
ordered batches and approved live-child concurrency limit; do not add a general
DAG scheduler, grandchildren or dynamic tasks through this skill.

Declare independent correctness and completeness review questions for parallel
outputs, assigning them to review/verification children within the approved
plan and capacity. Research-report synthesis is coordination by the parent.
Implementation outputs remain isolated for review and manual user integration;
the skill's single-owner synthesis guidance does not authorize the orchestrator
to edit, copy or merge code. One owner per artifact remains binding.

The displayed agent hierarchy communicates delegation. Task dependencies and
batch/wait labels communicate execution order; a parent-child edge alone does
not establish a task dependency. Small clear tasks do not gain extra research
or workers merely because a graph skill is loaded.

Preserve the baseline's explicit batches, dependencies, launch idempotency,
one attempt per task, and capacity accounting. Independent task chains use
separate worktrees. Dependent reuse requires the predecessor session to end
and the user's quiescence acknowledgement. A task report, skill event, score,
portrait animation, or idle hook never grants launch authority.

Additional tasks, retries, changed assignments, or broader authority require a
new exact plan revision and approval under the baseline. Child changes remain
available for manual integration. This extension does not combine code or
commit, merge, push, delete branches, or automatically clean up worktrees.

## Agent configuration

The orchestrator assembles a versioned configuration for each assignment:

| Part | Contents |
| --- | --- |
| Shared instructions | Repository rules, reporting contract, approval and escalation requirements |
| Role template | Responsibilities, expected evidence, stopping conditions, permitted coordination actions |
| Assignment | Task category, scope, dependencies, inputs, output contract, acceptance criteria |
| Skills | Selected skill IDs, versions/content digests, required/optional classification, selection reasons |
| Coding tool and model | Approved harness/model identity and relevant definition/version |
| Permissions | Requested profile, effective coding-tool settings, allowed resources/actions, support evidence |

Use reusable role templates. The orchestrator selects a template and fills its
assignment fields instead of improvising the entire instruction set. Mandatory
repository skills remain included. A selected skill never grants permissions or
overrides a repository rule. Conflicting instructions must be resolved before
launch or escalated to the user.

The approved definition includes the effective OrkWorks-supplied instructions,
skill content identities, task rubric, and permission settings. Store the
rendered configuration and its digest separately from mutable run evidence.
Do not claim to capture private vendor system instructions. A coding tool may
support a native system/agent prompt or only startup instructions; the adapter
must state which mechanism it uses and preserve mandatory harness policies.

The launch adapter revalidates the approved versions and settings. Drift
requires a revised proposal rather than silent substitution. Configuration
changes informed by learning take effect in a later approved assignment;
learning does not alter running agents.

## Coding-tool permissions

Define role defaults, then narrow them to the assignment:

| Role | Default permitted work |
| --- | --- |
| Orchestrator | Clarification, plan proposal, skill selection, approved child coordination, reading child reports |
| Research | Read declared sources and files; search external sources when approved; report findings |
| Implementation | Edit declared worktree scope and run declared development commands |
| Review | Inspect declared artifacts and evidence; report findings without editing product files |
| Verification | Run approved checks with explicitly allowed temporary/build/test output |
| Remediation | Edit and check the declared finding scope in a newly approved or already declared task |

Apply the profile before launch, using coding-tool tool restrictions, permission
modes, filesystem settings, and network/command controls where supported.
Broad inherited tools, shell execution, MCP access, or connectors must not
silently bypass the requested profile. In particular, disabling an edit tool
while leaving an unrestricted shell is insufficient to label a profile read-only.

Only offer a role/tool combination as permission-restricted after its adapter
has version-specific evidence that the needed controls apply. The launch path
rejects unsupported or unverified profiles and explains the missing control;
the user can choose a supported tool or approve a revised assignment. The
system never silently substitutes prompt-only restrictions.

Children request missing access through a scoped blocker report. The
orchestrator may propose a revised configuration but cannot broaden approved
permissions itself. Changes require approval. Skills, evaluation scores, and
child self-reports cannot authorize additional access.

These controls govern the selected coding tool. Children remain ordinary
same-user sessions under the launch baseline; this design does not promise OS
isolation, credential isolation, process-origin proof, or protection against a
same-user process directly calling ordinary sidecar endpoints. Permission
details show that boundary without adding security decoration to every node.

## Skill usage evidence

Use a dedicated record associated with one plan, task, child session, and skill
version. Existing workflow observations remain evidence about workflow friction;
they are not reinterpreted as exact skill telemetry.

| State | Evidence |
| --- | --- |
| Selected | The approved configuration includes the skill |
| Loaded | The adapter confirms content was supplied to the child's starting context |
| Reported used | The child identifies where it applied the skill and supplies references |
| Observed used | A supported adapter records a recognized invocation for that skill |
| Unknown | The integration cannot establish delivery or usage |

Selection does not prove delivery; delivery or invocation does not prove
instruction adherence or benefit. Preserve event origin, observation time,
coverage, skill identity, and bounded evidence references. Claims from the
agent remain labeled as reports, even when authenticated. Peon inference
remains inferred evidence and cannot become an observed invocation.

Count an invocation once using a stable event identity. Replayed receipts do
not increase usage. A terminal mention of a skill is not an invocation event.
Do not ingest full prompts, secrets, hidden reasoning, or complete transcripts
to decorate a skill badge. Broken or unsupported telemetry remains unknown.

## Assignment evaluation

Score the configuration on its particular assignment. The portrait represents
the session; it does not acquire a universal ability rating or permanent XP.
The orchestrator plans a distinct reviewer/verification child when the result
needs evaluation. Review children evaluate assigned results; the orchestrator
collects reports and applies the declared coordination rules without doing
the review itself.

Show one overall evaluation result on each agent: **Meets requirements**,
**Needs rework**, or **Unassessed**. Opening it reveals completeness, quality,
reviewer findings, and supporting evidence. These labels summarize an evaluation;
they do not mean user acceptance or authorize integration.

Derive the result from current, non-conflicting evidence. A known unsatisfied
required criterion or quality below the declared standard means Needs rework,
even if other details remain unassessed. If no failure is established but required
evidence is missing, stale, or disputed, show Unassessed. Meets requirements
requires every required criterion satisfied and quality meeting the declared
standard. Preserve blockers/cancellation/interruption as separate run statuses;
they do not automatically assign a poor evaluation.

Acceptance criteria and a role-appropriate quality rubric are part of the
approved task definition. New root/child configurations require at least one
required criterion; empty or optional-only sets fail approval validation. A
legacy/malformed evaluation with no required criteria has no completeness
percentage and cannot yield Meets requirements. Keep it Unassessed unless
another known failure establishes Needs rework; never calculate `0 / 0`.
The following are proposed score-detail defaults:

- Completeness: record each required criterion as satisfied, unsatisfied, or
  unassessed. When all are assessed, compute the satisfied count divided by
  the total required count as a percentage. When evidence is missing, show
  the counts and an unassessed score rather than excluding unknown criteria.
- Quality: use an ordinal rubric: 0 = unusable result, 1 = major rework,
  2 = limited rework, 3 = meets the declared standard. The reviewer cites
  evidence against the task's named quality criteria. Missing evidence makes
  quality unassessed. Correctness, clarity, scope adherence, maintainability,
  and verification matter only as declared for that role and assignment.
- Retain blockers, outcome, review findings, required rework, and available
  time/cost evidence alongside the two scores. Blocked, cancelled, interrupted,
  or unsupported runs are not automatically low-quality runs.

Evaluation binds to the result/artifact revision actually inspected. If that
result changes, preserve the earlier evaluation as historical and require a
new evaluation before describing the changed result as reviewed. Disagreement
between credible evaluations is visible and escalated rather than averaged away.
Worker self-assessments remain separate from reviewer assessments. Evaluation
of a reviewer must come from another declared reviewer or the user, and cannot
be a self-awarded quality score.

Quality and completeness are evaluation evidence. They do not replace user
acceptance, merge approval, or the baseline's parent-reported dependency gate.
A result can be fully complete and still fail the quality rubric.

## Configuration learning

Keep first-version history repository-scoped, preserving the existing learning
proposal's boundary. Cross-repository pooling requires a separate design.
Compare assignments by role, task category, relevant scope, coding tool/model,
role-template version, skill versions, and effective permission profile.
Display sample counts and evidence coverage; sparse or incompatible history
does not establish a reliable ranking.

For each skill, retain usage counts within comparable assignments, separating
reported and observed usage, plus associated assignment outcomes. Exclude
unknown usage from rate calculations and display the unknown count alongside
the denominator. Do not equate use frequency with quality or automatically
assign every loaded skill the task's quality score. Multiple skills, prompts,
models, permissions, and task difficulty can explain a result.

Use a learning loop of run, independent evaluation, comparison with similar work,
and improvement of a later configuration. A single run can raise a suggestion;
repeated comparable evidence supports changing optional skills, role instructions,
or task scope. Neither frequency of use alone nor unknown usage establishes
that a skill helps or should be removed. Repeated established non-use can suggest
a future trial without an optional skill; mandatory skills remain included.
Numeric evidence thresholds and comparison eligibility are specified by #745.

Remember rejected suggestions and suppress equivalent repeats until materially
new relevant evidence appears. New evidence must be visible with the renewed
suggestion; a rerun of the same analysis or passage of time alone is insufficient.
Rejection is not permanent skill retirement and does not invalidate historical
results. The exact equivalence/evidence-change contract belongs to #745.

The orchestrator uses task fit and mandatory rules first, then relevant history
to select optional skills and role settings. It records reasons for changed
choices. Learning can suggest a different skill combination, narrower task,
better context, or revised role instructions in the next proposed plan.
Recurring evidence can also support a draft skill update through the existing
reviewable setup-learning/workflow-improvement path. Drafting does not modify
repository assets, promote a skill, or grant launch authority.

Persist the exact historical configuration so an updated skill or role template
does not rewrite old scores. User correction or invalidation of an evaluation
must retain its provenance and remove it from active selection evidence.

## Hierarchy UI

The existing Sessions surface presents the parent and its declared children.
Planned tasks use a visibly planned state; a spawned child becomes an agent
node with its own ordinary session identity. Native coding-tool subagents are
not represented as separately controlled OrkWorks children without their own
supported identity contract. Preserve the existing single selected terminal.

Each agent node displays a recognizable portrait, role, short task label,
ordinary status, and compact loaded-skill badges. Portraits can reuse a stable
role visual while accessible text distinguishes individual agents. Parent-child
lines show delegation. Dependencies and waiting reasons use explicit labels;
a line between agents never implies a dependency that the plan does not declare.

Keep positions stable as status changes. Automatically collapse a completed
group after its current result summaries have been viewed. The user can reopen
it; do not repeatedly recollapse that same viewed result. Active work stays
expanded by default. Preserve selection and keyboard focus rather than hiding
the actively inspected child. Completion/collapse does not release live-session
capacity or change task outcomes. Keep blockers and required user decisions
discoverable when children are collapsed. Selecting an agent opens its normal terminal and
details; selecting a skill exposes its description, version, usage evidence,
and relevant history. Permissions and detailed scores belong in details and
run summaries, with the overall evaluation result on the agent.

Skill badges show a progression from outlined (planned/selected) to filled
(confirmed loaded), followed by a brief highlight when usage is recorded.
Delivery unconfirmed keeps an outline and an accessible unconfirmed label;
selection or usage reports cannot manufacture a confirmed-loaded receipt.
A usage report can highlight an outlined badge while delivery remains unconfirmed;
the two evidence states remain explicit in details. Missing usage evidence does
not mean unused. Skill descriptions, versions, selection reasons and evidence
remain available in details.

Recorded skill usage gives its badge a brief, soft highlight. Deduplicate event
replays and keep bursts calm; the animation is supplementary feedback, not a
reward or quality assertion. Distinguish reported and observed usage in the
accessible label and details. Reduced-motion mode uses a static indicator.
Other spawn/status/result feedback remains brief and restrained. Animation is supplementary; respect reduced
motion, support keyboard navigation, and communicate status without color
alone. Decorative skill icons have names available without hover.

This first version adds no XP economy, reward currency, streak obligations,
collectible rarity, persistent character leveling, or activity-based ranking.

## Architecture and implementation prerequisites

The implementation plan should define narrow owners for configuration
validation/launch mapping, skill evidence, evaluation/history, and renderer
projections, using the existing sidecar/session runtime and Electron authority
boundary. Renderer presentation cannot grant tools, mutate approved plans,
or choose an unverified permission fallback.

Before implementation:

1. Obtain review of this written design and its proposed scoring defaults.
2. Align authoritative Taskmaster/MVP scope, relevant ADRs, the baseline design,
   its implementation plan, and tracking issues. Preserve historical proposals.
3. Verify each initial coding-tool/version combination's prompt, skill delivery,
   permission, and evidence contracts. Unsupported combinations stay unavailable.
4. Define authenticated, versioned, idempotent reporting and correction contracts,
   explicit finite field/collection/storage bounds, retention/deletion behavior,
   and recovery behavior consistent with existing metadata ownership. Do not
   implement unbounded history or silently publish local learning.
5. Write and review an implementation plan with scoped deliverable issues;
   prioritize and sequence work through the repository's issue workflow.

Validation must cover skipped versus necessary preparation, research approval,
immutable configurations, unsupported permission controls, broader-access
requests, delivered versus selected skills, replayed/unknown usage, stale and
conflicting evaluation evidence, mandatory-skill preservation, approved future
changes, and unchanged launch/dependency/acceptance boundaries. UI validation
must exercise keyboard/reduced-motion behavior, collapsed blockers, many tasks,
and finding required user attention quickly in restrained visual mockups.

## Component specification progress

- [Role configuration and coding-tool permissions](2026-10-04-agent-role-configuration-design.md) — proposed immutable contract; support and written review remain gates.
- [Clarification and preparation lifecycle](2026-10-04-orchestrator-preparation-design.md) — same live parent, automatic proposal preparation, separate exact-plan execution grants; proposed contract awaiting review.
- [Coding-tool capability register](../../validation/agent-role-capabilities.md) — version-specific delivery/permission evidence; no verified launch slice is claimed.

## Evidence and unresolved implementation work

The current observation design records workflow friction with source and
confidence; it does not specify dedicated loaded-skill or invocation records.
The existing integration contracts do not provide a portable per-skill usage
interface. New telemetry needs verified adapters rather than assumed hook
coverage.

The launch baseline explicitly excludes OS confinement and preserves ordinary
child sessions. Coding-tool permission profiles amend that proposal at the
tool configuration layer. Per-tool control verification and the concrete
bounded reporting/storage contracts remain prerequisites for the separately
reviewed implementation plan, not claims of implemented capability.

The visual direction and configuration feedback loop are agreed product scope.
The user has authorized progression to scoped work specifications. The proposed
numeric/ordinal rubric, repository-scoped history, coding-tool support, and
reporting contracts remain explicit review items in issues #740–#746 and the
linked work specification plan. No runtime feature or public installer claim
follows from saving or planning the proposal.
