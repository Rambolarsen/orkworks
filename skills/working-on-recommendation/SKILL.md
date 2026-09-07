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
