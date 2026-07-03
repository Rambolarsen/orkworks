# Multi-Harness Attention Adapters Design

- Date: 2026-07-03
- Status: proposed

## Summary

OrkWorks already has the correct backend seam for deterministic attention updates: `POST /sessions/:id/attention` on the localhost sidecar, with `metadataSource: "agent"` and the existing priority/staleness rules. Claude Code is the first concrete adapter because it exposes a `Notification` hook that can be installed explicitly into `.claude/settings.local.json`.

This design defines the follow-up split for the remaining harnesses:

- Codex
- OpenCode
- Aider
- Gemini CLI
- Hermes
- Copilot

The backend contract remains generic and unchanged. Each harness gets its own issue for research plus implementation of an opt-in adapter that can emit deterministic attention when that harness supports it. If a harness does not currently expose a reliable attention signal, its issue should conclude with a documented fallback decision rather than force a weak or invasive integration.

## Problem

The current repo language can be misread as "attention hook support" being a Claude-specific capability rather than a harness-agnostic feature with a Claude-specific first adapter.

That is the wrong boundary:

- `provider` is for Peon inference/runtime fallback
- `harness` is the interactive coding tool session
- deterministic attention signals come from the harness session itself

Without explicit follow-up work, OrkWorks risks:

- keeping deterministic attention materially better for Claude than for other supported harnesses
- conflating provider integrations with harness integrations
- introducing ad hoc per-tool implementations without a shared acceptance bar

## Design Goals

- Keep the sidecar attention write path generic and unchanged.
- Split follow-up work into one independently schedulable issue per harness.
- Require opt-in, harness-local installation/configuration only where the harness supports it.
- Preserve the "observe and recommend before it controls" principle: no silent config writes, no auto-install at session spawn, no global shell profile mutation.
- Allow a harness issue to conclude "no deterministic adapter available right now" when the tool surface is insufficient.
- Preserve Peon inference as the fallback when no harness adapter exists or is installed.

## Non-Goals

- No new Rust trait or plugin system for attention sources.
- No provider-level attention integration.
- No requirement that every harness must end with an installable adapter.
- No expansion of OrkWorks into arbitrary automation inside the harness.
- No change to Peon's inference logic, metadata priority, or stale-write policy.

## Common Architecture

The architecture stays exactly as established by the Claude work:

1. The sidecar injects `ORKWORKS_SESSION_ID` and `ORKWORKS_PORT` into spawned sessions.
2. A harness-local mechanism, when available, emits a deterministic "needs user input" signal.
3. That mechanism posts to `POST /sessions/:id/attention`.
4. The sidecar writes `metadataSource: "agent"` with the existing overwrite rules.
5. If no deterministic signal exists, Peon remains the only attention source.

The pluggable boundary is the HTTP endpoint plus the session env vars, not a Rust abstraction layer.

## Per-Harness Acceptance Bar

Every harness-specific issue must answer these questions explicitly before implementation is considered complete:

1. What exact harness event or output is treated as deterministic attention?
2. How is the signal emitted: native hook, JSONL event stream, wrapper command, config file entry, or some other supported mechanism?
3. Where does the configuration live, if any, and is it user-local rather than shared/team config?
4. Is installation/configuration explicit and user-confirmed?
5. Is the behavior idempotent on repeated install attempts?
6. Does it become a silent no-op when the harness runs outside OrkWorks?
7. What is the fallback behavior when the harness lacks the capability, the config is absent, or the signal path fails?
8. What exact tests prove the adapter works and does not violate the metadata priority rules?

## Harness Decision Matrix

Each issue should classify the harness into one of three outcomes:

### A. Native deterministic signal available

Preferred case. The harness already exposes a hook, event, or callback that reliably means "waiting for user input" or an equivalent attention-worthy state.

Expected output:

- adapter implementation
- explicit install/config UX if needed
- tests
- docs

### B. Deterministic signal derivable from structured runtime output

Acceptable only if the signal is structured and robust, such as a documented JSONL event stream with a stable event name. Free-text scraping of terminal output does not qualify; that would just duplicate Peon badly.

Expected output:

- adapter implementation only if the event contract is stable enough
- otherwise a documented rejection with rationale

### C. No reliable deterministic signal currently available

Valid outcome. The issue should document why the harness cannot support this cleanly today and should leave Peon as the fallback.

Expected output:

- short research summary
- explicit "not implementable without brittle scraping or intrusive wrapping"
- any future upstream feature request worth tracking

## Harness-Specific Scope

### Codex

Investigate whether Codex exposes a stable structured event or hook path comparable to Claude's `Notification` hook. The repo already captures Codex native session IDs from `thread.started` JSONL events for harness-session mapping; this issue should determine whether the same or a related structured event channel can drive deterministic attention without terminal scraping.

### OpenCode

Investigate whether OpenCode exposes a user-input-needed hook, callback, or structured session event. If it supports only session ID capture and not attention signaling, the issue should stop there and document the limitation cleanly.

### Aider

Investigate whether Aider offers a supported hook/plugin/config mechanism that can emit a deterministic "waiting for input" signal. Reject shell-output scraping as non-deterministic and out of scope.

### Gemini CLI

Investigate whether Gemini CLI exposes any supported notification, hook, or structured event mechanism for user-input-needed states. If not, document fallback to Peon.

### Hermes

Investigate whether Hermes exposes any session-local event stream or hook suitable for deterministic attention. Keep the same install/no-op/idempotency bar as every other harness.

### Copilot

Investigate whether the relevant Copilot CLI surface exposes a supported notification or structured event mechanism. If the available surface is editor-centric rather than terminal-session-centric, the issue should document that mismatch and avoid forcing an adapter that does not fit the OrkWorks session model.

## Proposed Issue Split

Create one issue per harness:

1. `Codex deterministic attention adapter`
2. `OpenCode deterministic attention adapter`
3. `Aider deterministic attention adapter`
4. `Gemini CLI deterministic attention adapter`
5. `Hermes deterministic attention adapter`
6. `Copilot deterministic attention adapter`

Each issue should include:

- problem statement
- research task for harness capabilities
- implementation only if a deterministic signal exists
- explicit non-goal that Peon remains the fallback when no clean adapter exists
- acceptance criteria derived from the common acceptance bar in this spec

## Draft Issue Template

Use this body shape for each harness issue, replacing `<HARNESS>` with the tool name and filling in any harness-specific context:

```md
## Summary

Add or research a deterministic attention adapter for `<HARNESS>` using the existing `POST /sessions/:id/attention` sidecar endpoint.

## Why

Claude Code is only the first concrete adapter. Deterministic attention is a harness concern, not a provider concern, and OrkWorks should evaluate equivalent support for `<HARNESS>` rather than relying solely on Peon inference.

## Scope

- Determine whether `<HARNESS>` exposes a supported deterministic attention signal.
- If yes, implement an opt-in adapter using the existing sidecar contract.
- If no, document the limitation and keep Peon as fallback.

## Non-Goals

- No provider integration
- No terminal-output scraping
- No automatic config writes at session spawn
- No silent modification of shared/team config files

## Acceptance Criteria

- The issue identifies the exact deterministic signal source, or documents that none exists cleanly.
- Any install/config step is explicit, user-confirmed, and idempotent.
- The adapter is a silent no-op outside OrkWorks.
- Tests cover the signal path and preserve the existing metadata priority/staleness rules.
- Docs are updated to reflect the harness outcome.
```

## Testing Expectations

If a harness adapter is implemented, its issue should cover:

- signal validation for valid and invalid attention payloads
- no-op behavior when `ORKWORKS_SESSION_ID` or `ORKWORKS_PORT` is absent
- idempotent install/config behavior where installation exists
- rejection or safe handling of malformed config files
- proof that `agent`-sourced writes still obey existing overwrite rules

If a harness does not support deterministic attention cleanly, the issue should still end with a short doc update or issue comment recording that conclusion.

## Documentation Impact

If any new adapter lands, update at minimum:

- `specs/orkworks-mvp.md` if the supported-harness list for deterministic attention changes materially
- `README.md` if user-facing setup steps or supported harnesses change
- harness-specific design/spec notes where needed

No ADR is required for each harness adapter unless the implementation changes the core trust boundary or the generic sidecar contract.

## Recommendation

Track this as one shared design doc plus six harness-specific issues. That keeps the architecture centralized while allowing each tool's capabilities to be researched and implemented on its own schedule.
