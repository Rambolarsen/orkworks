# Harness Version Status Design

## Problem

When an installed harness is detected at an unsupported version, the sidecar
currently reports `NeedsTrust` for every JSON-hook integration. The desktop
renderer interprets that activation as a request to approve hooks with
`/hooks`, which is valid for Codex but misleading for Claude Code, Copilot,
Gemini, and OpenCode. The unsupported-version diagnostic already identifies
the actual problem.

## Decision

Report `Unknown` as the activation state when the detected tool version is
incompatible. Apply this in the shared `JsonHookHandler` and in OpenCode's
parallel status implementation. Keep the existing `unsupported_tool_version`
diagnostic unchanged.

`Unknown` represents that the integration cannot be considered active for the
detected version without adding a new cross-boundary protocol state. The
normal contract activation remains authoritative only when the tool version is
compatible and the owned fragment is installed.

## Alternatives considered

- Add an `IncompatibleVersion` activation: clearer in isolation, but expands
  the Rust/TypeScript protocol and renderer for a case already explained by
  the diagnostic.
- Reuse the contract's normal activation: incorrect for Codex, whose normal
  activation is genuinely `NeedsTrust` and would retain the misleading
  `/hooks` instruction.

## Testing

- Add a regression test for a non-Codex JSON-hook harness with an installed
  fragment and an incompatible detected version.
- Add or update the OpenCode regression test for the same state.
- Verify the desktop integration test continues to render the version
  diagnostic without the Codex-specific trust instruction.

No renderer behavior or microphone/voice capability is changed.
