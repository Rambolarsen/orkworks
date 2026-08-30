# Peon provider-first selection

- Status: proposed
- Deciders: OrkWorks maintainers
- Date: 2026-08-30

## Context

Peon settings historically exposed provider fallback order, a global Peon
model, and per-provider model fields, with precedence rules deciding which one
applied. This made it difficult to tell which provider and model Peon was
actually using, and the fallback-oriented runtime silently substituted
providers when the primary was unavailable. A single pinned model also breaks
every provider that does not know that model: with a global model taking
precedence, a provider that cannot serve it fails even though it has suitable
models of its own.

The desired interaction is explicit and provider-first: choose one provider,
verify it, choose one model, apply the choice by making a real inference call,
and save only a configuration that has applied successfully.

## Decision

Peon uses exactly one selected provider and model; it does not iterate
fallback providers. Fallback order and the legacy global/per-provider
model-precedence settings are removed and do not drive Peon runtime selection.

The settings flow is a staged Apply/Save transaction:

1. The user selects a provider. Provider selection verifies before model
   discovery; model selection remains disabled while verification is in
   progress.
2. After successful verification, the provider's available models are loaded.
3. The user selects a discovered model, or explicitly enables manual model
   entry. Manual entry bypasses discovery only — never connectivity
   verification or Apply inference.
4. Apply performs a bounded, real Peon inference with the exact provider/model
   pair — no tools and no arbitrary terminal input.
5. Save is enabled only after successful Apply and persists atomically.

Electron owns durable user settings; the Rust sidecar owns the active runtime
selection state and returns the applied identity (provider, model, and apply
time — runtime-only, not persisted). Changing either provider or model
invalidates the applied state and disables Save until the new selection
successfully applies. The currently applied provider and model are always
visible when the settings screen opens.

The v2 selection schema replaces the legacy global/provider model-precedence
settings with one explicit selection:

```json
{
  "peonSelection": {
    "provider": "ollama",
    "model": "llama3.2:3b",
    "ollamaBaseUrl": "http://localhost:11434"
  }
}
```

`ollamaBaseUrl` is present only for the Ollama provider; verification and
Apply use the same draft base URL. The desktop sends the complete staged
selection to the sidecar, which verifies the provider and performs the bounded
test inference, updating active runtime state only after valid inference.
Existing provider discovery and invocation adapters are reused, and unrelated
provider entries and capacity metadata are preserved across migration.

For any provider whose verification succeeds but whose model discovery returns
no models, the user may enter a model name manually; manual entry is also
available as an explicit override when discovered models are present. Apply
still makes a real inference call, so a bad manual model remains unapplyable
and cannot be saved.

ADR 0044 supersedes the remaining fallback-execution contract for Peon: the
backend must no longer iterate through fallback providers on Peon's behalf
(the contract originating in ADR 0015 and carried through ADRs 0016 and 0017).
Historical ADR statuses are preserved — ADRs 0015, 0016, and 0017 keep their
existing superseded statuses for their own UI-surface decisions, and those
records are not relabeled to make this narrow replacement look like a new
historical superseding relationship.

## Consequences

This makes Peon's active inference configuration legible and prevents a saved
configuration from silently depending on fallback order or model-precedence
rules. It removes automatic provider fallback behavior from Peon: provider
outages or capacity limits must be handled by the user selecting and applying
another provider, and a capped, unavailable, or otherwise failed provider
cannot be silently bypassed by fallback execution. Provider capacity reporting
and harness launch behavior are unchanged where they are not Peon selection.
A failed verification or Apply leaves Save unavailable and the previous
applied selection in effect.

The sidecar API and settings schema need a focused migration from the current
fallback-oriented and model-precedence fields. Provider verification and the
real Apply inference become the points at which provider and capacity errors
are surfaced, while the active runtime remains unchanged until Apply succeeds.
