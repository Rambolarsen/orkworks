# improve_workflow gains an explicit accept action that sends a fix prompt to the active session

- Status: accepted
- Deciders: Rambolarsen
- Date: 2026-09-06

## Context

ADR 0042's "Embedded recommendation evidence" decision records the passive
`improve_workflow` recommendation variant as having `requiresApproval: false`,
"no accept/execute action," and its Consequences section states these
recommendations are "dismissible only, never auto-applied, and never create a
GitHub issue or start a session on their own." `specs/taskmaster.md` echoes
this: `improve_workflow` "never starts, resumes, or focuses a session; it
exposes no accept/execute action... It is a proposal for the user to act on
manually." ADR 0045 restates that only `proposed` and `dismissed` are
reachable statuses for this variant.

Recommendations were populated but the user had no way to act on one beyond
Dismiss. The request was for an explicit "Fix with AI" action — but two
constraints shaped what that could mean:

1. Acting on a recommendation must not mean returning to the historical
   session(s) that surfaced the evidence (`sourceSessionIds` /
   `affectedSessionIds`) — those are closed; the point is to stop *future*
   sessions from hitting the same friction, by implementing the
   recommendation's proposed improvement against its `targetSurface`
   (instructions, a skill, a test, tooling, or docs).
2. The fix should be carried out by the session the user already has open and
   is actively using, not by spawning a new one.

ADR 0045's entire decision was a single, narrow correction: "for the passive
`improve_workflow` variant, only `proposed` and `dismissed` are reachable; no
evaluator transition ever writes `superseded`." This ADR directly contradicts
that sentence by making `accepted` (and transiently `executing`) reachable
too, so per the repository's decision-tracking policy ADR 0045 is marked
`superseded` by this ADR rather than merely amended — its other content (the
dismissal-immutability and `supersedesRecommendationId` lineage rules) was
already established by ADR 0042 and remains unaffected and authoritative.

OrkWorks already has a production code path for exactly this shape of action:
`SessionApplication::request_plan_review` builds a backend-generated prompt
string (ending in `\r`) and submits it into a live session's PTY via
`terminal_runtime::submit_approved_input`, which is the same transport live
xterm.js keystrokes use (`send_runtime_input` → `RuntimeCommand::Input` →
`writer.write_all`). No new session is created; the request-plan-review action
only checks the target session's `lifecycle == "alive"` before submitting.

## Decision

`improve_workflow` recommendations gain one explicit, user-confirmed `accept`
action: `POST /taskmaster/recommendations/:id/accept`, taking a caller-supplied
`sessionId` (the frontend's current `activeSessionId` — the backend's
persisted "last active session" can lag by one round-trip and must not be
treated as authoritative) and an optional prompt override. The backend
validates the recommendation is `improve_workflow`/`proposed` and the target
session is `lifecycle == "alive"`, builds a fix-scoped prompt from
`workflowImprovement.proposedImprovement`/`targetSurface`/`reason` when no
override is given, and submits it through the existing
`submit_approved_input` path — the same mechanism `request_plan_review`
already uses, not a new capability. The generated prompt explicitly instructs
the agent to scope its work to the repository's target surface and not to
resume, reopen, or modify any other session.

The recommendation is reserved before the PTY write, not after: the store
transitions `proposed` → `executing` synchronously, within the same lock
acquisition as the eligibility checks, before `submit_approved_input` is
awaited. This closes a race where two concurrent accept requests for the same
recommendation could otherwise both observe `proposed`, both write to the
session's terminal, and only one would win the later store transition — the
second request now loses at the reservation step instead, before any write.
On success, `executing` transitions to `accepted` and `targetSessionId` is
set to the session the prompt was sent to; on a delivery failure, it rolls
back to `proposed` so the user can retry. If the sidecar crashes between
reservation and either outcome, the recommendation is left stuck at
`executing` — `dismiss` accepts `executing` as well as `proposed` specifically
as the manual escape hatch for that rare case, since the evaluator (below)
otherwise never revisits it.

`evaluate_workflow_improvements` treats `accepted` and `executing` (and every
other non-`proposed`/non-`dismissed` status) as terminal for that dedupe
family: a recommendation is never resurfaced or overwritten by re-evaluation
once it leaves `proposed`. (This closes a latent gap in the evaluator that
predates this decision — before now, nothing had ever produced a
non-`proposed`/`dismissed` status, so the evaluator's lack of a branch for it
had never been exercised.)

The desktop UI's "Fix with AI" button is disabled unless the currently active
session's `lifecycle` is `alive` — matching the backend's own eligibility
check — not merely whenever a session happens to be selected, since a dead
session can remain selected after it exits.

This decision supersedes the "no accept/execute action" and "dismissible
only" clauses of ADR 0042's "Embedded recommendation evidence" bullet and
Consequences section, and supersedes ADR 0045 in full (see Context above). It
does **not** supersede or contradict ADR 0042's "never... start a session on
their own" clause — no session is created, resumed, or reconfigured by this
action; a generated string is submitted into an already-running session
exactly as a keystroke would be. It likewise does not touch ADR 0042's "never
create a GitHub issue" statement.

The remaining 11 recommendation types defined in `specs/taskmaster.md`
(`start_review_session`, `start_fix_session`, etc.) are unimplemented and
untouched by this decision.

## Consequences

- `improve_workflow` recommendations now show two actions in the desktop UI:
  `Dismiss` and `Fix with AI`. `Fix with AI` is disabled unless the active
  session's `lifecycle` is `alive`.
- No new `SessionMetadata` fields or session-creation code paths are needed;
  the only new backend surface is `RecommendationStore::begin_execution` /
  `complete_execution` / `cancel_execution`, `taskmaster::build_fix_prompt`,
  `SessionApplication::accept_recommendation`, and the new HTTP route.
- The target session is chosen by the user's current focus at the moment they
  click "Fix with AI," not by any evidence/affected-session linkage — the
  recommendation's own evidence sessions are never written to.
- No idle/attention gate is added beyond the existing `lifecycle == "alive"`
  check, consistent with `request_plan_review`'s existing behavior; if a
  session's harness is mid-task when the prompt arrives, the injected text is
  delivered exactly as a keystroke typed at that moment would be — this is an
  accepted, pre-existing characteristic of `submit_approved_input`, not new
  behavior introduced here.
- `executing` is a genuinely reachable, if normally transient, status —
  reviewers of future recommendation-lifecycle changes should account for it
  alongside `proposed`, `dismissed`, and `accepted`.
- Future reviewers should read ADR 0042's "no accept/execute action" and
  "dismissible only" language as amended by this ADR, and ADR 0045 as fully
  superseded by it.
