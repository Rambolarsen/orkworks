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

The renderer will recognize the existing `unsupported_tool_version` diagnostic
when an integration is installed and suppress the normal success or trust
confirmation. It will show the version-incompatibility message without
suggesting `/hooks` or claiming that the installed integration is active. This
is a presentation-only use of the existing diagnostic; no new protocol value
is required.

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
- Add or update the OpenCode regression test for an installed fragment and an
  incompatible detected version.
- Update existing HTTP expectations that currently assert `needs_trust` for
  incompatible versions.
- Add a desktop presentation test that asserts the unsupported-version state
  omits both the Codex-specific `/hooks` instruction and the normal
  "hooks installed" success message.

No voice capability or microphone behavior is changed.
