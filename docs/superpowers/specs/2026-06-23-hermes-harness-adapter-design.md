# Hermes Harness Adapter Design

- Date: 2026-06-23
- Status: proposed

## Summary

OrkWorks should add Hermes Agent as a built-in interactive harness with documented resume support.

The first Hermes slice should include:

- built-in harness availability in the new-session flow
- a Hermes-specific sidecar adapter for launch and documented resume commands
- a neutral resume model extension for Hermes' global-latest continue behavior

The first Hermes slice should not include:

- Hermes-aware recommendation policy changes
- Hermes-specific session-id, model, context, or capacity parsing
- read access to Hermes' local SQLite state
- cwd-scoped or repo-scoped latest resume unless Hermes documents those semantics

Follow-up work deferred out of scope for this design is tracked in:

- GitHub issue [#54](https://github.com/Rambolarsen/orkworks/issues/54) for recommendation visibility
- GitHub issue [#55](https://github.com/Rambolarsen/orkworks/issues/55) for richer Hermes metadata detection
- GitHub issue [#56](https://github.com/Rambolarsen/orkworks/issues/56) for cwd/repo-scoped latest resume evaluation

## Problem

Hermes Agent is a real CLI harness with documented interactive launch and resume commands. OrkWorks can already support generic launch commands, but Hermes has enough documented session behavior that treating it as only a generic shell target would leave useful resume functionality on the table.

At the same time, Hermes does not cleanly fit OrkWorks' existing "latest in cwd" and "latest in repo" resume buckets. Hermes documents exact resume by session identifier or title and a global latest-session continue flow. OrkWorks should model those semantics honestly instead of pretending Hermes supports narrower resume scopes that have not been verified.

## Design Goals

- Add Hermes as a built-in harness in the same user-facing session-start flow as the existing built-ins.
- Support only documented Hermes CLI behavior.
- Preserve OrkWorks' launcher/observer boundary without coupling to Hermes internals.
- Represent Hermes' global latest-session continue behavior truthfully in the neutral adapter model.
- Keep the first implementation narrow enough that launch and resume support can ship without solving deeper observability work.

## Proposed Design

### Architecture

Hermes should be added as a first-class built-in harness with a dedicated harness adapter in the Rust sidecar.

The built-in harness entry makes Hermes selectable in the new-session UI with a stable name, command, and default launch path. The dedicated adapter owns Hermes-specific command construction for:

- fresh interactive launch
- exact resume through `--resume <session>`
- global latest-session continue through `--continue`

OrkWorks should not read Hermes' local SQLite state, infer undocumented command behavior, or create a Hermes-only control plane. Hermes remains responsible for its own internal state, memory, and broader agent-platform features. OrkWorks remains responsible for PTY launch, remembered-session metadata, resume command selection, and session visibility.

### Components And Data Flow

The Hermes v1 change should touch only the existing launch/resume surfaces:

- `builtin_harness_configs()` gains a `Hermes Agent` entry so Hermes appears in the new-session flow.
- `builtin_adapters()` gains a Hermes adapter with documented command mappings only.
- The neutral resume model gains a `latest_global` strategy and matching capability flag so Hermes' `--continue` behavior can be represented directly.

Runtime behavior:

1. Fresh Hermes session:
   - the user selects `Hermes Agent`
   - OrkWorks launches Hermes as a normal interactive harness session
2. Remembered Hermes session with exact ID:
   - if `resume.harnessSessionId` exists, OrkWorks uses exact resume
3. Remembered Hermes session without exact ID:
   - if exact resume is unavailable but Hermes global-latest resume is supported, OrkWorks uses `--continue`
4. Remembered Hermes session with no supported resume path:
   - the session remains visible as remembered historical state, but resume is disabled

The first Hermes slice does not solve Hermes session-ID detection. Exact resume is supported only when OrkWorks metadata already contains a Hermes session identifier. Otherwise the adapter falls back to documented global latest-session continue or disables resume.

### Command Mappings

The adapter should use documented Hermes CLI commands only.

Planned mappings:

- launch: `hermes chat`
- launch with model override: `hermes chat --model {model}`
- exact resume: `hermes --resume {harnessSessionId}`
- global latest resume: `hermes --continue`

If implementation-time verification shows Hermes requires a slightly different documented launch entrypoint, OrkWorks may adopt that documented form without changing the design intent of this spec.

### Resume Semantics

Hermes' documented `--continue` behavior is "latest session", not "latest session in cwd" or "latest session in repo". OrkWorks should not overload existing scoped-latest fields to approximate that meaning.

Instead, the neutral resume model should gain:

- a `latest_global` resume strategy
- a `resume_latest_global` adapter capability

For Hermes:

- `resume_exact = true`
- `resume_latest_global = true`
- `resume_latest_in_cwd = false`
- `resume_latest_in_repo = false`

Resume strategy selection should continue to prefer exact resume when a stored Hermes session ID exists. Global latest resume is a fallback only when exact resume is unavailable and the adapter advertises documented support.

UI copy for resumed remembered sessions should distinguish:

- exact resume of this remembered session
- continue latest Hermes session

Those are meaningfully different promises and should not be conflated.

## Error Handling And UX

Hermes should fail like any other harness.

- If the `hermes` binary is unavailable, session launch fails as a normal launch error.
- If a remembered Hermes session lacks `harnessSessionId`, OrkWorks must not invent one.
- If exact resume is impossible, OrkWorks may use documented global latest resume only when the adapter supports it.
- If no supported resume path exists, the remembered session remains visible but non-resumable.

The UI must make fallback behavior explicit. If a user resumes via global latest, the copy should communicate that OrkWorks is continuing the latest Hermes session, not guaranteeing a return to the exact remembered session.

## Non-Goals

- Recommendation-engine or Taskmaster ranking changes for Hermes
- Hermes-specific provider, model, context, or capacity parsing
- Reading `~/.hermes/state.db` or other undocumented Hermes internals
- Adding cwd-scoped or repo-scoped latest resume without documented Hermes support
- Modeling Hermes gateway, cron, delegation, kanban, or API-server features inside OrkWorks

## Testing And Validation

Implementation should verify:

- Hermes appears as a built-in harness in the new-session flow.
- The adapter generates the correct Hermes launch command.
- The adapter generates the correct exact resume command.
- The adapter generates the correct global latest resume command.
- Resume strategy selection still prefers exact resume when `harnessSessionId` exists.
- Hermes does not advertise cwd-scoped or repo-scoped latest resume unless that support is later verified.
- Remembered-session UI states distinguish exact resume, global latest resume, and disabled resume.

This first implementation should focus on adapter truthfulness and user-facing behavior rather than end-to-end validation of Hermes internals.

## Open Questions

None for this slice.

The deferred questions have already been broken out into follow-up issues instead of being left ambiguous inside the implementation scope.
