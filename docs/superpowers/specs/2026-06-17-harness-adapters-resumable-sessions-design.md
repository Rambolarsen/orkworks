# Harness Adapters and Resumable Session Memory Design

> Issue: #23 Harness adapter interface and resumable session memory

## Overview

Add a harness adapter layer that normalizes common AI coding harness behavior for launch, resume, detection, and future observability. The immediate product goal is session memory: OrkWorks should remember the last repo/workspace and the last selected session, then offer an explicit way to relaunch that session through the harness' own continuation mechanism.

OrkWorks must not pretend a dead PTY can be restored. After app/backend restart, remembered sessions are historical metadata until the user starts a new PTY with a resume command.

## Product Behavior

- OrkWorks remembers the last active workspace/repo.
- OrkWorks remembers the last selected OrkWorks session in that workspace.
- Remembered sessions appear in the UI as historical/resumable, not live, when their PTY process is gone.
- Resume is always an explicit user action for this design.
- If an exact harness session id is available and the adapter supports exact resume, OrkWorks offers exact resume.
- If an exact id is unavailable but the adapter supports latest-in-cwd or latest-in-repo resume, OrkWorks offers latest resume.
- If neither strategy is supported, OrkWorks shows the remembered session but disables resume.

## Backend Architecture

The adapter layer lives in `orkworksd` because the backend owns PTY launch, process environment, terminal output collection, and `.orkworks/` metadata writes.

```rust
struct HarnessAdapter {
    id: String,
    display_name: String,
    capabilities: HarnessCapabilities,
}

struct HarnessCapabilities {
    launch: bool,
    resume_exact: bool,
    resume_latest_in_cwd: bool,
    detect_session_id: bool,
    detect_model: bool,
    detect_context_usage: bool,
    detect_capacity: bool,
    native_voice: bool,
}
```

Adapters expose functions equivalent to:

- Build launch command from a neutral launch request.
- Build resume command from a neutral resume request.
- Detect harness/model/session id from process output when possible.
- Later: detect context usage and capacity state from output or harness metadata.

Built-in adapters should cover common harnesses incrementally. Unknown/custom harnesses use a generic adapter backed by command templates.

## Persisted Metadata

Extend `.orkworks/sessions/<id>.json` with neutral resume memory:

```json
{
  "harness": "codex",
  "model": "gpt-5-codex",
  "cwd": "/path/to/repo",
  "repoRoot": "/path/to/repo",
  "branch": "feature-x",
  "resume": {
    "state": "available",
    "preferredStrategy": "exact",
    "harnessSessionId": "abc123",
    "latestFallback": true,
    "lastSeenAt": "2026-06-17T12:00:00Z"
  }
}
```

Add app-level workspace memory outside the repo, stored in the desktop app's user data directory:

```json
{
  "lastWorkspacePath": "/path/to/repo",
  "recentWorkspacePaths": ["/path/to/repo"]
}
```

This app-level file lets OrkWorks reopen the last repo after restart. It must not store credentials, tokens, or raw terminal transcripts.

Add repo-local workspace memory under `.orkworks/workspace.json`:

```json
{
  "lastActiveSessionId": "orkworks-session-id",
  "lastActiveAt": "2026-06-17T12:00:00Z"
}
```

The repo-local file remembers the selected OrkWorks session for that workspace. It must not store credentials, tokens, or raw terminal transcripts.

## Resume Strategy

Resume strategy selection is deterministic:

1. If `resume.harnessSessionId` exists and the adapter supports `resume_exact`, use exact resume.
2. Otherwise, if the adapter supports `resume_latest_in_cwd`, use latest resume scoped to the stored `cwd`.
3. Otherwise, if the adapter supports latest-in-repo semantics, use latest resume scoped to the stored `repoRoot`.
4. Otherwise, mark the session as remembered but not resumable.

The UI copy should make fallback behavior clear. Exact resume should not be conflated with "latest" resume.

## Frontend Behavior

The frontend receives normalized session fields from `GET /sessions` or a future WebSocket push:

- live lifecycle status: `creating`, `running`, `ended`, `killed`, `error`
- observer status: `waiting_for_input`, `blocked`, `failed`, `done`, `stale`, `working`, `idle`
- memory state: live, remembered, resumable, unsupported
- resume strategy: exact, latest_cwd, latest_repo, none
- harness/model labels

The session list/detail panel should distinguish live PTY sessions from remembered sessions. A remembered session can be selected to inspect metadata and can show a `Resume` action when a valid strategy exists.

## Future Usage and Capacity Detection

The same adapter interface should support later capabilities without redesign:

```json
{
  "context": {
    "usedTokens": 84000,
    "maxTokens": 200000,
    "percent": 42
  },
  "limits": {
    "state": "healthy",
    "resetAt": null,
    "summary": "No cap detected"
  },
  "source": "adapter",
  "confidence": 0.8
}
```

These fields feed later capacity tracking and context usage UI. Adapters may leave these fields absent until a harness supports reliable detection.

## Boundaries

- OrkWorks does not auto-resume sessions on startup by default.
- OrkWorks does not restore dead PTY processes.
- OrkWorks does not send terminal input except through explicit user-triggered launch/resume actions.
- OrkWorks does not own git workflow, task decomposition, merging, or worktree management.
- Harness-specific commands must be verified before being hardcoded.

## Testing

- Unit-test adapter command generation for launch, exact resume, and latest resume.
- Unit-test resume strategy selection: exact id wins, latest fallback works, unsupported disables resume.
- Unit-test metadata read/write for session resume memory.
- Unit-test workspace memory read/write.
- Frontend tests should cover remembered/resumable state display and disabled resume state.
- Later adapter tests should cover context percentage and capacity parsing with captured output fixtures.
