# Multi-Harness Attention Adapter Issue Drafts

- Date: 2026-07-03
- Status: draft
- Related spec: `docs/superpowers/specs/2026-07-03-multi-harness-attention-adapters-design.md`

## Codex Deterministic Attention Adapter

```md
## Summary

Add or research a deterministic attention adapter for Codex using the existing `POST /sessions/:id/attention` sidecar endpoint.

## Why

Claude Code is only the first concrete adapter. Deterministic attention is a harness concern, not a model-provider concern, and OrkWorks should evaluate equivalent support for Codex rather than relying solely on Peon inference.

The repo already captures Codex native session IDs from structured `thread.started` events for harness-session mapping. This issue should determine whether Codex exposes a similarly stable, structured path for user-input-needed attention events.

## Scope

- Determine whether Codex exposes a supported deterministic attention signal.
- If yes, implement an opt-in adapter using the existing sidecar contract.
- If no, document the limitation and keep Peon as fallback.

## Non-Goals

- No model-provider integration
- No terminal-output scraping
- No automatic config writes at session spawn
- No silent modification of shared/team config files
- No new sidecar PTY parser for Codex-specific runtime output

## Acceptance Criteria

- The issue identifies the exact deterministic signal source, or documents that none exists cleanly.
- The issue classifies Codex using the shared decision matrix (`A`, `B`, `C`, or `D`).
- Any structured-output adapter keeps parsing in the harness-local integration layer rather than the sidecar.
- Any install/config step is explicit, user-confirmed, and idempotent.
- Any config write is user-local, not shared/team config, and does not require global shell profile mutation.
- The adapter is a silent no-op outside OrkWorks.
- The issue defines fallback behavior for missing config, failed delivery, unsupported capability, and malformed config where applicable.
- The issue defines how deterministic attention is cleared, downgraded, or prevented from becoming stale.
- Allowed deterministic mapping is narrowly scoped to `waiting_for_input` or an explicit approval/input-needed equivalent.
- Tests cover the signal path and preserve the existing metadata priority/staleness rules.
- Tests cover session-end or late-delivery behavior where applicable.
- Docs are updated to reflect the Codex outcome and the shared harness-outcome index is updated.
```

## OpenCode Deterministic Attention Adapter

```md
## Summary

Add or research a deterministic attention adapter for OpenCode using the existing `POST /sessions/:id/attention` sidecar endpoint.

## Why

Claude Code is only the first concrete adapter. Deterministic attention is a harness concern, not a model-provider concern, and OrkWorks should evaluate equivalent support for OpenCode rather than relying solely on Peon inference.

OpenCode already matters to OrkWorks as a first-class harness. This issue should determine whether it exposes a supported notification, hook, callback, or structured event that reliably means the session is waiting on the user.

## Scope

- Determine whether OpenCode exposes a supported deterministic attention signal.
- If yes, implement an opt-in adapter using the existing sidecar contract.
- If no, document the limitation and keep Peon as fallback.

## Non-Goals

- No model-provider integration
- No terminal-output scraping
- No automatic config writes at session spawn
- No silent modification of shared/team config files
- No new sidecar PTY parser for OpenCode-specific runtime output

## Acceptance Criteria

- The issue identifies the exact deterministic signal source, or documents that none exists cleanly.
- The issue classifies OpenCode using the shared decision matrix (`A`, `B`, `C`, or `D`).
- Any structured-output adapter keeps parsing in the harness-local integration layer rather than the sidecar.
- Any install/config step is explicit, user-confirmed, and idempotent.
- Any config write is user-local, not shared/team config, and does not require global shell profile mutation.
- The adapter is a silent no-op outside OrkWorks.
- The issue defines fallback behavior for missing config, failed delivery, unsupported capability, and malformed config where applicable.
- The issue defines how deterministic attention is cleared, downgraded, or prevented from becoming stale.
- Allowed deterministic mapping is narrowly scoped to `waiting_for_input` or an explicit approval/input-needed equivalent.
- Tests cover the signal path and preserve the existing metadata priority/staleness rules.
- Tests cover session-end or late-delivery behavior where applicable.
- Docs are updated to reflect the OpenCode outcome and the shared harness-outcome index is updated.
```

## Aider Deterministic Attention Adapter

```md
## Summary

Add or research a deterministic attention adapter for Aider using the existing `POST /sessions/:id/attention` sidecar endpoint.

## Why

Claude Code is only the first concrete adapter. Deterministic attention is a harness concern, not a model-provider concern, and OrkWorks should evaluate equivalent support for Aider rather than relying solely on Peon inference.

Aider may or may not expose a supported hook/plugin/config surface that can emit an explicit user-input-needed event. This issue should answer that cleanly rather than assuming feature parity with Claude.

## Scope

- Determine whether Aider exposes a supported deterministic attention signal.
- If yes, implement an opt-in adapter using the existing sidecar contract.
- If no, document the limitation and keep Peon as fallback.

## Non-Goals

- No model-provider integration
- No terminal-output scraping
- No automatic config writes at session spawn
- No silent modification of shared/team config files
- No new sidecar PTY parser for Aider-specific runtime output

## Acceptance Criteria

- The issue identifies the exact deterministic signal source, or documents that none exists cleanly.
- The issue classifies Aider using the shared decision matrix (`A`, `B`, `C`, or `D`).
- Any structured-output adapter keeps parsing in the harness-local integration layer rather than the sidecar.
- Any install/config step is explicit, user-confirmed, and idempotent.
- Any config write is user-local, not shared/team config, and does not require global shell profile mutation.
- The adapter is a silent no-op outside OrkWorks.
- The issue defines fallback behavior for missing config, failed delivery, unsupported capability, and malformed config where applicable.
- The issue defines how deterministic attention is cleared, downgraded, or prevented from becoming stale.
- Allowed deterministic mapping is narrowly scoped to `waiting_for_input` or an explicit approval/input-needed equivalent.
- Tests cover the signal path and preserve the existing metadata priority/staleness rules.
- Tests cover session-end or late-delivery behavior where applicable.
- Docs are updated to reflect the Aider outcome and the shared harness-outcome index is updated.
```

## Gemini CLI Deterministic Attention Adapter

```md
## Summary

Add or research a deterministic attention adapter for Gemini CLI using the existing `POST /sessions/:id/attention` sidecar endpoint.

## Why

Claude Code is only the first concrete adapter. Deterministic attention is a harness concern, not a model-provider concern, and OrkWorks should evaluate equivalent support for Gemini CLI rather than relying solely on Peon inference.

This issue should determine whether Gemini CLI exposes a supported notification, hook, or structured event mechanism for explicit user-input-needed states. If not, the correct outcome is to document the limitation and leave Peon in place.

## Scope

- Determine whether Gemini CLI exposes a supported deterministic attention signal.
- If yes, implement an opt-in adapter using the existing sidecar contract.
- If no, document the limitation and keep Peon as fallback.

## Non-Goals

- No model-provider integration
- No terminal-output scraping
- No automatic config writes at session spawn
- No silent modification of shared/team config files
- No new sidecar PTY parser for Gemini CLI-specific runtime output

## Acceptance Criteria

- The issue identifies the exact deterministic signal source, or documents that none exists cleanly.
- The issue classifies Gemini CLI using the shared decision matrix (`A`, `B`, `C`, or `D`).
- Any structured-output adapter keeps parsing in the harness-local integration layer rather than the sidecar.
- Any install/config step is explicit, user-confirmed, and idempotent.
- Any config write is user-local, not shared/team config, and does not require global shell profile mutation.
- The adapter is a silent no-op outside OrkWorks.
- The issue defines fallback behavior for missing config, failed delivery, unsupported capability, and malformed config where applicable.
- The issue defines how deterministic attention is cleared, downgraded, or prevented from becoming stale.
- Allowed deterministic mapping is narrowly scoped to `waiting_for_input` or an explicit approval/input-needed equivalent.
- Tests cover the signal path and preserve the existing metadata priority/staleness rules.
- Tests cover session-end or late-delivery behavior where applicable.
- Docs are updated to reflect the Gemini CLI outcome and the shared harness-outcome index is updated.
```

## Hermes Deterministic Attention Adapter

```md
## Summary

Add or research a deterministic attention adapter for Hermes using the existing `POST /sessions/:id/attention` sidecar endpoint.

## Why

Claude Code is only the first concrete adapter. Deterministic attention is a harness concern, not a model-provider concern, and OrkWorks should evaluate equivalent support for Hermes rather than relying solely on Peon inference.

This issue should determine whether Hermes exposes any session-local hook, callback, or structured event stream suitable for deterministic attention without violating the harness-local boundary defined in the shared design doc.

## Scope

- Determine whether Hermes exposes a supported deterministic attention signal.
- If yes, implement an opt-in adapter using the existing sidecar contract.
- If no, document the limitation and keep Peon as fallback.

## Non-Goals

- No model-provider integration
- No terminal-output scraping
- No automatic config writes at session spawn
- No silent modification of shared/team config files
- No new sidecar PTY parser for Hermes-specific runtime output

## Acceptance Criteria

- The issue identifies the exact deterministic signal source, or documents that none exists cleanly.
- The issue classifies Hermes using the shared decision matrix (`A`, `B`, `C`, or `D`).
- Any structured-output adapter keeps parsing in the harness-local integration layer rather than the sidecar.
- Any install/config step is explicit, user-confirmed, and idempotent.
- Any config write is user-local, not shared/team config, and does not require global shell profile mutation.
- The adapter is a silent no-op outside OrkWorks.
- The issue defines fallback behavior for missing config, failed delivery, unsupported capability, and malformed config where applicable.
- The issue defines how deterministic attention is cleared, downgraded, or prevented from becoming stale.
- Allowed deterministic mapping is narrowly scoped to `waiting_for_input` or an explicit approval/input-needed equivalent.
- Tests cover the signal path and preserve the existing metadata priority/staleness rules.
- Tests cover session-end or late-delivery behavior where applicable.
- Docs are updated to reflect the Hermes outcome and the shared harness-outcome index is updated.
```

## Copilot Deterministic Attention Adapter

```md
## Summary

Add or research a deterministic attention adapter for Copilot using the existing `POST /sessions/:id/attention` sidecar endpoint.

## Why

Claude Code is only the first concrete adapter. Deterministic attention is a harness concern, not a model-provider concern, and OrkWorks should evaluate equivalent support for Copilot rather than relying solely on Peon inference.

This issue should determine whether the relevant Copilot CLI surface exposes a supported notification or structured event mechanism that fits OrkWorks' session model. If the available surface is editor-centric rather than terminal-session-centric, the issue should document that mismatch and avoid forcing an adapter that does not fit.

## Scope

- Determine whether Copilot exposes a supported deterministic attention signal.
- If yes, implement an opt-in adapter using the existing sidecar contract.
- If no, document the limitation and keep Peon as fallback.

## Non-Goals

- No model-provider integration
- No terminal-output scraping
- No automatic config writes at session spawn
- No silent modification of shared/team config files
- No new sidecar PTY parser for Copilot-specific runtime output

## Acceptance Criteria

- The issue identifies the exact deterministic signal source, or documents that none exists cleanly.
- The issue classifies Copilot using the shared decision matrix (`A`, `B`, `C`, or `D`).
- Any structured-output adapter keeps parsing in the harness-local integration layer rather than the sidecar.
- Any install/config step is explicit, user-confirmed, and idempotent.
- Any config write is user-local, not shared/team config, and does not require global shell profile mutation.
- The adapter is a silent no-op outside OrkWorks.
- The issue defines fallback behavior for missing config, failed delivery, unsupported capability, and malformed config where applicable.
- The issue defines how deterministic attention is cleared, downgraded, or prevented from becoming stale.
- Allowed deterministic mapping is narrowly scoped to `waiting_for_input` or an explicit approval/input-needed equivalent.
- Tests cover the signal path and preserve the existing metadata priority/staleness rules.
- Tests cover session-end or late-delivery behavior where applicable.
- Docs are updated to reflect the Copilot outcome and the shared harness-outcome index is updated.
```
