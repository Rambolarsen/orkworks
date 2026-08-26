# Provider Settings Migration Design

## Goal

Stop Peon from invoking the retired Gemini CLI by default and resolve the
legacy Copilot provider ID without overriding deliberate user settings.

## Scope

Fresh provider settings contain only harnesses that declare a Peon capability:
OpenCode, Claude Code, Codex, Aider, Copilot, and Ollama. Both independent
Electron-main and renderer `ProviderId` copies express that same set; they stay
separate to preserve the Electron/renderer boundary. Gemini is retired: legacy
settings are migrated out of executable provider settings and retired harnesses
are excluded from the sidecar Peon registry. Antigravity does not appear as a
Peon provider.

Copilot is a Peon provider under its canonical `copilot` ID. Its adapter invokes
the documented non-interactive `copilot -p` form, with an empty tool allow-list
and custom instructions disabled. It supports a model override
(`--model={model}`) against a static model list sourced from the CLI's own
`--model` completion values; the rendered argument is placed ahead of the
adapter's argument tail so the prompt-consuming `-p` flag cannot swallow it
(#336). The prompt is passed as a
command argument rather than stdin; this transport remains an explicit Peon
capability so existing stdin-based adapters retain their behavior. Copilot has
no resume configuration, capacity detector, or new session-ID source.

When Electron reads existing settings, it migrates `gh-copilot` to `copilot`:
the legacy entry is recognized from the raw persisted array before the ordinary
provider allow-list runs. If a canonical entry already exists it wins and the
legacy duplicate is discarded; otherwise the legacy entry is rewritten to the
canonical ID and persisted. Every raw persisted Gemini entry is removed before
default completion, regardless of its historical
shape or user ordering. Its retained harness definition remains readable for
existing history and settings migration only; OrkWorks never selects or launches
it for new Peon inference.

Normalization reports whether it performed a migration. During Electron
startup, before the first sidecar provider-settings sync, main attempts to
persist normalized settings only when that migration flag is set. Persistence is
best-effort: a write failure leaves the repaired settings active for that launch
and retries on the next startup. The persistence retains the existing revision:
canonicalization is local data repair, not a user settings edit. Ordinary reads
remain non-mutating.

The sidecar retains defensive fallback behavior. On every settings apply it
canonicalizes `gh-copilot` to `copilot`, applies the same canonical-entry
precedence, then filters entries against its resolved Peon provider definitions.
This protects stale files, older Electron processes, and direct API callers
from reaching the missing-registry warning. The provider response and model APIs
consequently expose only executable Peon providers.

## Provider model compatibility

The v1-compatible persisted shape retains the global `peonModel` and adds an
optional `model` field to each provider entry. For every enabled provider, the
resolved model uses this precedence:

1. `provider.model` when it is non-blank.
2. The global `peonModel` fallback.
3. The provider's own default when neither setting supplies a model.

Model values are trimmed at both Electron normalization and sidecar settings
application boundaries. A blank or whitespace-only value clears the provider
override and is persisted as `null`, so it falls through to the global
`peonModel`; a blank global value resolves to the provider default. Suggestions
are provider-scoped and come only from the matching provider's model list.
Ollama receives the same resolved model value through its HTTP runner, rather
than reading the global snapshot independently. The full rationale and
implementation contract are in the [dated provider-scoped model selection
design](2026-08-25-provider-model-selection-design.md).

## Error Handling

Gemini provider settings are removed rather than invoked. The migration is
local and does not attempt to repair Google credentials or configure Antigravity
as a Peon provider.

## Testing

Electron unit tests cover both independent provider ID declarations, fresh
defaults, migration of raw legacy Copilot input including duplicate precedence,
and removal of both default-shaped and user-modified Gemini entries. They assert
the startup migration preserves revision, is idempotent, and continues if the
best-effort persistence write fails. Rust tests use the production harness
catalog to verify retired, non-Peon, and unknown IDs cannot reach fallback
execution, while legacy Copilot canonicalizes correctly. Registry and provider
tests assert the canonical Copilot definition's exact no-tool argument order
and argument prompt transport without requiring authentication.
