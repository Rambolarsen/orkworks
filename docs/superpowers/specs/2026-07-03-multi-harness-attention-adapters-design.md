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

- `model provider` is for inference services, local runtimes, provider state/capacity, and Peon routing/fallback
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

Structured runtime output is allowed only when the harness itself, or a harness-local wrapper/process owned by that harness integration, can observe the structured event and post to the existing HTTP endpoint. This design does not permit adding sidecar PTY parsers or new backend event-ingestion paths for each harness. If a harness can surface a stable structured event only by teaching the sidecar to parse terminal/runtime output directly, that outcome should be rejected for this slice and left to Peon.

## Attention Lifecycle Rules

This slice is about deterministic signaling for "needs user input" and equivalent immediate-attention states, not a general replacement for Peon's broader status inference.

Per-harness issues must define not only how attention is asserted, but also how stale or cleared attention is avoided:

- The preferred deterministic signal for this slice is `waiting_for_input`.
- A harness may also map a narrowly defined equivalent event, such as an explicit permission/approval prompt, to `waiting_for_input` when the user must act before work can continue.
- Harness-specific adapters must not invent new attention states.
- Harness-specific adapters should not set `done`, `blocked`, `failed`, or rate/capacity states through this path unless the harness exposes an equally explicit, deterministic event and the issue/spec for that harness defines the mapping unambiguously.
- If the harness exposes a deterministic "resume" or "input received" event, the issue must define whether that event clears or downgrades a prior deterministic attention state.
- If the harness does not expose a deterministic clear/resume event, the issue must explicitly rely on existing lifecycle/staleness behavior and document the residual risk.
- Late hook/event delivery after a session has ended or moved on must not leave a session permanently attention-worthy; the issue must define the safe behavior and tests.

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
9. How is deterministic attention cleared, downgraded, or prevented from going stale when the harness resumes or the session ends?
10. If structured runtime output is used, where does the parsing live, and does it stay within the harness-local boundary rather than adding sidecar parsing?
11. Does the integration avoid global shell profile mutation, shared/team config writes, and intrusive wrapper behavior that changes how the harness is normally launched?

## Harness Decision Matrix

Each issue should classify the harness into one of four outcomes:

### A. Native deterministic signal available

Preferred case. The harness already exposes a hook, event, or callback that reliably means "waiting for user input" or an equivalent attention-worthy state.

Expected output:

- adapter implementation
- explicit install/config UX if needed
- tests
- docs

### B. Deterministic signal derivable from structured runtime output

Acceptable only if the signal is structured and robust, such as a documented JSONL event stream with a stable event name, and only if the harness integration can observe that structured output locally and post to the existing HTTP endpoint without extending the sidecar boundary. Free-text scraping of terminal output does not qualify; that would just duplicate Peon badly.

Expected output:

- adapter implementation only if the event contract is stable enough
- otherwise a documented rejection with rationale

### C. Reliable signal exists, but only via a boundary-violating integration

Valid rejection case. Some harnesses may expose a useful signal only if OrkWorks mutates global shell config, edits shared/team config, inserts intrusive wrappers, or adds new sidecar parsing logic that this design explicitly forbids.

Expected output:

- short research summary
- explicit statement of which boundary would be violated
- decision to reject or defer rather than force the integration

### D. No reliable deterministic signal currently available

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

## Canonical Status Mapping

To keep the harness issues consistent, this slice uses a narrow mapping policy:

- Default target status: `waiting_for_input`
- Allowed equivalent source events: explicit user-input-needed or permission/approval-needed events that block progress until the user acts
- Disallowed for this slice without a harness-specific follow-up spec: vague notifications, generic "message" events, free-text terminal output, inferred completion, inferred blockers, inferred failures, inferred quota/rate-limit conditions

If a harness appears to support a broader deterministic state model, that should be treated as separate follow-up design work rather than folded into these adapter issues.

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
- The issue classifies the harness outcome using the decision matrix in the shared design doc.
- Any structured-output adapter keeps parsing in the harness-local integration layer rather than the sidecar.
- Any install/config step is explicit, user-confirmed, and idempotent.
- Any config write is user-local, not shared/team config, and does not require global shell profile mutation.
- The adapter is a silent no-op outside OrkWorks.
- The issue defines fallback behavior for missing config, failed delivery, unsupported capability, and malformed config where applicable.
- The issue defines how deterministic attention is cleared, downgraded, or prevented from becoming stale.
- Tests cover the signal path and preserve the existing metadata priority/staleness rules.
- Tests cover session-end or late-delivery behavior where applicable.
- Docs are updated to reflect the harness outcome.
```

## Testing Expectations

If a harness adapter is implemented, its issue should cover:

- signal validation for valid and invalid attention payloads
- no-op behavior when `ORKWORKS_SESSION_ID` or `ORKWORKS_PORT` is absent
- idempotent install/config behavior where installation exists
- rejection or safe handling of malformed config files
- proof that `agent`-sourced writes still obey existing overwrite rules
- deterministic clear/downgrade behavior if the harness exposes it
- safe handling of late events after session end or after the session is no longer waiting for input

If a harness does not support deterministic attention cleanly, the issue should still end with a doc update recording that conclusion in the durable harness-outcome index.

## Documentation Impact

If any new adapter lands, update at minimum:

- `specs/orkworks-mvp.md` if the supported-harness list for deterministic attention changes materially
- `README.md` if user-facing setup steps or supported harnesses change
- harness-specific design/spec notes where needed

This shared design doc should also remain the durable index of harness outcomes. Each harness issue should update this document, or a clearly linked successor table, with:

- harness name
- decision-matrix outcome (`A`, `B`, `C`, or `D`)
- short rationale
- link to the implementing issue/PR or the rejection/defer decision

No ADR is required for each harness adapter unless the implementation changes the core trust boundary or the generic sidecar contract.

## Recommendation

Track this as one shared design doc plus six harness-specific issues. That keeps the architecture centralized while allowing each tool's capabilities to be researched and implemented on its own schedule.
