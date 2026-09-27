---
name: working-on-recommendation
description: Use when work starts from an OrkWorks Taskmaster recommendation or a prompt contains a recommendation ID.
---

# Working on a Taskmaster recommendation

Treat the sidecar recommendation as the source of truth for the task. The
prompt is only a handoff pointer and may be edited by the user.

1. Read the recommendation ID from the prompt and set it as
   `RECOMMENDATION_ID`. Fetch the authoritative record:

   ```bash
   curl --fail-with-body "http://127.0.0.1:${ORKWORKS_PORT}/taskmaster/recommendations/${RECOMMENDATION_ID}"
   ```

2. Inspect `workflowImprovement.targetSurface`,
   `workflowImprovement.proposedImprovement`, `evidence`, and
   `sourceSessionIds` before editing. Source sessions are evidence context;
   they are not permission to resume, reopen, or modify those sessions.
3. Follow the repository's normal branch, testing, and review instructions.
   Keep changes within the recommendation's target surface and never edit
   recommendation files directly.
4. Run focused verification, then the desktop/Rust checks required by the
   repository for the change.
5. Only after verification succeeds, report completion with a concise summary:

   ```bash
   curl --fail-with-body -X POST \
     "http://127.0.0.1:${ORKWORKS_PORT}/taskmaster/recommendations/${RECOMMENDATION_ID}/complete" \
     -H "Authorization: Bearer ${ORKWORKS_REPORT_TOKEN}" \
     -H "Content-Type: application/json" \
     --data '{"summary":"Describe the verified change and checks run."}'
   ```

   The completion request has no caller-supplied session ID. The sidecar
   derives the target session from `ORKWORKS_REPORT_TOKEN`.
6. If the change is not verified or the completion callback fails, explain the
   failure and do not claim that the recommendation is completed.

## Tie-off rule for work that lands a recommendation indirectly

This skill applies when work *starts from* a recommendation. The same
completion duty applies when your change implements an existing
recommendation's improvement from any other starting point (an issue, a code
review finding, or general maintenance): after your change is verified and
before the PR reaches a terminal state, tie off that recommendation and
reference its ID in the PR body. The completion endpoint is session-bound
(`targetSessionId` must match `ORKWORKS_REPORT_TOKEN`'s session and the
recommendation must be `Accepted`), so indirect work must first locate the
matching active record and accept it from this session:

```bash
# 1. Find the matching active recommendation (none matches → nothing to tie off)
curl --fail-with-body "http://127.0.0.1:${ORKWORKS_PORT}/taskmaster/recommendations"
# 2. Accept it from this session
curl --fail-with-body -X POST \
  "http://127.0.0.1:${ORKWORKS_PORT}/taskmaster/recommendations/${RECOMMENDATION_ID}/accept" \
  -H "Content-Type: application/json" \
  --data "{\"session_id\":\"${ORKWORKS_SESSION_ID}\"}"
# 3. Complete it (step 5 above)
```

If the recommendation carries a `completionPacket`, the accept and complete
requests must also include a `packetMutation` with the packet's current
`revision`, `evidenceFingerprint`, and `approval.idempotencyKey` from the
same `GET` payload. See
[Taskmaster recommendation tie-off](../../docs/agents/development-workflow.md#taskmaster-recommendation-tie-off).
