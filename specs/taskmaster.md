# Taskmaster — Cross-Session Coordination Spec

Status: accepted
Date: 2026-06-19

## Summary

Taskmaster is the workspace-level coordination layer in OrkWorks.

Peons observe individual sessions and normalize what is happening. Taskmaster consumes those reports together with Git context, capacity, harness configuration, recommendation history, and user preferences, then proposes the best next action.

The core relationship is:

```text
AI session
    ↓ terminal output and explicit agent metadata
Peon
    ↓ normalized session state and events
Taskmaster
    ↓ evidence-backed next-step recommendation
User
    ↓ explicit approval
OrkWorks
    ↓ starts, focuses, or resumes a session
```

Taskmaster does not perform implementation or review work itself. It decides what kind of work should happen next, recommends the most suitable harness/model, and prepares the handoff for user approval.

This spec introduces **Taskmaster** as an approved OrkWorks product term. It supersedes the earlier naming restriction in `specs/orkworks-mvp.md` that Peon must be the only fantasy-themed product term. Normal engineering terminology should still be used for all other concepts.

## Motivation

Session observability is useful, but observation alone still leaves the user coordinating the workflow manually.

A Peon may report:

- implementation is complete
- tests passed
- the session is waiting for review
- a command failed repeatedly
- the model is near its context limit
- the session needs a product decision

Without a coordination layer, the user must interpret every report, choose the next workflow step, select a harness/model, create the next session, and write the handoff prompt.

Taskmaster turns normalized session state into an actionable recommendation.

Example:

> The implementation session reports that the change is complete and tests pass. No independent review exists. Start a read-only review session using Codex with a strong model before involving the user.

The intended outcome is not full autonomy. It is fewer unnecessary interruptions and better use of cheap and strong models throughout a development workflow.

## Product roles

### Peon

Peon is session-scoped or repo-scoped observation.

A session Peon answers:

- What is this session doing?
- What phase is it in?
- Is it working, blocked, failed, stale, or waiting for input?
- What changed?
- What tests ran?
- What should probably happen next inside this session?
- How confident is the observation?

A repo Peon may summarize repo-level signals (the repo-scoped Peon from ADR 0012), but it does not coordinate sessions.

### Taskmaster

Taskmaster is workspace-scoped coordination.

Taskmaster answers:

- What should happen next across the workspace?
- Does completed work need independent review or verification?
- Should the next action use a cheap model or a strong model?
- Should work continue in the existing session or move to a fresh session?
- Is a session blocked on the user, or can another agent perform the next pass first?
- Can another session safely start in parallel?
- Is the suggested action still relevant, or has it been superseded?

### User

The user remains the authority.

The user:

- approves or dismisses Taskmaster recommendations
- decides product and architectural questions
- accepts completed work
- controls merges and destructive actions
- may override model, harness, prompt, or working directory before starting a recommended session

### OrkWorks runtime

The OrkWorks runtime performs approved actions using existing session-management capabilities.

It may:

- focus an existing session
- start a new session after explicit approval
- prepare a handoff prompt
- link the new session to the source session and recommendation
- mark the recommendation as executing or completed

## Design principles

### Peons report facts; Taskmaster recommends transitions

Peons should not decide the cross-session workflow. Taskmaster should not independently reinterpret raw terminal output when normalized Peon metadata is available.

### Recommendations must be explainable

Every recommendation must include evidence and plain-language reasoning.

Bad:

> Start Codex.

Good:

> Start Codex for an independent review. The implementation session reports completion, all 42 tests passed, the change affects retry behavior, and no review session is linked to this work.

### Independence is valuable

When recommending review, Taskmaster should prefer a new session and, where practical, a different harness or model family from the implementation session.

Independence is a preference, not a hard requirement. Capacity, cost, local availability, and user configuration may justify reusing the same provider.

### Strong models should be used deliberately

Taskmaster should preserve expensive or premium models for work where they provide disproportionate value, such as:

- architecture review
- high-risk code review
- difficult debugging after cheaper attempts fail
- security-sensitive analysis
- resolving conflicting findings

### The user should be interrupted at the right time

Taskmaster should route mechanical review, verification, and summarization through agents before asking the user to inspect work.

It should still involve the user immediately for:

- product decisions
- ambiguous requirements
- credentials or permissions
- destructive actions
- merge approval
- conflicting high-confidence reviews
- acceptance of high-risk changes

## Scope for v1

Taskmaster v1 is a recommendation engine with one-click, user-approved session transitions.

It includes:

- consuming normalized session metadata and events
- evaluating deterministic coordination rules
- ranking suitable harness/model choices
- proposing the next workflow action
- preparing a handoff prompt
- persisting recommendation lifecycle and history
- presenting recommendations in the desktop UI
- starting or focusing a session only after explicit user approval
- linking recommended sessions back to their source session and recommendation chain
- preventing duplicate and looping recommendations

## Out of scope for v1

Taskmaster v1 does not:

- start sessions without user approval
- type into existing terminals
- approve commands
- merge, rebase, reset, stash, or delete work
- declare work accepted on behalf of the user
- perform arbitrary task decomposition
- maintain a Jira-style task board
- replace Peon session observation
- parse all raw terminal output independently of Peon
- run an unrestricted autonomous multi-agent swarm
- create or clean up Git worktrees outside the separately gated proposed ordinary-child orchestration extension
- send review findings into a running terminal automatically
- keep chaining sessions indefinitely

## Coordinator design gate

The data-only coordinator foundation from
[#604](https://github.com/Rambolarsen/orkworks/issues/604) and
[#606](https://github.com/Rambolarsen/orkworks/issues/606) is implemented.
It persists definitions and approvals without runtime authority. The former
brokered coordinator and confined runner are historical proposals, retained in
[ADR 0064](../docs/adr/0064-bounded-taskmaster-coordinator.md) and the
[old runner design](../docs/superpowers/specs/2026-09-25-master-session-parallel-runner-design.md).

### Proposed ordinary-child orchestration extension

[ADR 0077](../docs/adr/0077-taskmaster-orchestrated-child-sessions.md) and
[#610 scope alignment](../docs/superpowers/specs/2026-10-04-taskmaster-orchestration-scope-design.md)
record the replacement direction. This section is a proposed extension to the
accepted v1 recommendation contract. Its product/architecture scope is accepted;
detailed component review, capability evidence and scoped execution-plan
approval remain required.
It enables no runtime API and does not change v1's per-action approvals.

A user explicitly creates an orchestrator run with an immutable reviewed
bootstrap, goal and live-child ceiling. The parent coordinates; investigation,
implementation, review and verification are delegated to ordinary sessions in
the existing selected-workspace sidecar and metadata store. One terminal is
selected. No child sidecars or cross-instance authority are introduced.

Each exact immutable plan revision declares tasks, prompts/configuration and
input digests, harness/model identities, ordered batches, dependencies, the
clean repository/base revision, worktree paths/branches and concurrency cap.
Electron-main approval is required before any plan-owned branch, worktree or
child is created. A run planning bearer cannot approve work; launches require
the separate server-held grant for that exact revision and fresh revalidation.
The [preparation contract](../docs/superpowers/specs/2026-10-04-orchestrator-preparation-design.md)
governs run/grant identity, numeric bounds, admission fencing and recovery.

Successful research ends that plan's launch grant while the same parent's
planning authority continues to synthesis and an execution proposal. Every new execution plan needs its own exact-plan approval; research
completion never transfers approval into that new plan. Only one
revision holds a launch grant per run. All nonterminal children from every plan
and unattached reservations consume the run cap, even after a task/plan result.
Final execution completion or run cancellation revokes run authority. After
completed research, exact version-bound UI finish can complete a research-only
run or decline execution, including a no-go conclusion; it revokes all authority,
is allowed only before execution approval, invalidates pending proposals and
requires settled reservations and no interrupted allocation. This is coordination completion, not result acceptance. Parent
end and workspace/sidecar changes revoke volatile authority and pause the run.
Exact-identity UI resume restores fresh authority, with exact-plan reapproval
before launches and no automatic replacement parent or child relaunch.

Independent chains use separate approved worktree groups. Dependent reuse
within a plan requires a terminal predecessor and explicit user quiescence
acknowledgement, not OS proof; no cross-plan reuse or automatic integration is
introduced. Owned-child resume serializes with successor admission and enforces
current group ownership and live-child capacity. Successor reservation durably
transfers ownership and denies predecessor resume thereafter, even once the
successor ends; a pre-handoff resume invalidates earlier quiescence evidence.
Resume cannot retry a task or revive a grant. Allocation intent and one task reservation are durable before
mutation/spawn. Duplicate requests cannot launch twice; ambiguous recovery
blocks admission rather than guessing or retrying. A failed/blocked task pauses
launches. Added/retried work requires a new approved revision.

An authenticated turn receipt, the explicit UI ready-for-review action, or
terminal reconciliation permits an exact version-bound parent result. Only
that explicit coordination result advances declared dependencies. Process exit,
terminal text, hooks and research reports alone do not prove success; a parent
result is neither independent quality review nor user acceptance. Missing,
stale, conflicting or oversized evidence cannot advance undeclared work.

The [role contract](../docs/superpowers/specs/2026-10-04-agent-role-configuration-design.md)
requires version-specific evidence for coding-tool permissions and content
delivery under [#740](https://github.com/Rambolarsen/orkworks/issues/740).
Unverified profiles block launch without silently widening access. Automatic
parent continuation needs a verified machine-readable event/wait channel; a
live PTY is insufficient. Roles, skills and scoring do not grant authority.
Collection, persistence, replay, retention and deletion must obey the reviewed
finite contracts and preserve referenced ownership/evidence; exhausted bounds
block admission instead of dropping recovery proof.

This is a workflow/coding-tool boundary, not native confinement or OS process
authentication. Ordinary host permissions and logins remain; same-user
processes may inspect/replay environment bearers and may directly call the
unauthenticated ordinary `POST /sessions` route. The UI token remains in
Electron main and the sidecar, withheld from renderer and coding-tool
environments. Native confinement, credential isolation, hard resource budgets,
recursive delegation and automatic integration are outside this slice.
ADR 0060's independent-instance cleanup/replacement proof remains unchanged.

Worktrees and children remain for ordinary management/manual integration after
plan completion, cancellation or parent exit. Orchestration never transfers
edits, commits, merges, rebases, pushes, changes existing branches or deletes
branches. The initial slice does not automatically clean up worktrees; any
later separately reviewed removal must be clean, quiescent and plan-owned.
Product/architecture decisions, ambiguous requirements, credentials/permissions,
destructive actions, Git mutation or merge approval, conflicting high-confidence
results and high-risk acceptance remain mandatory user escalations.

The [ordinary-child baseline](../docs/superpowers/specs/2026-09-26-taskmaster-orchestrated-child-sessions-design.md),
[hierarchy work specification](../docs/superpowers/plans/2026-10-04-agent-hierarchy-work-specification.md)
and [scoped baseline handoff](../docs/superpowers/plans/2026-09-26-taskmaster-orchestrated-child-sessions.md)
define the review sequence. Documentation alignment is not launch eligibility
or approval of runtime implementation.

## Inputs

Taskmaster evaluates workspace-level state from the following sources.

### Session snapshots

From `.orkworks/sessions/<session-id>.json` and the backend session registry:

- lifecycle status
- observed status
- phase
- task description
- summary
- next action
- question and suggested options
- files touched
- commands run
- test status and summary
- metadata source and confidence
- working directory
- branch and worktree context
- harness and model
- context usage where available
- source recommendation or parent session

### Session events

From `.orkworks/events/<session-id>.ndjson`:

- meaningful progress
- waiting and blocker transitions
- test runs
- failures
- completion
- review findings
- user decisions

### Git context

- repository root
- branch
- working directory
- worktree identity
- dirty state
- changed file count
- shared-working-directory conflicts

### Capacity and cost

- healthy, degraded, capped, unknown, or disabled state
- reset time where known
- local, low, medium, high, or premium cost tier
- current active sessions per harness/model

### Harness configuration

- task fit
- model capabilities
- configured commands
- review suitability
- cost preferences
- provider availability

### Recommendation history

- active and completed recommendations
- prior review attempts
- prior verification attempts
- dismissed recommendations
- chain depth
- model/harnesses already used

### Workflow observations

From the shared workflow-evidence module (`workspace_observations`), not from raw event or terminal text:

- immutable `WorkflowObservation` records: `id`, `sequence`, `sessionId`, `observedAt`, `kind`, `description`, optional `problemArea`, `evidence`, `reportedImpact`, `source`, `confidence`, `fingerprint`
- accepted within the active workspace only, ordered by `sequence`

Taskmaster reads workflow observations for a different purpose than session snapshots: session snapshots (including the current-summary snapshot — `summary`/`summarySource`/`summaryConfidence`/`summaryObservedAt`, see `specs/orkworks-mvp.md`) describe current work for coordination and handoff prompts; workflow observations describe durable friction used only to propose workflow improvements. Taskmaster never parses activity-summary prose to manufacture workflow evidence, and it never mutates or amends a stored observation. See [ADR 0042](../docs/adr/0042-workflow-observations-replace-summary-checkpoints.md) for the full rationale.

### User preferences

Examples:

- preferred reviewer harness
- whether independent model families are preferred
- maximum recommendation chain depth
- maximum premium-model usage
- whether low-risk verification suggestions should be suppressed
- actions that always require direct user involvement

## Evaluation triggers

Taskmaster reevaluates when:

- a session snapshot changes materially
- a session lifecycle state changes
- a Peon changes `observedStatus`
- a new session event is appended
- a recommendation is accepted, dismissed, completed, or superseded
- capacity state changes
- Git context changes materially
- a linked child session finishes
- a workspace is opened and persisted recommendations are restored
- a periodic stale-state check runs

Cosmetic metadata changes should not cause reevaluation.

## Recommendation types

Initial recommendation types:

- `start_review_session` — create an independent, read-only review pass
- `start_verification_session` — run tests, checks, or evidence gathering without changing implementation
- `retry_with_stronger_model` — retry failed or stuck work with a stronger model
- `start_fix_session` — address findings from a review or verification session
- `resume_source_session` — return findings to the original session for continued work
- `start_fresh_handoff_session` — continue work in a new context when the existing session is exhausted or stale
- `focus_session` — bring an existing session requiring user input to the foreground
- `request_user_decision` — surface a question that should not be delegated
- `wait_for_capacity` — postpone a model-specific action until capacity resets
- `avoid_parallel_session` — warn against starting more work in a shared dirty workspace
- `archive_completed_session` — suggest clearing completed runtime clutter after downstream work is complete
- `improve_workflow` — passive, evidence-backed suggestion to update instructions, a skill, a test, tooling, or documentation, derived from correlated `WorkflowObservation` records (see "Workflow-improvement recommendations" below)

Recommendation types describe intent. The shared recommendation engine selects the best available harness/model for intents that require a new session.

## Initial coordination rules

The first implementation should be deterministic and testable.

### Independent review

```text
WHEN an implementation session reports review-ready or completed work
AND tests are not known to be failing
AND no independent review is active or completed for the current change
THEN recommend start_review_session
```

Prefer a strong, healthy model. Prefer a different model family from the implementation session where practical.

### Verification before review

```text
WHEN implementation is reported complete
AND tests were not run or evidence is insufficient
THEN recommend start_verification_session
```

A verification session may use a cheaper model unless the change is high risk.

### Escalation after repeated failure

```text
WHEN the same work has failed or become blocked repeatedly
AND previous attempts used a low-cost model
THEN recommend retry_with_stronger_model
```

The reason must identify the failed attempts and explain why escalation is justified.

### Review findings

```text
WHEN a review session reports actionable findings
THEN recommend resume_source_session or start_fix_session
```

Prefer resuming the source implementation session when it still has usable context and is available. Prefer a fresh fix session when the source session is ended, near its context limit, or unsuitable for the findings.

### User-owned decisions

```text
WHEN a session is waiting on product intent, architecture approval, credentials, permissions, or a destructive action
THEN recommend request_user_decision or focus_session
```

Taskmaster must not route these decisions to another coding agent as though they were implementation work.

### Context exhaustion

```text
WHEN a session is near its context limit
AND meaningful work remains
THEN recommend start_fresh_handoff_session
```

The handoff should include the source summary, current state, tests, files touched, unresolved questions, and next action.

### Shared workspace risk

```text
WHEN another active coding session would share a dirty working directory
THEN recommend avoid_parallel_session
```

Taskmaster may explain that a separate worktree would reduce risk, but v1 does not create one.

### Completion after review

```text
WHEN implementation and independent review are complete
AND no unresolved findings remain
THEN surface the work as ready for user acceptance
```

Taskmaster may say that the work is ready for the user. It must not mark the work accepted or merge it.

## Workflow-improvement recommendations

### Guided completion packets (Phase 1)

Taskmaster may carry an optional `completionPacket` projection on an existing
recommendation. A packet is scoped to exactly one source session, one
workspace-local change subject, and one workspace snapshot. If concurrent dirty
workspace activity makes attribution ambiguous, the packet records
`inconclusive` attribution and cannot claim review readiness.

The packet stores versioned evidence and provenance: source session,
observation time, workspace and snapshot identity, verification command/result
and applicable revision, independent reviewer identity/outcome and revision,
the proposed action, and explicit missing or conflicting evidence. Readiness is
derived and limited to `verification_needed`, `review_ready`,
`findings_need_fix`, or `ready_for_user_review`; the last state means only that
the work is ready for the user to inspect and never means user acceptance.

Packets and their mutations are bound to an immutable revision and evidence
fingerprint. Accept and complete requests must carry the current revision,
fingerprint, and an idempotency key. Prompt, model, scope, role, or evidence
changes create a superseding packet revision, preserve lineage, clear prior
approval, and return the recommendation to `proposed`. Stale, malformed,
cross-workspace, unauthorized, cancelled, or late results are rejected. A
missing target session can be recovered to `proposed` without discarding the
packet evidence.

This projection does not add a second lifecycle system or change the existing
single-active-context and explicit-approval rules. The authenticated
`completion-packet` report only updates an existing recommendation; it cannot
start a session, focus a terminal, edit files, mutate Git, or delegate work.

Recursive coordination is outside the proposed ordinary-child slice.
Child-session orchestration remains deferred behind the
[Coordinator design gate](#coordinator-design-gate) and proposed
[ADR 0077](../docs/adr/0077-taskmaster-orchestrated-child-sessions.md). Written
acceptance of that design authorizes only a separate implementation plan and
its review; coordinator implementation still requires that plan's approval.

`improve_workflow` is the passive variant of the canonical recommendation contract described above. It never resumes or focuses an existing session, and it never edits repository files, instructions, skills, tests, or tooling itself — the only exception is through the explicit `accept` action described below, which starts no new session but submits a prompt into a session the user is already running. It exposes one explicit, user-confirmed `accept` action that sends a prompt derived from the recommendation into the user's currently active session, scoped to editing the recommended target surface — it never resumes, reopens, or modifies any session that supplied evidence for it.

### Eligibility

Taskmaster reevaluates five seconds after the latest accepted workflow observation in the active workspace, so a burst of related records can be considered together, and reconstructs its view from persisted observations after a restart. A deterministic evaluator considers a cluster eligible only when it contains at least two distinct observations sharing a fingerprint, each with confidence ≥ `0.6`; high-impact observations additionally require confidence ≥ `0.8`. A single observation, however confident or impactful, never proposal-qualifies — it remains stored as supporting context until a second distinct observation shares its fingerprint.

Two inference results over the same unchanged Peon evidence window count as one observation; a genuinely repeated action produces a later evidence range and therefore a distinct, separately-countable occurrence. Recurrence may span one session or multiple sessions; the recommendation states which. Exact evidence families remain deterministic audit units. A separate bounded rollup layer may combine related proposed exact families; it does not replace exact identity or allow generated prose to become evidence. Observations below `0.6` confidence, and high-impact observations below `0.8` confidence, are not cited or counted, though they may remain stored as supporting context.

### Identity and rollup contract

New workflow observations may include an optional, short `problemArea`
separate from the user-visible `description`. The sidecar validates and
normalizes this field with Unicode NFKC, Unicode lowercase, trimmed edges, and
every run of Unicode whitespace collapsed to one ASCII space; punctuation is
retained. The fingerprint input concatenates `kind`, one NUL byte (`U+0000`),
and `canonical_problem_area`; new records store
`v2:<kind>:<sha256-hex>`. The field is bounded to a non-empty,
non-control value of at most 120 characters. Explicit overlong values are
truncated; explicit empty or control-bearing values are rejected. Missing
`problemArea` preserves the legacy v1 description-based fingerprint, may be
derived in memory for model context, and is never rewritten during a read.
Wire and persisted values default to `null`.

After exact-family evaluation, only currently proposed exact families are
eligible for a semantic rollup. If a configured Taskmaster model is
unavailable, exact recommendations continue to work and no rollup is
created. A rollup request contains at most 32 family snapshots, three
deterministic representative observations per family (earliest, latest, and
highest-impact when distinct), 96 observations total, eight source-session
IDs per family, and 128 KiB of serialized input. The model response is bounded
to 64 KiB and at most eight clusters, each containing two to eight distinct
supplied family IDs, with a title of at most 240 characters and a summary of
at most 1,000 characters. The sidecar rejects unknown, empty, duplicate,
overlapping, invalid, or cross-target clusters as a whole; no partial rollup
is applied, and a rollup validation failure discards the rollups without
rejecting the rest of the combined response (see
[Rollup proposed change](#rollup-proposed-change)). Cluster and member ordering is normalized before identity is
computed, and the parent ID is `rollup:<sha256-hex>` over sorted member
recommendation IDs. A server-owned evaluation token contains the workspace
instance ID, a monotonic generation within that workspace instance, the
provider/model identity, and a hash of the supplied family snapshot; it is not
model-supplied or persisted as authority. Applying model output requires the
same workspace instance and generation, followed by locked revalidation that
every supplied family is still proposed, has the same evidence snapshot, and
has no changed active parent. Stale output is discarded without changing exact
recommendations or observations.

`RecommendationStatus` includes `rolled_up`. Rollup parents add
`rollupMemberIds`, `rollupMemberDedupeKeys`, `rollupGeneration`,
`supersedesRecommendationId`, and a bounded projection of member evidence;
rolled-up members add `rolledUpBy`. These fields default to empty lists or
`null` for legacy records. The parent projection is limited to 64 evidence
entries and 128 KiB. The sidecar derives the parent's evidence, recurrence
count, affected sessions, impact, confidence, and target surface from member
evidence; all v1 members must share one target surface. It also stores the
sorted, de-duplicated union of member source-session IDs within existing
metadata bounds.

Parent/member changes are one recoverable store transaction under the
workspace lock. Staged files, durable backups, expected old-file hashes, and
a fsynced manifest allow startup recovery to roll back an uncommitted graph or
complete a committed graph. Recommendation reads remain unavailable with a
diagnostic if neither complete graph can be established. A member belongs to
at most one active parent, every `rolled_up` member has exactly one existing
parent, and every active parent lists existing members.

The normal list returns active rollup parents and proposed exact families with
no active parent; it never returns a `rolled_up` member as an actionable card.
Detail retains parent/member relationships for audit and handoff, while an
action against a rolled-up member returns the existing invalid-transition
response. A changed proposed membership supersedes the old parent and
transactionally releases or assigns members; a same-member-set result updates
in place. Dismissed, accepted, completed, expired, failed, and other terminal
parents remain immutable history with hidden `rolled_up` members. New
qualifying evidence creates a new exact-family generation rather than
reopening a terminal graph.

### Rollup proposed change

A rollup parent's title and summary describe the problem; they do not say what
to change. Each cluster in a rollup response therefore carries one required
`proposedChange`, a model-written, validated, **non-evidence** proposal for the
receiving session to check and act on. Model response and API use the same
camelCase names:

```text
proposedChange
  summary        non-empty, at most 200 characters; the concrete change in one sentence
  targets        one to three entries, no duplicate paths
    path         repo-relative path
    action       edit | create
    instruction  non-empty, at most 160 characters; what to do in that file
    sensitive    sidecar-computed boolean, stored and exposed; never model-supplied
  verification   non-empty, at most 200 characters; how to confirm the change worked
```

The model response omits `sensitive`; a model-supplied `sensitive` is an
unknown field and fails the section. The re-serialized UTF-8 JSON of a
`proposedChange` is at most 1.5 KiB. The 64 KiB combined-response cap is
unchanged, is measured on the raw provider string, and covers the entire
response (enrichments, proposals, and rollups). Typical output fits, but
worst-case multibyte or `\uXXXX`-escaped text across proposals and eight
clusters can exceed it; that is an oversized response, which remains a
whole-response failure. The implementation plan must size real provider
responses against the cap before relying on it. `verification` is prose describing a check, never a command for
OrkWorks or the sidecar to run. All text fields, including `path`, use the same
generated-text validation as `title` and `summary` and additionally reject `<`
and `>`, so the prompt reference cleaner never alters a stored value.

The sidecar validates each target path against the canonical root of the
active workspace.
Path length is at most 260 bytes. Reject:

- control characters (including NUL), backslashes, any `:`, a leading `/`, and
  empty, `.` or `..` segments;
- a segment with a trailing dot or space, or a Windows reserved device name
  (`CON`, `PRN`, `AUX`, `NUL`, `COM1`-`COM9`, `LPT1`-`LPT9`), with or without an
  extension;
- any segment equal to `.git`, compared case-insensitively, at any depth;
- duplicate paths within one `proposedChange`, compared case-insensitively;
- a path outside the repository-level surfaces below.

A target must classify, case-insensitively, as at least one of these generic
(not repository-specific) surface classes, independent of the cluster's
`target_surface`; anything else, such as product source, is out of scope. This
mirrors the Fix prompt's existing scope of repository-level instructions,
skills, tests, tooling, and documentation:

- **instructions:** a file named `AGENTS.md`, `CLAUDE.md`, `GEMINI.md`, or
  `.cursorrules` at any depth, or `.github/copilot-instructions.md`;
- **skills:** a path under `skills/`, `.claude/skills/`, `.agents/skills/`, or
  `.codex/skills/`;
- **documentation:** a path under `docs/` or `specs/`, or a `.md`, `.mdx`,
  `.rst`, or `.txt` file at any depth;
- **tests:** a path with a `tests`, `test`, `__tests__`, or `spec` directory
  segment, or a file name matching `*_test.*`, `*_tests.*`, `*.test.*`,
  `*.spec.*`, or `test_*`;
- **tooling:** a path under `scripts/`, `.github/`, `.husky/`, `.githooks/`,
  `.devcontainer/`, `.vscode/`, `.cargo/`, `.claude/`, `.codex/`, `.opencode/`,
  `.agents/`, or `.cursor/`; a file named `Makefile`, `justfile`, or
  `Dockerfile`, or in the sensitive file-name list below, at any depth; or a
  `.json`, `.yml`, `.yaml`, or `.toml` file directly in the repository root.

An `edit` path must resolve, after canonicalization, to an existing regular
file (not a directory, device, or FIFO) inside the workspace root, so a symlink
cannot escape it. A `create` path's leaf must be reported absent by
`symlink_metadata` (a dangling symlink counts as existing), and its parent must
already exist, be a directory, and canonicalize inside the workspace root. Two
targets that resolve to the same file (two `edit` paths with one canonical file,
or two `create` paths with one canonical parent and leaf, for example through an
in-workspace symlink alias) are rejected as duplicates. After
canonicalization the sidecar derives the canonical repo-relative destination
(for `create`, the canonical parent-relative path plus the leaf) and re-applies
the `.git` segment rule, the surface-class rule, and sensitivity to it, so an
allowed-looking symlink such as `docs/guide.md` pointing into `.git/` or into
product source is rejected. `sensitive` is true when either the stated or the
canonical path is sensitive, and the card and handoff use that computed value.
An `edit` target that the platform reports as having more than one hard link is
also rejected, because a hard link hides its other path from canonicalization
(for example `docs/guide.md` hard-linked to `.git/config`). The stored `path` is
the validated stated repo-relative path, not the canonical one.

Path validation does filesystem I/O, so it runs under the workspace lock when
output is applied; an earlier parse-time pre-filter is optional. It is
advisory: the file can change before the user acts, and the
sidecar does not re-validate at handoff. The receiving session rechecks every
target, including whether an `edit` target still exists or a `create` target
has appeared, and reports the difference instead of forcing the change.

Instruction and configuration surfaces such as `AGENTS.md`, `CLAUDE.md`, and
skills are legitimate targets and are allowed. The sidecar sets
`sensitive: true`, comparing case-insensitively, on a target under `.claude/`,
`.codex/`, `.opencode/`, `.agents/`, `.cursor/`, `.vscode/`, `.devcontainer/`,
`.husky/`, `.githooks/`, `.github/`, `.cargo/`, or `scripts/`, or whose final path
segment equals `.mcp.json`, `.gitattributes`, `.gitmodules`, `opencode.json`, `apm.yml`,
`package.json`, `Cargo.toml`, `build.rs`, `Makefile`, `justfile`, or `Dockerfile`, at any depth, because those paths can run code or
change hooks, permissions, or CI. A Markdown file under `skills/`, `.claude/skills/`,
`.agents/skills/`, or `.codex/skills/` is never flagged, even beneath a flagged
directory; other files there, such as skill scripts, follow the normal rules.
`AGENTS.md`, `CLAUDE.md`, and skills are not flagged by name: the flag marks
paths that execute or change permissions, and these are the primary intended
targets. The directory rule takes precedence, so `.claude/CLAUDE.md` or
`scripts/AGENTS.md` is flagged because of the directory it sits under. The model
never sets this flag.

**Failure behavior.** Rollup failures degrade instead of rejecting the
response. The rollups section is parsed leniently and validated as a unit,
including `proposedChange`, so a missing, malformed, or oversized-cluster-count
section, a missing `proposedChange`, a model-supplied `sensitive`, an unknown
field inside a cluster, or any cluster or path validation failure (at parse or
apply time) discards the whole section with nothing partial applied. A response
that is not valid JSON as a whole, or exceeds the response cap, remains a
whole-response failure.

A degraded rollups section is not an empty clustering result. A valid empty
result is authoritative and may dissolve proposed parents; a degraded section
must instead leave every existing rollup parent and member untouched and apply
only the non-rollup updates, without graph reconciliation. Enrichments, which name a
dedupe key, are dropped when they target a rollup parent or rolled-up member so
that "untouched" holds. Proposals create new records and carry no identity of an
existing one, so they always apply, as do all other enrichments and the
deterministic exact recommendations.

The run is recorded as succeeded with a rollup-degraded diagnostic: one bounded
classified reason code, never model text, in the existing evaluation status.
Except for a filesystem-check degradation, it is cached like any successful
run, so identical inputs are not re-evaluated or re-billed until they change,
which stops a consistently bad model from burning the daily allowance. A
degradation caused by the apply-time filesystem check is not written to the
evaluation cache: the environment can change without changing the cache inputs,
and a manual analysis keeps the cache, so caching it could suppress a
now-valid rollup indefinitely. Automatic retries stay bounded by the cooldown
and daily allowance; a manual analysis remains unlimited under the existing
manual-analysis contract. The evaluation reservation is consumed either way. This
replaces the earlier rule that an invalid rollup rejected the whole combined
response.

The rollup prompt version advances to `taskmaster-rollup-v2`. Because the
version is part of the evaluation token and the provider cache key, a cached
`v1` result is a cache miss after upgrade and an in-flight `v1` request fails
the token check at apply time; neither is applied. The instruction
string and the response schema in the prompt name `proposedChange` explicitly
and describe the limits above.

`proposedChange` is presentation, not evidence. It never feeds parent identity
(the `rollup:<sha256-hex>` ID is still over sorted member IDs) and never
affects recurrence, affected sessions, impact, or confidence, which stay
derived from member observations. It is not counted inside the 64-entry,
128 KiB parent evidence projection. It is stored on the rollup parent as an
optional field next to the other rollup-only fields, not inside
`workflowImprovement`, and is written in the same recoverable parent/member
transaction. It serializes as `null` for exact cards, legacy records, and
rollups produced before `v2`. A same-member-set `v2` result sets or updates it
in place on a parent that is still `proposed`, including backfilling an
existing `v1` parent. `executing`, `accepted`, and terminal parents keep the
value they had, including `null`. The cluster's `target_surface` stays derived
from its members. It is not required to match a target path's surface class,
but every target must fall within the surface classes above.

The card labels the block a model-written hypothesis, renders every field as
plain text (no Markdown, no links), and shows a warning badge on a `sensitive`
target. The Fix with AI handoff for a rollup places the proposed change inside
the delimited untrusted reference data, included whole or omitted whole when
the reference is truncated. Outside the delimiters, it directs the receiving
session to read each target file first and confirm the change still applies, to
state why and stop when it does not, to name any file it changes outside the
listed targets, to treat `verification` as a hint rather than a command, and,
for a `sensitive` target, to tell the user before editing and never widen
permissions, hooks, or CI behavior. A rollup without a `proposedChange`
produces the existing handoff unchanged. The Rust and desktop prompt builders
are tested against one shared fixture so they cannot drift.

### Kind-to-target mapping

Every observation kind maps to a fixed target surface and recommendation-text template. The template is presentation, not evidence — every recurrence, session, impact, and confidence claim in the recommendation is computed from the cited observations.

| Observation kind | Default target | Proposed-improvement template |
| --- | --- | --- |
| `repetition` | `tooling` | Automate or remove repeated work: `<description>` |
| `obstacle` | `tooling` | Remove or document the obstacle: `<description>` |
| `missing_context` | `instructions` | Add missing repository context: `<description>` |
| `assumption` | `instructions` | Make the required assumption explicit: `<description>` |
| `correction` | `instructions` | Prevent this recurring correction: `<description>` |
| `workaround` | `tooling` | Replace the workaround with a supported path: `<description>` |
| `verification_gap` | `test` | Add reliable verification for: `<description>` |

### Recommendation shape

`improve_workflow` carries all shared recommendation fields (see "Recommendation contract" below) with `suggestedHarnessId`, `suggestedModel`, `suggestedWorkingDirectory`, and `suggestedPrompt` all `null` (the evaluator does not populate these), `targetSessionId` `null` until the recommendation is accepted (then set to the session the fix prompt was sent to), `requiresApproval: false`, and priority derived from the highest cited impact. Confidence is conservative: `high` only when every qualifying cited observation is at least `0.8`; otherwise `medium`. It adds a `workflowImprovement` object:

```text
workflowImprovement
  proposedImprovement
  targetSurface       instructions | skill | test | tooling | documentation
  observationIds
  recurrenceCount
  affectedSessionIds
  impact
  expectedBenefit
  supersedesRecommendationId null, dismissed predecessor ID, superseded rollup parent ID, or invalidated assessment predecessor ID
  dismissalWatermark null or dismissed evidence watermark
```

Each canonical `evidence` entry embeds an immutable snapshot of a cited observation (ID, sequence, session ID, kind, description, evidence text, impact, source, confidence, observed time), so ordinary observation-segment trimming cannot invalidate an existing proposed or dismissed card. A recommendation cannot claim more recurrences or sessions than its evidence contains. A proposed recommendation may be updated with later qualifying evidence while retaining its identity and lifecycle history.

For exact-family evaluation, `proposed`, `dismissed`, `executing`, `accepted`, and `completed` are reachable in this version; the remaining canonical statuses stay valid for shared deserialization but are never produced by that evaluator, except an assessment-derived exact-family recommendation may transition from `proposed` to terminal `superseded` when its captured assessment input becomes stale or effective access narrows. Such a superseded assessment recommendation cannot be accepted or executed. A later valid assessment may create one new linked successor for that superseded assessment record, with a new stable ID bound to the assessment ID and predecessor; the predecessor remains immutable. The separate rollup evaluator may also produce `superseded` when a proposed rollup's membership changes. A dismissed record remains immutable history even when its evidence later qualifies for a resurfaced successor — the successor's `supersedesRecommendationId` records the lineage, and the predecessor's status is never rewritten. `executing` is a brief reservation the `accept` action holds while it delivers the fix prompt, before resolving to `accepted` (delivered) or rolling back to `proposed` (delivery failed). An authenticated agent completion report transitions `accepted` to `completed` after verified work; a repeated completion report from the same target session is idempotent. `dismiss` accepts `executing` too, as a manual recovery path if a crash ever leaves one stuck there. For exact-family recommendations and unchanged rollup membership, `executing`, `accepted`, `completed`, and `superseded` are terminal for the evaluator: once a recommendation leaves `proposed`, it is never resurfaced or rewritten by later qualifying evidence under the same dedupe family in this version, except for the linked assessment successor rule above.

### Deduplication and dismissal watermark

The dedupe family is `improve_workflow:v1:<target-surface>:<observation-fingerprint>`; it never uses generated prose, and only one proposed member of a family may exist at a time.

Dismissal stores a `dismissalWatermark` containing `dismissedAt`, `dismissedThroughSequence`, the qualifying observation IDs and count, highest impact, and affected session IDs. The evaluator compares later durable observations against that fixed watermark without mutating or resurfacing the dismissed record unless either:

- the highest cited impact increases; or
- at least two qualifying observations have a sequence greater than `dismissedThroughSequence`, including one from a session not represented in the watermark.

When either condition holds, Taskmaster creates one new `proposed` recommendation in the same dedupe family with the dismissed ID in `supersedesRecommendationId`. The dismissed record remains immutable history. Unchanged evidence cannot create a duplicate.

### Presentation

The Taskmaster surface presents one card per active `improve_workflow` recommendation, showing the proposed improvement and target surface, why Taskmaster is suggesting it now, recurrence count and affected sessions, impact/confidence/expected benefit, expandable supporting observations (source and timestamp), and two actions: `Dismiss` and `Fix with AI` (sends a generated prompt containing the stable recommendation ID and the `working-on-recommendation` skill handoff into the user's currently active session, scoped to the recommended target surface; disabled when no session is active). This version does not create a GitHub issue or edit repository files itself — only the explicit `Fix with AI` action submits a prompt that may result in an edit, carried out by the session the user already has open. A target agent may report verified completion through the authenticated completion route; Taskmaster never infers completion from terminal text.

## Review-session handoff

When proposing `start_review_session`, Taskmaster prepares a read-only review handoff containing:

- source task description
- source session ID
- source harness/model
- implementation summary
- working directory, branch, and worktree identity
- files touched
- commands and tests run
- known risks and unresolved questions
- review objectives
- instruction not to modify code unless the user changes the mode
- expected structured review result

Example generated intent:

```text
Review the changes produced by session upload-refactor.

Focus on behavioral regressions, retry semantics, missing tests, error handling, and unnecessary complexity.

The implementation session reports that 42 tests pass. Treat that as evidence, not proof. Inspect the current working tree and report findings with severity and file references. Do not modify files.
```

The review session should normally start in the source session's working directory so it can inspect the same uncommitted changes. The UI must make this shared-directory relationship explicit. The session is read-only by instruction, not by filesystem enforcement in v1.

## Structured review result

A review Peon should normalize review output into fields such as:

```json
{
  "reviewStatus": "changes_requested",
  "summary": "Two correctness issues and one missing boundary test were found.",
  "findings": [
    {
      "severity": "high",
      "title": "Retry counter resets after transient failure",
      "file": "src/Uploads/UploadRetryService.cs",
      "line": 84,
      "recommendation": "Preserve the attempt count across retryable exceptions."
    }
  ],
  "confidence": "high"
}
```

Valid review statuses:

- `approved`
- `approved_with_notes`
- `changes_requested`
- `blocked`
- `inconclusive`

A review approval is an agent opinion. It does not equal user acceptance.

## Recommendation contract

Recommendations are persisted under:

```text
.orkworks/recommendations/<recommendation-id>.json
```

Example:

```json
{
  "id": "rec-upload-refactor-review",
  "workspaceId": "onlyclips",
  "chainId": "chain-upload-refactor",
  "chainDepth": 1,
  "type": "start_review_session",
  "status": "proposed",
  "priority": "high",
  "title": "Run an independent implementation review",
  "summary": "The implementation is complete and tests pass, but no independent review exists.",
  "reason": [
    "The source session reports review-ready work.",
    "42 tests passed.",
    "The change affects retry behavior.",
    "No review session is linked to this change."
  ],
  "evidence": [
    {
      "source": "session",
      "sessionId": "upload-refactor",
      "field": "tests.status",
      "value": "passed"
    }
  ],
  "sourceSessionIds": ["upload-refactor"],
  "targetSessionId": null,
  "suggestedHarnessId": "codex-gpt55",
  "suggestedModel": "gpt-5.5",
  "suggestedWorkingDirectory": "/Users/lars/dev/onlyclips-upload-refactor",
  "suggestedPrompt": "Review the current changes without modifying files...",
  "confidence": "high",
  "requiresApproval": true,
  "dedupeKey": "upload-refactor:start_review_session:working-tree-v3",
  "createdAt": "2026-06-19T10:00:00+02:00",
  "updatedAt": "2026-06-19T10:00:00+02:00",
  "expiresAt": null
}
```

Required recommendation fields:

- identity and workspace
- type and lifecycle status
- priority
- title and summary
- plain-language reason
- evidence
- source sessions
- suggested action details
- confidence
- approval requirement
- deduplication key
- timestamps

Rollup records additionally expose `rollupMemberIds`,
`rollupMemberDedupeKeys`, `rollupGeneration`, `rolledUpBy`, and
`supersedesRecommendationId`. For a replacement rollup, that last field links
the new parent to its superseded rollup parent. Rollup parents also expose
`proposedChange` (see [Rollup proposed change](#rollup-proposed-change)).
Legacy records deserialize these as empty lists or `null`.

## Recommendation lifecycle

Valid statuses:

- `proposed` — visible and awaiting user action
- `accepted` — approved by the user
- `executing` — linked action or session has started
- `completed` — the action reached its intended terminal state
- `dismissed` — rejected by the user
- `rolled_up` — retained as an internal exact-family record under a rollup parent
- `superseded` — replaced by newer workspace state
- `expired` — no longer relevant after a configured time or state change
- `failed` — OrkWorks could not execute the accepted action

Typical flow:

```text
proposed → accepted → executing → completed
```

A material state change may instead cause:

```text
proposed → superseded
```

Dismissed recommendations must not immediately reappear from unchanged evidence.

## Deduplication and loop prevention

Taskmaster must avoid noisy or endless agent chains.

V1 safeguards:

- one active recommendation per deduplication key
- one active review session per source change
- dismissed recommendations remain suppressed until evidence changes materially
- completed recommendations are part of future evaluation context
- child sessions record their source recommendation and parent session
- default maximum chain depth of 3
- default maximum of one independent review pass before user involvement
- a second review requires new implementation evidence, unresolved high-risk findings, or explicit user request
- conflicting review outcomes require user involvement
- Taskmaster cannot recommend a new session solely because the previous recommendation completed; new evidence is required

The maximum chain depth is configurable, but v1 must always require explicit approval for each transition.

## Model and harness selection

Taskmaster should reuse the shared recommendation engine rather than introduce a second scoring system.

Additional scoring inputs for workflow transitions:

- action intent, such as review or verification
- risk level
- independence from source model/harness
- prior attempts and failures
- context-window suitability
- capacity and reset time
- cost tier
- active-session load
- working-directory safety
- user reviewer preferences

Example review preference order:

1. Healthy strong model configured for review.
2. Healthy strong model from a different family than the implementer.
3. Healthy medium-cost review-capable model.
4. The same model family when no better option is available.
5. Wait for capacity when the user has configured that preference.

The explanation must state meaningful trade-offs, such as using the same provider because the preferred reviewer is capped.

## Optional model assistance

The accepted [brain-informed recommendation extension](taskmaster-knowledge.md)
defines the implemented direction for optional model assistance: independent
Taskmaster provider/model selection, bounded background analysis, signed shared
knowledge, repository evidence, and user-controlled context. Its specific v1
contract governs this extension; session-transition features below remain
separately staged.

The initial rules should remain deterministic.

An optional model may later help with:

- ranking multiple valid next steps
- summarizing evidence
- drafting the handoff prompt
- estimating risk from changed files and session summaries
- explaining why a recommendation is useful

Model output must be schema-validated and cannot bypass deterministic safety rules or user approval.

Taskmaster is a logical product component. It does not require a permanently running premium model or a dedicated harness process.

## Brain-guided next-step assessment

Status: proposed by [the #769 design](../docs/superpowers/specs/2026-10-07-taskmaster-brain-guided-assessment-design.md); runtime availability depends on the reviewed signed guidance from #529.

Recommendations offers a separate, user-triggered **Assess workflow** action
beside **Analyze now**. It assesses only the workspace selected by the current
OrkWorks instance, using the configured Taskmaster provider/model, current
context level and exclusions, permitted workflow observations, and the active
verified Brain knowledge bundle. It shares the existing installation-wide
single-analysis lease and manual-run admission gate. It does not change Peon's
provider, raise context access, read additional terminal replay, run repository
commands or scripts, or contact the private Brain repository.

The assessment uses relevant general Brain concepts as guidance, not as a
scorecard. It returns at most one evidence-backed next improvement, or a
no-proposal result that explains uncertainty or missing evidence. A proposal
must cite at least one current repository fact hash and at least one relevant
Brain page ID from the selected eligible bundle; reject a proposed result with
an empty page-ID list. A no-proposal result may cite no Brain pages. Bounded
excerpts establish only what they contain. Checks requiring command execution
remain unverified. Workspace and Brain content are untrusted reference data and
cannot override repository instructions, user decisions, or Taskmaster's
authority contract.

The action is unavailable when the active bundle is not eligible under the
strict privacy and integrity rules in `taskmaster-knowledge.md`, or when its
signed `capabilities` array lacks `taskmaster-assessment-v1`. The publisher
grants that capability only when the reviewed assessment entry point and
required general guidance are present. Bundle age or offline update state alone
does not make a verified eligible bundle unavailable; an unavailable feed
retains the last eligible bundle. No inference may use an unreviewed legacy
bundle as a fallback. A missing Taskmaster provider, an active analysis, an
active Brain-derived
`improve_workflow` recommendation, or insufficient current repository evidence
produces a clear inline unavailable, blocked, or no-proposal result without a
speculative recommendation.

Assessment runs, cache identity, status, and report are distinct from **Analyze
now**. A report is workspace-local and bounded to the latest result for that
workspace. It records the workspace/evidence/configuration identity, assessment
time, provider/model, Brain bundle version, relevant concept/page IDs, cited
evidence snapshots and hashes, a concise state summary, and either the single
proposed next step or a no-proposal reason. It does not store the full prompt or
uncited workspace files. Each serialized report is capped at 64 KiB. Retain one
report per workspace, replacing it after a new assessment; do not expire
reports by age. Deleting local Taskmaster data deletes the reports. If the same
current workspace, evidence, configuration, provider, and knowledge snapshot
has already been assessed, the report may be reused only as an assessment
result; an **Analyze now** result is never shown as an assessment.

The input identity and cache key cover the complete bounded request supplied
to the model, not only facts the output later cites. Include all permitted
observations, all collected repository facts (including uncited facts), the
pre-run recommendation snapshot, selected knowledge pages, context settings
and exclusions, workspace generation, prompt/schema version, provider/model,
and full bundle identity. If this assessment creates a recommendation, record
its stable ID and exclude only that exact assessment-created record when
revalidating the current recommendation snapshot. If existing deduplication
suppresses the proposal, record no derived recommendation ID and do not exclude
the pre-existing record. Other recommendation changes remain input changes and
invalidate the report. This prevents an assessment's own proposal from
invalidating its report on the first status read. Any other input change
invalidates the report, including a no-proposal result.

A credible next step enters the existing Brain-derived `improve_workflow`
recommendation path, retaining its `proactive:v1:` identity, deduplication,
dismissal, active-recommendation, acceptance, and **Fix with AI** lifecycle.
Assessment creates no second recommendation mutation or approval path. It does
not edit files, type into terminals, create or resume sessions, change
repository configuration, or publish workspace evidence or results to Brain.
Progress, provenance, outcome, and failure appear inline in Recommendations;
workspace changes discard stale display and result state. Background popups and
focus changes are not introduced.

If the existing deduplication key belongs to a proposed, dismissed, executing,
accepted, or completed recommendation, return a no-proposal result with a
duplicate-suppression reason; do not mutate or relink that record. If the
matching record is an assessment-derived recommendation terminally superseded
because its captured inputs became stale or access narrowed, a later valid
assessment may create one successor in the same `proactive:v1:` family. Give
that successor an ID derived from the dedupe key, predecessor ID, and unique
assessment ID, and set `supersedesRecommendationId` to the predecessor. Retries
of the same assessment remain idempotent; no other lifecycle state may be
bypassed by this exception.

Proposal persistence is crash-consistent with the existing recommendation
store. Before upserting a proposal, atomically persist the bounded workspace
assessment record with an internal pending-recommendation marker and the
recommendation's stable ID. Keep pending records out of status responses until
the recommendation is durably upserted and the assessment record is committed.
Under the shared analysis lease, startup recovery idempotently repeats the
upsert and commits the report before the sidecar serves status. The pending and
committed forms share the report's 64 KiB serialized cap.

If an input change or narrowed context invalidates an assessment, supersede any
still-proposed recommendation derived from it, remove it from active
recommendations, and prevent **Fix with AI** from using its stale evidence.
For assessment-derived `improve_workflow` records, `superseded` is an allowed
terminal status reached only from `proposed` when the assessment input identity
changes or access narrows; it cannot be accepted or executed afterward. Other
`improve_workflow` recommendations retain their existing lifecycle contract.
When effective access narrowing makes evidence disallowed, redact its immutable
snapshot from the report and every lifecycle record that retains it, including
proposed, superseded, dismissed, accepted, executing, and completed
recommendations. Also redact all model-generated display fields that may copy
or paraphrase repository evidence, including recommendation title, summary,
reason, proposed improvement, expected benefit, and corresponding report
summary/proposal text. Clear affected evidence excerpts and source-derived
display text rather than trying to infer which words disclose a source. Replace
required display strings with a generic access-redacted placeholder. Preserve
record identity, lifecycle transitions, dismissal decisions, and outcome
history. Other input changes invalidate the report and supersede a proposed
recommendation without redacting audit snapshots.

Before any recommendation is returned or used, revalidate an assessment-derived
proposal against its complete stored input identity. This applies to list and
get responses, active-recommendation admission results, acceptance, and the
**Fix with AI** handoff; no prior status poll is required. Suppress a stale
proposal from responses and refuse its acceptance or handoff while preserving
the report's read-only status projection semantics.

The assessment status projection is read-only. It revalidates report identity
and omits stale content, but never deletes or rewrites state. A pending
recommendation transaction is never exposed or actionable: the writer that
encounters a report-commit failure immediately attempts idempotent recovery
before returning. Read-only status and recommendation list/get routes only
check pending state; if it remains unreconciled, they omit the pending result or
return unavailable and do not write. Active-recommendation admission,
acceptance, and Fix with AI require successful reconciliation under the
analysis lease before exposing or using a pending record. If reconciliation
still fails, return an unavailable/service failure without exposing or acting
on the pending recommendation; it does not count as an active recommendation
for another assessment. Do not start new provider work while a prior pending
transaction remains unreconciled.
A context/access
settings mutation redacts newly disallowed report and recommendation snapshots
before replying. Because defaults apply across workspaces, a global context or
exclusion reduction redacts every affected workspace-local report and every
affected assessment-derived recommendation before the settings response; a
workspace override change affects only that workspace. If an affected workspace
is not open, its persisted assessment evidence must be reconciled and redacted
before any later status, recommendation, acceptance, or Fix with AI response can
expose or use it. Other stale reports are removed by the next state-changing
assessment or workspace cleanup.

## API

Proposed HTTP endpoints:

- `GET /taskmaster/recommendations` — list recommendations for the active workspace
- `GET /taskmaster/recommendations/:id` — get one recommendation
- `POST /taskmaster/recommendations/:id/accept` — approve and execute the proposed action
- `POST /taskmaster/recommendations/:id/complete` — authenticated target-agent completion report
- `POST /taskmaster/recommendations/:id/dismiss` — dismiss with an optional reason
- `POST /taskmaster/recommendations/:id/refresh` — reevaluate against current state
- `POST /taskmaster/analyze` — request an immediate Brain-backed evaluation for the selected workspace, subject to the daily allowance and the active-improvement gate
- `POST /taskmaster/assess-workflow` — request a manual Brain-guided workflow assessment for the currently selected workspace; the request has no caller-supplied workspace, provider, context override, knowledge page, or report body
- `GET /taskmaster/run-status` — return the existing run status plus a distinct read-only assessment status projection (`idle`, `queued`, `running`, or the latest `succeeded`, `failed`, or `interrupted` outcome)

`assess-workflow` is authenticated through the sidecar's existing local API boundary. Its response distinguishes `scheduled`, `unavailable`, `already_running`, and `active_recommendation`; it never returns repository excerpts in the scheduling response. The `active_recommendation` response contains only a stable recommendation ID and status, not the recommendation object or its evidence. Assessment status and report details are scoped to the selected workspace. The status projection identifies the assessment and outcome, and may return the bounded latest report only while its workspace, effective context settings, exclusions, provider/harness identity, bundle version, complete input identity, cited fact hashes, and cited knowledge page IDs still match current state; identity revalidation excludes only that report's own derived recommendation. A mismatch logically invalidates the report and any proposed recommendation derived from it before either can be returned or used. Apply this validation before every recommendation list/get, active-recommendation response, acceptance, or Fix with AI handoff, even when no status poll occurred. This read-only status request never deletes or rewrites state: context/access settings mutations redact newly disallowed evidence before replying, and other stale reports are removed by the next state-changing assessment or workspace cleanup. A global context/exclusion reduction redacts every affected workspace-local snapshot before replying; unopened workspaces are reconciled before their data can next be exposed or used.

Proposed WebSocket event:

- `taskmaster.recommendation.updated`

Workflow observations reach Taskmaster through a session-scoped sidecar route rather than a Taskmaster route directly:

- `POST /sessions/:id/workflow-observations` — authenticated, harness-neutral explicit workflow-observation report (see `specs/orkworks-mvp.md`)

`improve_workflow` recommendations use `GET /taskmaster/recommendations` (list), `POST /taskmaster/recommendations/:id/dismiss`, and `POST /taskmaster/recommendations/:id/accept` from the API above — `refresh` has no effect for this passive variant, since Taskmaster's five-second correlation debounce drives its own reevaluation. Unlike the general `accept` contract below (which starts a session), `improve_workflow`'s `accept` takes a caller-supplied `sessionId` identifying the user's currently active session and submits a generated fix prompt into it through the same mechanism as a live keystroke — it starts no session.

The `complete` action is separate from user acceptance: the agent uses its
current session's `ORKWORKS_REPORT_TOKEN` and supplies only an optional bounded
summary. The sidecar resolves the token to the session, verifies the
recommendation target and `accepted` status, and transitions it to
`completed`.

Accepting a recommendation that starts a session should use the existing session creation path. The created session records:

- `parentSessionId`
- `sourceRecommendationId`
- `coordinationRole`, such as `review`, `verification`, or `fix`
- `chainId`
- `chainDepth`

## Desktop UI

Taskmaster should appear as a Dockview panel and as a concise recommendation surface in the existing action overview.

A recommendation card shows:

- recommended next action
- source session or sessions
- evidence summary
- proposed harness/model
- cost and capacity state
- working-directory implications
- confidence
- chain depth

Primary actions:

- **Start review** / **Start verification** / action-specific approval
- **Edit instructions**
- **Change model**
- **Focus source session**
- **Dismiss**

Example:

```text
Recommended next step

Run an independent review
Codex / GPT-5.5 · healthy · premium

Why
• Implementation is reported complete
• 42 tests passed
• Retry behavior changed
• No independent review exists

The reviewer will inspect the same working directory in read-only mode.

[Start review] [Edit instructions] [Dismiss]
```

Urgent user-owned decisions should remain more prominent than optional agent follow-ups.

## Architecture

Suggested backend structure:

```text
crates/orkworksd/src/taskmaster/
├─ mod.rs
├─ evaluator.rs
├─ rules.rs
├─ model.rs
├─ store.rs
├─ handoff.rs
└─ api.rs
```

Responsibilities:

- `evaluator` gathers workspace facts and runs rules
- `rules` emits candidate intents with evidence
- shared recommendation scoring selects harness/model
- `handoff` prepares prompts and launch context
- `store` persists lifecycle and deduplication state
- `api` exposes recommendations and approval actions

Suggested frontend structure:

```text
apps/desktop/src/features/taskmaster/
├─ TaskmasterPanel.tsx
├─ RecommendationCard.tsx
├─ RecommendationDetails.tsx
└─ taskmasterApi.ts
```

Taskmaster consumes metadata watcher events. Peon does not call Taskmaster through a model-to-model protocol; Peon writes normalized state, and the backend propagates that state to the evaluator.

## Interaction with existing features

### Recommendation engine

The existing recommendation engine answers:

> Which harness/model fits this task?

Taskmaster adds:

> Given what just happened across the workspace, what task or workflow transition should happen next?

Taskmaster should produce the action intent, then delegate harness/model ranking to the shared recommendation engine.

### Session plan/spec review

The repo-wide Review Queue proposal (`specs/review-queue.md`) is superseded by
`specs/session-plan-review.md`; the artifact-inbox design is not in product
scope. Session-owned plan/spec review — surfaced through the selected-session
Review tab and explicit review-prompt handoff — is the mechanism Taskmaster may
observe evidence from, but Taskmaster remains a workflow recommendation engine:

- Session plan/spec review: session-owned artifact review
- Taskmaster: workflow recommendation engine

### Right-side action overview

The action overview continues to answer what needs attention now. Taskmaster recommendations are one category of actionable item, ordered below direct user blockers and above routine working-session information.

## Rollout

### Phase 1 — Contract and persistence

- recommendation schema
- lifecycle store
- source/parent session links
- deduplication keys
- API read and dismiss support

### Phase 2 — Deterministic evaluator

- metadata/event triggers
- independent-review rule
- verification rule
- user-decision rule
- stronger-model escalation rule
- loop guards

### Phase 3 — Desktop panel

- Taskmaster Dockview panel
- recommendation cards
- evidence details
- dismiss and refresh
- **Analyze now** for Brain-backed recommendations; bypass the workspace cooldown and the daily analysis allowance, and direct the user to an outstanding Brain recommendation before another analysis can run

### Phase 4 — Approved session launch

- one-click accept
- edit prompt/model before launch
- start through existing session creation path
- link child session to recommendation chain
- lifecycle updates

### Phase 5 — Review and fix chains

- structured review result
- resume-source and start-fix recommendations
- completion/supersession handling
- ready-for-user-acceptance state

### Phase 6 — Optional model enrichment

- handoff drafting
- evidence summarization
- candidate ranking
- risk explanation

## Acceptance criteria

- [ ] A Peon transition to review-ready state can produce one `start_review_session` recommendation.
- [ ] The recommendation explains which session evidence triggered it.
- [ ] A healthy review-capable harness/model is suggested using shared recommendation scoring.
- [ ] No session starts until the user explicitly accepts the recommendation.
- [ ] The user can edit the prompt and change the model before starting.
- [ ] The review session is linked to the source session, recommendation, chain, and coordination role.
- [ ] The same unchanged evidence does not create duplicate recommendations.
- [ ] Dismissing a recommendation suppresses it until material evidence changes.
- [ ] Review findings can produce a fix or resume-source recommendation.
- [ ] A completed review with no unresolved findings surfaces work as ready for user acceptance, not automatically accepted.
- [ ] Product decisions, credentials, destructive actions, and merge approval are always routed to the user.
- [ ] Chain depth and review-count limits prevent indefinite session spawning.
- [ ] Capacity changes can supersede or rerank a proposed recommendation.
- [ ] Taskmaster never writes terminal input, modifies source files, or performs Git workflow actions directly, except through the user-confirmed `improve_workflow` `accept` action, which submits a generated prompt into the user's own active session (never a session Taskmaster chose or started) and never edits files itself.
- [ ] Two sessions that each produce a matching workflow observation (same fingerprint, confidence ≥ `0.6`) can produce one evidence-backed `improve_workflow` recommendation citing both.
- [ ] `improve_workflow` recommendations expose exactly one explicit `accept` action (no automatic/background execution, and it never starts a new session) and reach only `proposed`, `dismissed`, `executing`, `accepted`, or `completed` status, except an assessment-derived recommendation may transition from `proposed` to terminal `superseded` when its input identity changes or access narrows; a superseded recommendation cannot be accepted or executed.
- [ ] A manual Brain analysis bypasses the per-workspace cooldown and the daily allowance (manual analyses are unlimited; the allowance governs automatic background discovery only), and is refused while a Brain-derived `improve_workflow` recommendation is proposed, accepted, or executing; deterministic observation-only recommendations do not block it, and the user is directed to the existing Fix with AI handoff for a Brain recommendation.
- [ ] **Assess workflow** is a distinct manual run and result from **Analyze now**, uses the selected workspace's existing Taskmaster provider/context/evidence and shared single-analysis lease, returns at most one grounded improvement or a clear no-proposal result, and is unavailable until an eligible verified bundle carries matching signed `privacyPolicyVersion: 1` and `taskmaster-assessment-v1` capability fields and the reviewed assessment guidance required by #529.
- [ ] Assessment proposals cite at least one current repository fact hash and at least one relevant Brain page ID, validate every citation and captured identity before every list/get/active-response/acceptance/Fix with AI use path, and reuse the existing `proactive:v1:` `improve_workflow` recommendation lifecycle without adding execution authority. A valid assessment may create a linked successor only for a matching assessment-derived recommendation terminally superseded by input invalidation, with an idempotent ID bound to that assessment and predecessor.
- [ ] An assessment proposal superseded after input invalidation can later receive one linked generation-aware successor; dismissed, accepted, completed, and other non-invalidated records continue to suppress duplicate proposals.
- [ ] Effective-access narrowing redacts cited snapshots and model-generated assessment text that may copy or paraphrase repository evidence from reports and recommendations in every lifecycle state, while preserving identity, transitions, dismissal decisions, and outcomes.
- [ ] A pending assessment recommendation from a failed report commit is hidden and non-actionable across status, list/get, active-recommendation, accept, and Fix with AI paths until reconciliation succeeds; it does not count as an active recommendation, and new provider work waits for reconciliation.
- [ ] One bounded local assessment report per workspace records its evidence/configuration/provider/knowledge identity and outcome, is removed with that workspace's local data, never stores the full prompt or uncited files, and cannot be populated from an **Analyze now** result.
- [ ] Assessment progress, provenance, outcome, and failures are shown inline in Recommendations; workspace/configuration changes discard stale results, and no background popup or focus change is introduced.
- [ ] A Fix with AI prompt contains the stable recommendation ID and directs the target agent to use the `working-on-recommendation` skill to read the recommendation and its source-session evidence.
- [ ] An authenticated target agent can transition an accepted `improve_workflow` recommendation to `completed`; the callback cannot name a different target session or lifecycle state, and retries are idempotent.
- [ ] Dismissing an `improve_workflow` recommendation persists an evidence watermark and does not resurface it from unchanged evidence.
- [ ] A rollup parent stores a validated `proposedChange` (summary, one to three `edit`/`create` targets, verification) of at most 1.5 KiB; a missing or invalid one, a model-supplied `sensitive`, an escaping, `.git`, Windows-reserved, out-of-scope, or duplicate (lexical or canonical) path, a missing or non-regular `edit` target, or an existing or dangling-symlink `create` target degrades the rollups section while enrichments, proposals, and exact recommendations are still applied.
- [ ] A degraded rollups section leaves every existing rollup parent and member untouched (it is not treated as an authoritative empty clustering result, and enrichments targeting a rollup parent or member are dropped; proposals always apply), is recorded as a succeeded run with a classified rollup-degraded reason and no model text, and is cached for identical inputs except when caused by the apply-time filesystem check, which is not cached; a response that is invalid JSON or over the cap still fails as a whole.
- [ ] `proposedChange` never alters rollup identity, recurrence, affected sessions, impact, or confidence; a same-member-set `v2` result backfills a still-`proposed` parent, and `executing`, `accepted`, and terminal parents keep the value they had.
- [ ] The sidecar, never the model, sets `sensitive` on targets under `.claude/`, `.codex/`, `.opencode/`, `.github/workflows/`, `opencode.json`, or `apm.yml`; the card badges them and the Fix prompt tells the session to tell the user before editing them.
- [ ] A target whose canonical destination lands under `.git/`, out of scope, or in a sensitive path (for example a `docs/` symlink into `.git/` or product source) is rejected or flagged using the canonical path, and a Markdown file under a skills subtree is never flagged sensitive even beneath `.claude/`.
- [ ] A rollup Fix with AI prompt places `proposedChange` whole inside the delimited untrusted reference data (or omits it whole) and tells the session to recheck each target first; a rollup without one produces the unchanged prompt, and the Rust and desktop prompt builders pass the same shared fixture.

## Non-goals reaffirmed

Taskmaster does not turn OrkWorks into an autonomous coding harness, task-board product, Git workflow manager, or automatic merge system.

It extends the core OrkWorks principle:

> Peons tell OrkWorks what is happening. Taskmaster recommends what should happen next. The user decides whether it happens.
