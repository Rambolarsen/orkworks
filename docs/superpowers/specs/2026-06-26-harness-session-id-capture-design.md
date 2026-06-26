# Harness Session ID Capture Design

## Context

OrkWorks currently distinguishes its own session ID from a harness-native session ID:

- `SessionInfo.id` is the OrkWorks UUID used for metadata files, PTY state, and UI selection.
- `harnessId` identifies the coding tool adapter, such as `opencode`, `claude-code`, or `codex`.
- `resume.harnessSessionId` is the underlying harness session/thread ID used for exact resume.

The existing Peon path can infer `harnessSessionId` from terminal output, but that is probabilistic. Several harnesses expose the native session ID through more reliable mechanisms:

- OpenCode exposes `OPENCODE_SESSION_ID` to executed shell commands.
- Claude Code hooks can provide the native session ID in hook JSON.
- Codex non-interactive mode emits a `thread.started` JSONL event with `thread_id` when run with `codex exec --json`, and supports `codex exec resume <SESSION_ID>`.

OrkWorks needs a generic capture contract so harness-specific mechanisms can all write the same normalized metadata without coupling the backend to one harness.

## Goals

- Add one generic backend endpoint for reporting harness-native session IDs.
- Prefer deterministic harness-owned sources over Peon inference.
- Support OpenCode, Claude Code, and Codex in the first implementation.
- Preserve exact resume behavior through the existing `resume.harnessSessionId` field.
- Track source, confidence, and capture time so later writers cannot casually downgrade reliable metadata.

## Non-Goals

- Do not silently type interactive commands such as `/status` into a user session.
- Do not auto-install Claude Code hooks. Hook installation remains explicit and user-confirmed.
- Do not read undocumented private harness databases.
- Do not remove Peon inference; it remains a low-confidence fallback.
- Do not solve every harness in this slice.

## API

Add:

```http
POST /sessions/:id/harness-session
```

Request body:

```json
{
  "harnessSessionId": "native-session-id",
  "source": "opencode_env",
  "confidence": 0.98
}
```

Validation:

- `:id` must be a known OrkWorks session ID in the live session map or current workspace metadata.
- `harnessSessionId` must be non-empty, have no whitespace, and stay below a conservative maximum length.
- `source` must be non-empty and should be one of the known source strings for first-party writers.
- `confidence` must be between `0.0` and `1.0`.

Response:

- `200 OK` when the value is accepted or an equal/higher-confidence duplicate is already stored.
- `400 Bad Request` for invalid request shape or invalid session ID value.
- `404 Not Found` when the OrkWorks session ID is unknown.
- `409 Conflict` when no workspace is open.

## Metadata

Extend session metadata with capture metadata near `resume`:

```json
{
  "resume": {
    "state": "available",
    "preferredStrategy": "exact",
    "harnessSessionId": "native-session-id",
    "latestFallback": true,
    "lastSeenAt": "2026-06-26T12:00:00Z"
  },
  "harnessSessionIdSource": "opencode_env",
  "harnessSessionIdConfidence": 0.98,
  "harnessSessionIdCapturedAt": "2026-06-26T12:00:00Z"
}
```

Merge rules:

- If no stored native ID exists, accept the new value.
- If the same native ID is already stored, update source metadata only when the new confidence is at least the stored confidence.
- If a different native ID is stored, overwrite only when the new confidence is greater than or equal to the stored confidence.
- Peon may write only when there is no higher-confidence value.
- On accepted write, ensure `resume.state = "available"`, set `resume.preferredStrategy = "exact"` when it is currently `none`, preserve `latestFallback`, and update `resume.lastSeenAt`.

Append an event:

```json
{
  "eventType": "session.harness_session_captured",
  "status": "<current session status>"
}
```

The event payload can remain minimal in this slice; the session metadata file is the source of truth for source/confidence details.

## Capture Sources

Use adapter-specific capture mechanisms, normalized through the generic endpoint.

| Harness | Source | Mechanism | Confidence | Automatic |
| --- | --- | --- | --- | --- |
| OpenCode | `opencode_env` | Report `$OPENCODE_SESSION_ID` from an OpenCode-executed shell/script path | `0.98` | Yes |
| Claude Code | `claude_hook` | Parse hook JSON `session_id` in the explicit Claude hook command and POST it | `0.98` | Yes, after hook install |
| Codex exec | `codex_exec_json` | Capture `thread.started.thread_id` from `codex exec --json` | `0.98` | Yes for exec-mode launches |
| Codex interactive | `codex_status` | User-triggered `/status` probe parsed by a Codex-specific parser | `0.90` | No |
| Any | `peon` | Existing Peon inference from terminal output | existing confidence, capped below deterministic sources | Yes |

## OpenCode Flow

When OrkWorks launches an OpenCode session, it already injects `ORKWORKS_SESSION_ID` and `ORKWORKS_PORT` into the PTY environment. The OpenCode capture path should run only inside OpenCode, where `OPENCODE_SESSION_ID` exists.

Reporter shape:

```bash
[ -n "$ORKWORKS_SESSION_ID" ] &&
[ -n "$ORKWORKS_PORT" ] &&
[ -n "$OPENCODE_SESSION_ID" ] &&
curl -sS -X POST "http://127.0.0.1:$ORKWORKS_PORT/sessions/$ORKWORKS_SESSION_ID/harness-session" \
  -H "Content-Type: application/json" \
  -d "{\"harnessSessionId\":\"$OPENCODE_SESSION_ID\",\"source\":\"opencode_env\",\"confidence\":0.98}"
```

The implementation should use the smallest OpenCode-specific reporter path that can run with `OPENCODE_SESSION_ID` present. The important boundary is that the value comes from OpenCode's own environment, not terminal-output inference.

## Claude Code Flow

The existing Claude Code attention-signal design already uses explicit, user-confirmed installation into `.claude/settings.local.json` and a hook command guarded by `ORKWORKS_SESSION_ID` / `ORKWORKS_PORT`.

Extend that hook command to:

1. Read hook JSON from stdin.
2. Extract `session_id`.
3. POST it to `/sessions/:id/harness-session` with `source = "claude_hook"` and `confidence = 0.98`.
4. Continue posting the attention signal as already designed.

If the hook payload lacks `session_id`, the hook silently skips the harness-session write and still reports attention.

## Codex Flow

For `codex exec` launches, OrkWorks should prefer JSONL capture:

```bash
codex exec --json "<prompt>"
```

The sidecar reads stdout JSONL, captures the first event matching:

```json
{
  "type": "thread.started",
  "thread_id": "..."
}
```

Then it posts or directly merges that value as:

```json
{
  "harnessSessionId": "<thread_id>",
  "source": "codex_exec_json",
  "confidence": 0.98
}
```

For interactive Codex TUI launches, OrkWorks must not silently type `/status`. The first implementation should expose a user-triggered "Capture session ID" or "Refresh harness status" action later. That action can send `/status`, parse the next output with a Codex-specific parser, and report `source = "codex_status"`.

## New Harness Checklist Skill

Adding a new harness should require a repo skill that walks the implementer through the full adapter contract before code changes. The skill should cover at least:

- launch command and model argument behavior
- exact resume support and command shape
- latest-session fallback semantics, if documented
- native session ID capture sources, ordered by reliability
- hook, env var, JSONL, status command, and deterministic output options
- whether any capture path requires user approval because it types into the session or writes local harness config
- provider/model detection and whether OrkWorks should preserve or infer those fields
- native voice support and pass-through boundaries
- capacity/context/status signals the harness exposes
- tests and docs that must be updated for the new adapter

This skill should make "how do we get the harness-native session ID?" a required question for every new harness. If no reliable source exists, the adapter must state that explicitly and fall back to manual entry, user-triggered status capture, or Peon inference in that order.

## Error Handling

- Invalid reports are rejected without changing metadata.
- Reports for unknown OrkWorks sessions return `404`.
- Reports received after a session is killed or ended may still update metadata if the metadata file exists; this allows late hook delivery.
- Lower-confidence conflicting reports are ignored rather than failing the request. This keeps duplicate or delayed low-confidence writers harmless.
- Hook/report scripts should silently no-op when required env vars are absent so they are safe outside OrkWorks.

## Testing

Rust tests:

- Valid harness-session report writes `resume.harnessSessionId` and capture metadata.
- Unknown session ID returns `404`.
- Invalid native ID returns `400`.
- Equal or higher confidence overwrites conflicting value.
- Lower confidence does not overwrite.
- Peon inference does not overwrite a higher-confidence value.

Adapter/capture tests:

- OpenCode reporter command includes `OPENCODE_SESSION_ID`, `ORKWORKS_SESSION_ID`, and `ORKWORKS_PORT`.
- Claude hook installer preserves existing settings and adds the harness-session report without duplicate entries.
- Codex JSONL parser extracts `thread.started.thread_id`.
- Codex interactive status capture remains user-triggered only.

Frontend tests can wait until the user-facing capture action is added. The backend endpoint and first automatic capture paths are sufficient for this slice.

## Documentation

Update the architecture/API docs when implemented to include:

- `POST /sessions/:id/harness-session`
- harness-native session ID source/confidence metadata
- the deterministic capture ladder: env/hook/jsonl/status before Peon
