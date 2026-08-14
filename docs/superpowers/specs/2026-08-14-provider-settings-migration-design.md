# Provider Settings Migration Design

## Goal

Stop Peon from invoking the retired Gemini CLI by default and resolve the
legacy Copilot provider ID without overriding deliberate user settings.

## Scope

Fresh provider settings contain only harnesses that declare a Peon capability:
OpenCode, Claude Code, Codex, Gemini, Aider, and Ollama. Both independent
Electron-main and renderer `ProviderId` copies express that same set; they stay
separate to preserve the Electron/renderer boundary. Gemini remains a
supported, explicit opt-in provider but is disabled in the default fallback
chain. Neither Copilot nor Antigravity appears as a Peon provider because
neither declares a Peon capability.

When Electron reads existing settings, it canonicalizes `gh-copilot` to
its removal: the legacy entry is recognized from the raw persisted array
before the ordinary provider allow-list runs, then omitted from the normalized
and persisted result. It also disables Gemini only when the raw persisted
Gemini entry exactly matches the historical default:
`{ id: "gemini", enabled: true, fallbackOrder: 3, defaultState: "unknown",
overrideState: null }`. This comparison happens before default completion.
Missing or malformed Gemini input therefore receives the new disabled default;
any valid variation of the old entry is preserved as the user's choice. The
matcher operates before sorting or renumbering fallback order, so a user
reorder cannot be mistaken for an untouched default.

Normalization reports whether it performed a migration. During Electron
startup, before the first sidecar provider-settings sync, main persists the
normalized settings only when that migration flag is set. The persistence
retains the existing revision: canonicalization is local data repair, not a
user settings edit. Ordinary reads remain non-mutating, and already-current or
user-custom Gemini settings do not trigger a write.

The sidecar retains defensive fallback behavior. On every settings apply it
filters entries against its resolved Peon provider definitions and discards the
legacy `gh-copilot` ID before storing settings. This protects stale files,
older Electron processes, and direct API callers from reaching the
missing-registry warning. The provider response and model APIs consequently
expose only executable Peon providers.

## Error Handling

Gemini is not removed: an explicit enabled setting is still attempted and its
auth failure remains a provider runtime failure. The migration is local and
does not attempt to repair Google credentials or configure Antigravity as a
Peon provider.

## Testing

Electron unit tests cover both independent provider ID declarations, fresh
defaults, removal of raw legacy Copilot input, and migration of both the legacy
default Gemini entry and a user-modified Gemini entry. They assert the startup
migration is persisted before provider sync, preserves revision, and is
idempotent. Rust tests use the production harness catalog to verify settings
application discards legacy and non-Peon IDs, so no fallback attempt or
missing-registry warning can result.
