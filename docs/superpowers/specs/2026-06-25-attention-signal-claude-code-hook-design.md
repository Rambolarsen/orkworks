# Attention Signal: Claude Code Notification Hook Design

- Date: 2026-06-25
- Status: proposed

## Summary

OrkWorks should add a second, deterministic source of session attention state alongside Peon's existing LLM-based terminal inference: a generic localhost endpoint that a harness's own hook/notification mechanism can call directly when it knows the session is waiting on the user.

The first slice should include:

- a generic `POST /sessions/:id/attention` endpoint, harness-agnostic, that writes `AttentionState` with `metadataSource: "agent"`
- `ORKWORKS_SESSION_ID` / `ORKWORKS_PORT` env vars injected into every spawned session, so an in-session hook can address the sidecar
- a Claude Code `Notification` hook adapter, documented as a one-line command using those env vars
- a user-initiated, explicit "Install hook" action that merges the hook entry into the workspace's `.claude/settings.local.json`, gated behind confirmation
- a status/install affordance in the existing Settings/Providers UI for the Claude Code harness

The first slice should not include:

- a generic Rust trait/port abstraction for "attention signal sources" (no second implementation exists yet to justify one)
- silent, automatic hook installation at session spawn time
- equivalent research/adapters for Codex, OpenCode, Gemini CLI, Aider, or Hermes (tracked separately)
- any change to Peon's existing inference behavior or priority rules

Follow-up work deferred out of scope for this design is tracked in:

- GitHub issue [#71](https://github.com/Rambolarsen/orkworks/issues/71) (parent issue; multi-harness research checkboxes split out as needed)

## Problem

Peon infers `waiting_for_input` and related attention states purely by feeding terminal output to an LLM (`crates/orkworksd/src/peon.rs`). That works for any harness but is probabilistic and runs on a polling interval. Claude Code already emits a deterministic `Notification` hook event when it is idle waiting on input or needs permission — a much higher-confidence signal that OrkWorks isn't using.

The existing metadata priority model (`user > agent > peon > backend_inference > process > unknown`, enforced by `peon::should_overwrite`) already has a slot for `"agent"`-sourced writes that outrank Peon, but nothing currently writes with that source. There is also no existing HTTP endpoint for anything outside the daemon's own Peon loop to push session metadata.

## Design Goals

- Let Claude Code's `Notification` hook report attention state with higher confidence than Peon's inference, without changing Peon's behavior when no hook is configured.
- Keep the integration point harness-agnostic at the HTTP boundary, so a future Codex/OpenCode/etc. adapter can reuse the same endpoint without new Rust abstractions.
- Never write into the user's repo config without an explicit, confirmed user action — no silent auto-configuration at session spawn.
- Reuse the existing metadata priority/staleness invariants; do not introduce a parallel priority system.
- Keep the v1 surface area small: no new auth model, no new domain trait, no new persistence store.

## Proposed Design

### Architecture

The "pluggable interface" for other harnesses is the HTTP endpoint itself, not a Rust trait. Any harness whose attention-signal mechanism can run a shell command can call `POST /sessions/:id/attention`; any harness that exposes a native session ID can call `POST /sessions/:id/harness-session`. A Rust-side port would be a speculative abstraction with exactly one caller; it can be introduced later if a future adapter needs in-process Rust logic instead of an HTTP call.

Sidecar endpoints plus one PTY-spawn change:

1. `POST /sessions/:id/attention` — the signal receiver. Generic across harnesses.
2. `POST /sessions/:id/harness-session` — records Claude Code's `session_id` as the native harness session ID with `source: "claude_hook"`.
3. `POST /workspace/attention-hook/install` and `GET /workspace/attention-hook/status` — the explicit, user-confirmed installer for the Claude Code hook entry, scoped to the open workspace.
4. PTY spawn (`terminal_env_overrides()` in `main.rs`) gains `ORKWORKS_SESSION_ID` and `ORKWORKS_PORT` so a hook running inside the session can address the sidecar.

### Components And Data Flow

**Signal write path**

- Hook JSON from Claude Code includes `session_id`. The hook reporter extracts it and posts `{"harnessSessionId":"<session_id>","source":"claude_hook","confidence":0.98}` to `POST /sessions/:id/harness-session`.
- Request body: `{"status": "waiting_for_input", "message": "<optional>"}`. `status` validated against the existing `VALID_STATUSES` list in `peon.rs`.
- Handler reads current session metadata, calls `peon::should_overwrite("agent", age)` against the *existing* metadata source before writing — identical invariant Peon already respects, so an agent signal can't clobber `user`-set metadata, and a stale (>5 min) prior agent signal can be superseded by a fresher one.
- On write: new `metadata.rs` function (sibling to `merge_peon_inference`) sets `observedStatus`, optional `summary`/`detectedQuestion` from `message`, `metadataSource: "agent"`, `metadataConfidence: 1.0` (deterministic, distinct from Peon's probabilistic score).
- No changes to `SessionInfo`/`SessionMetadata` field shapes — reuses existing fields the frontend already renders.

**Hook install path**

- `GET /workspace/attention-hook/status`: reads `.claude/settings.local.json` in the workspace root (treats missing file as `{}`), checks whether any entry under `hooks.Notification[].hooks[]` has a `command` containing the substring `ORKWORKS_SESSION_ID`. Returns `{"installed": bool}`. Read-only; malformed JSON returns `{"installed": false, "error": "..."}` rather than failing the whole settings panel.
- `POST /workspace/attention-hook/install`: same idempotency check; if already installed, no-op success. Otherwise:
  - creates `.claude/` if missing
  - parses existing `settings.local.json` if present; on parse failure, returns an error and does **not** touch the file
  - appends a new entry to `hooks.Notification` (creating the array/key if absent), preserving every other key untouched
  - writes the file back
- The installed hook entry has no custom marker fields — Claude Code's hook schema gets only `{"hooks": [{"type": "command", "command": "..."}]}`, matching the shape already used elsewhere in this repo's own `.claude/settings.json`. Idempotency detection relies on the `ORKWORKS_SESSION_ID` substring already present in the command, not an added field.
- Installed command points to `crates/orkworksd/scripts/report-claude-session-from-hook.sh`, which reads the hook JSON from stdin and performs two guarded writes:
  ```
  POST /sessions/$ORKWORKS_SESSION_ID/harness-session with session_id and source claude_hook
  POST /sessions/$ORKWORKS_SESSION_ID/attention with status waiting_for_input
  ```
  Guarded so it's a silent no-op when the hook fires in a session Claude Code runs outside OrkWorks (env vars absent).

**Electron / frontend**

- Electron `main.ts` gains `install-claude-code-hook` and `get-claude-code-hook-status` IPC handlers, thin proxies to the two sidecar endpoints — same pattern as `save-provider-settings` / `get-provider-models`.
- The Claude Code harness's existing Settings/Providers area gains a status line ("Attention signal: not configured" / "✓ Installed") and an "Install Notification hook" button. Click → confirmation dialog naming the file that will be modified → confirm → install → status updates.
- No new panel, no new route; reuses existing provider settings UI real estate (see ADR 0017 — provider context stays session/workspace scoped, not app-wide).

### Why `.claude/settings.local.json`, not `settings.json`

`settings.local.json` is Claude Code's own convention for personal, gitignored, machine-specific config. This hook is meaningless without a running OrkWorks sidecar on a specific local port, so it is inherently personal, not team-shared. Writing into `settings.json` would commit a tool-specific curl call into the team's shared config, which is wrong even if the write itself were otherwise safe.

## Error Handling And UX

- `/sessions/:id/attention` with an unknown session ID: 404, no write.
- `/sessions/:id/attention` with an invalid `status` value: 400, no write.
- Hook install with malformed existing `settings.local.json`: error surfaced in the UI ("couldn't parse .claude/settings.local.json — fix and retry"), file untouched.
- Hook install when no workspace is open: install/status endpoints return an error; UI hides the affordance entirely when there's no active workspace (mirrors how other workspace-scoped settings already behave).
- Double-click "Install": idempotency check makes the second call a no-op success, not a duplicate hook entry.

## Non-Goals

- A `AttentionSignalSource` Rust trait/port — deferred until a second adapter actually needs one.
- Automatic hook installation at session spawn time.
- Codex, OpenCode, Gemini CLI, Aider, or Hermes equivalents — tracked as separate research issues split from #71.
- Any new authentication/authorization model for the sidecar's HTTP API — this endpoint binds to the same unauthenticated `127.0.0.1:0` listener as everything else, consistent with current trust posture.
- Parsing the Notification hook's free-text message into a specific `AttentionState` variant beyond `waiting_for_input` — Peon's inference remains responsible for distinguishing `blocked`/`failed`/etc.

## Testing And Validation

Implementation should verify:

- `should_overwrite`-equivalent priority check applied to the new write path: agent signal overwrites peon/backend_inference/process/unknown metadata; cannot overwrite fresh `user` metadata; cannot overwrite fresh `agent` metadata (must be stale, same >5min rule).
- Merge function for hook install: fresh file (no `.claude/` dir), file with unrelated existing keys (e.g. only `permissions`), already-installed idempotency (no duplicate entry on second install), malformed JSON rejected without modifying the file.
- `/sessions/:id/attention`: unknown session ID rejected, invalid status rejected.
- Env var injection: spawned session process environment contains `ORKWORKS_SESSION_ID` matching the session and `ORKWORKS_PORT` matching the bound port.
- Frontend: extend the existing `tests/providersPanel.test.ts`-style coverage rather than add a new test file, for the install button's not-configured/installed/error states.

## Open Questions

None for this slice. Multi-harness equivalents are intentionally deferred to follow-up issues rather than left ambiguous inside this implementation's scope.

## Related Documentation Updates

Required before implementation lands, per AGENTS.md:

- `specs/taskmaster.md` and/or `specs/orkworks-mvp.md`: add a short section describing this as a second metadata-source path alongside Peon's inference — deterministic, harness-supplied, opt-in, manual-install-only. (Both specs currently describe only Peon's LLM-based inference for `waiting_for_input`.)
- A new ADR recording the decision to add an unauthenticated localhost endpoint accepting external attention signals, opt-in via user-confirmed hook install, with no auto-write into shared repo config as the rejected alternative.
