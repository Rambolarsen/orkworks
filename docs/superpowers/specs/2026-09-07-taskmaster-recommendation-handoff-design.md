# Taskmaster Recommendation Handoff

Status: accepted
Date: 2026-09-07

## Goal

Make `Fix with AI` a traceable, agent-readable handoff. The current live
session remains the execution target, while the agent can resolve the exact
Taskmaster recommendation, inspect its evidence and source sessions, and
report verified completion back to the sidecar.

## Current gap

`improve_workflow` acceptance currently sends editable prompt text to the
desktop's active live session. The sidecar records only a generic fix-request
summary and transitions the recommendation from `executing` to `accepted` as
soon as PTY delivery succeeds. The prompt does not carry the recommendation
identity or explain how the agent can retrieve it, and there is no
session-scoped completion callback.

Every persisted recommendation already has a stable `id`; this design reuses
that identity and does not require an ID backfill migration.

## Design

### Recommendation-aware prompt

The generated Fix with AI prompt includes:

- the stable recommendation ID;
- the local sidecar address from `ORKWORKS_PORT`;
- the current session identity from `ORKWORKS_SESSION_ID`;
- the instruction to load `GET /taskmaster/recommendations/:id` before acting;
- the instruction to inspect the recommendation's target surface, evidence,
  and `sourceSessionIds` rather than treating the prose prompt as the source
  of truth;
- the instruction to use the repository skill
  `skills/working-on-recommendation/SKILL.md`;
- the instruction to call the authenticated completion route only after the
  change has been made and verified.

The sidecar-generated prompt and the desktop's editable draft remain
semantically synchronized. The editable prompt may contain user changes, but
the completion contract remains scoped to the recommendation ID captured by
the accept request.

### Authenticated completion

Add:

```text
POST /taskmaster/recommendations/:id/complete
Authorization: Bearer <ORKWORKS_REPORT_TOKEN>
Content-Type: application/json
```

The request body contains only an optional bounded `summary` string. The
sidecar derives the caller's session from the reporting capability, verifies
that the recommendation exists, is an `improve_workflow` recommendation in
the active workspace, is currently `accepted`, and has the same
`targetSessionId`. It then transitions the recommendation to the existing
`completed` status and appends a completion event to the target session.

The route is idempotent for an already completed recommendation when the
capability still identifies the same target session; it rejects mismatched
sessions and all other lifecycle states. It never accepts a caller-supplied
session ID, recommendation content, target session, or lifecycle status.

If the agent cannot verify the work or cannot complete the callback, the
recommendation remains `accepted`, meaning the fix prompt was delivered but
completion was not confirmed.

### Session provenance

The existing `taskmaster_fix_requested` and new completion events carry the
recommendation ID as structured event data as well as human-readable summary
text. The summary-log response exposes that ID so the desktop can render a
recommendation link from session history back to the Taskmaster card. Existing
events without the field remain readable.

### Repository skill

Create `skills/working-on-recommendation/SKILL.md`. It applies when work is
started from a Taskmaster recommendation or a prompt contains a recommendation
ID. It teaches agents to:

1. resolve the recommendation through the local sidecar;
2. inspect target surface, evidence, and originating sessions;
3. keep edits within the recommendation's scope and repository constraints;
4. verify the resulting change;
5. call the authenticated completion route with a concise summary only after
   verification succeeds.

The generated prompt explicitly invokes the skill. The repository skill list
and relevant agent documentation link to it so harnesses can discover it
outside the generated prompt too.

### Lifecycle and compatibility

`accepted` means the user-approved fix prompt was delivered. `completed`
means the target agent explicitly reported verified completion. The evaluator
must treat both as terminal for the same recommendation family, so completed
recommendations do not resurface from unchanged evidence.

Persisted recommendations already contain `id`, `targetSessionId`, and status
fields. Deserialization keeps new optional event data backward compatible;
there is no destructive migration and no attempt to infer completion for old
accepted recommendations.

## Verification

- Rust unit and handler tests cover token authentication, target-session
  matching, accepted-to-completed transition, idempotency, invalid lifecycle
  states, event provenance, and backward-compatible event decoding.
- Desktop tests cover recommendation ID and skill instructions in the draft
  prompt, API route construction, and recommendation-link rendering.
- The skill is validated with the repository skill validator.
- Full desktop type-check/tests and Rust format/build/tests run before handoff.

## Non-goals

- Do not create a new session for Fix with AI.
- Do not allow the agent to edit recommendation files directly.
- Do not infer completion solely from terminal output.
- Do not add arbitrary terminal-input or Git workflow authority.
