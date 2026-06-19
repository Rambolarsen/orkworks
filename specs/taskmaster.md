# Taskmaster — Cross-Session Coordination Spec

Status: proposed  
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

A repo Peon may summarize repo-level artifacts and signals, but it does not coordinate sessions.

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
- create or clean up Git worktrees
- send review findings into a running terminal automatically
- keep chaining sessions indefinitely

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

## Recommendation lifecycle

Valid statuses:

- `proposed` — visible and awaiting user action
- `accepted` — approved by the user
- `executing` — linked action or session has started
- `completed` — the action reached its intended terminal state
- `dismissed` — rejected by the user
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

The initial rules should remain deterministic.

An optional model may later help with:

- ranking multiple valid next steps
- summarizing evidence
- drafting the handoff prompt
- estimating risk from changed files and session summaries
- explaining why a recommendation is useful

Model output must be schema-validated and cannot bypass deterministic safety rules or user approval.

Taskmaster is a logical product component. It does not require a permanently running premium model or a dedicated harness process.

## API

Proposed HTTP endpoints:

- `GET /taskmaster/recommendations` — list recommendations for the active workspace
- `GET /taskmaster/recommendations/:id` — get one recommendation
- `POST /taskmaster/recommendations/:id/accept` — approve and execute the proposed action
- `POST /taskmaster/recommendations/:id/dismiss` — dismiss with an optional reason
- `POST /taskmaster/recommendations/:id/refresh` — reevaluate against current state

Proposed WebSocket event:

- `taskmaster.recommendation.updated`

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

### Review Queue

The Review Queue surfaces plan and spec artifacts for the user to read. Taskmaster coordinates session transitions.

They may share UI patterns and repo-level Peon infrastructure, but their responsibilities remain distinct:

- Review Queue: artifact inbox
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
- [ ] Taskmaster never writes terminal input, modifies source files, or performs Git workflow actions directly.

## Non-goals reaffirmed

Taskmaster does not turn OrkWorks into an autonomous coding harness, task-board product, Git workflow manager, or automatic merge system.

It extends the core OrkWorks principle:

> Peons tell OrkWorks what is happening. Taskmaster recommends what should happen next. The user decides whether it happens.
