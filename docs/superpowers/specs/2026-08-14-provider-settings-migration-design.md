# Provider Settings Migration Design

## Goal

Stop Peon from invoking the retired Gemini CLI by default and resolve the
legacy Copilot provider ID without overriding deliberate user settings.

## Scope

Fresh provider settings use the canonical `copilot` ID. Both Electron
`ProviderId` copies and the settings validator recognize `copilot`; the parser
accepts `gh-copilot` only as legacy input. Gemini remains a supported, explicit
opt-in provider but is disabled in the default fallback chain.

When Electron reads existing settings, it canonicalizes `gh-copilot` to
`copilot`. If a document contains both names, the explicit canonical
`copilot` entry wins, independent of array order. It also disables Gemini only
when the raw persisted Gemini entry exactly matches the historical default:
`{ id: "gemini", enabled: true, fallbackOrder: 3, defaultState: "unknown",
overrideState: null }`. This comparison happens before default completion.
Missing or malformed Gemini input therefore receives the new disabled default;
any valid variation of the old entry is preserved as the user's choice.

Normalization reports whether it performed a migration. During Electron
startup, before the first sidecar provider-settings sync, main persists the
normalized settings only when that migration flag is set. The persistence
retains the existing revision: canonicalization is local data repair, not a
user settings edit. Ordinary reads remain non-mutating, and already-current or
user-custom Gemini settings do not trigger a write.

The sidecar retains defensive fallback behavior. `ResolvedHarnessRegistry`
exposes provider-definition resolution through its existing alias mechanism;
the Peon execution path obtains both the canonical provider ID and definition
from that API. A legacy `gh-copilot` payload therefore invokes canonical
`copilot` and records runtime/observation state under `copilot`, without the
missing-registry warning.

## Error Handling

Gemini is not removed: an explicit enabled setting is still attempted and its
auth failure remains a provider runtime failure. The migration is local and
does not attempt to repair Google credentials or configure Antigravity as a
Peon provider.

## Testing

Electron unit tests cover fresh defaults, canonical type/parser behavior,
legacy Copilot normalization, duplicate precedence, and migration of both the
legacy default Gemini entry and a user-modified Gemini entry. They assert the
startup migration is persisted before provider sync, preserves revision, and
is idempotent. Rust tests use the production harness catalog to verify a
legacy Copilot ID resolves and executes as canonical `copilot`, never entering
the missing-registry path.
