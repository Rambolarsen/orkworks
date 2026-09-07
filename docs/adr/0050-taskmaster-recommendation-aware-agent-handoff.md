# Taskmaster recommendation-aware agent handoff

- Status: accepted
- Deciders: OrkWorks maintainers
- Date: 2026-09-07

## Context

The `improve_workflow` Fix with AI action sends a prompt into the user's
currently active session, but the prompt does not identify the originating
recommendation. The session history records only generic prose, so an agent
cannot reliably retrieve the immutable evidence and source-session context.
The sidecar also marks the recommendation `accepted` immediately after PTY
delivery and has no trusted way for the agent to report verified completion.

The agent already receives a per-session reporting capability through
`ORKWORKS_REPORT_TOKEN`, alongside `ORKWORKS_PORT` and
`ORKWORKS_SESSION_ID`.

## Decision

Use the existing recommendation ID as the handoff identity. Include it in the
Fix with AI prompt and direct the agent to retrieve the recommendation through
the local sidecar. Add a session-capability-authenticated completion endpoint
that derives the caller's session from the token, verifies it matches the
recommendation target, and transitions `accepted` to the existing `completed`
status after the agent reports a verified result.

Record the recommendation ID in structured session events and expose it to the
desktop so task history can link back to the recommendation. Teach the agent
workflow through a committed `working-on-recommendation` repo skill, explicitly
invoked by the generated prompt.

## Consequences

- Recommendation evidence and originating sessions remain available to the
  target agent without giving it direct metadata-store access.
- Completion is explicit and authenticated; missing or failed callbacks leave
  the recommendation visibly accepted rather than falsely completed.
- The sidecar owns the lifecycle and provenance invariants, while the skill
  provides the agent-facing procedure.
- Existing persisted recommendation IDs and accepted records remain readable;
  old accepted work cannot be retroactively marked completed without a new
  callback.
- The local protocol and accepted Taskmaster spec gain a new completion route,
  event field, lifecycle transition, and discoverable repository skill.
