# Provider Settings Migration Design

## Goal

Stop Peon from invoking the retired Gemini CLI by default and resolve the
legacy Copilot provider ID without overriding deliberate user settings.

## Scope

Fresh provider settings use the canonical `copilot` ID. Gemini remains a
supported, explicit opt-in provider but is disabled in the default fallback
chain.

When Electron reads existing settings, it canonicalizes `gh-copilot` to
`copilot`. It also disables the Gemini entry only when that entry is still
identical to the previous default, so a user who changed Gemini's enabled
state, order, or capacity configuration keeps that decision.

The normalized settings are persisted using the existing settings write path
and then sent to the sidecar. The sidecar retains defensive fallback behavior:
the provider execution path resolves a legacy `gh-copilot` entry through the
harness registry alias rather than issuing a missing-registry warning.

## Error Handling

Gemini is not removed: an explicit enabled setting is still attempted and its
auth failure remains a provider runtime failure. The migration is local and
does not attempt to repair Google credentials or configure Antigravity as a
Peon provider.

## Testing

Electron unit tests cover fresh defaults and migration of both legacy default
entries and a user-modified Gemini entry. Rust tests cover execution of the
legacy Copilot alias through its canonical provider definition.
