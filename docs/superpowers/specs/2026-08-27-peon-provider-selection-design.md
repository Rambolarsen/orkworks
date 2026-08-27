# Peon Provider-First Selection Design

## Status

Proposed

## Context

Peon settings currently expose provider fallback order, a global Peon model, and
per-provider model fields. This makes it difficult to tell which provider and
model Peon is actually using. Ollama model discovery is also only visible after
verification, while the persisted selection is not consistently prominent.

The desired interaction is explicit and provider-first: choose one provider,
verify it, choose one model, apply the choice by making a real inference call,
and save only a configuration that has applied successfully.

## Decision

Replace the fallback-oriented Peon configuration experience with a compact
provider-first form. Peon uses exactly one selected provider and model. Fallback
order and provider-specific Peon model fields are removed from the Peon UI and
do not drive Peon runtime selection.

The form behaves as follows:

1. The user selects a provider.
2. Provider verification starts automatically.
3. Model selection remains disabled while verification is in progress.
4. After successful verification, the provider's available models are loaded.
5. The user selects a discovered model or explicitly enables manual model entry.
6. Apply performs a bounded, real minimal Peon inference using the exact
   provider/model pair.
7. A successful Apply updates the active Peon runtime configuration and records
   the applied provider, model, and time.
8. Save persists the configuration only after Apply succeeds.

Changing either provider or model invalidates the current applied state and
disables Save until the new selection successfully applies.

The currently applied provider and model are always visible when the settings
screen opens.

## Ollama

Ollama is a normal provider in the provider picker. Selecting it verifies the
configured Ollama base URL, then loads candidate models from Ollama. The base
URL remains available as Ollama-specific connection configuration rather than
part of the normal provider/model flow.

If verification cannot discover a model, the user may enter a model name
manually as an explicit override. Apply still makes a real inference call, so a
bad manual model remains unapplyable and cannot be saved.

## Runtime and data model

Peon stores one explicit selection:

```text
peonProvider: "ollama"
peonModel: "llama3.2:3b"
```

The desktop sends this exact pair to the sidecar on Apply. The sidecar verifies
the provider and performs the bounded test inference, updating active runtime
state only on success. Existing provider discovery and invocation adapters are
reused; Peon no longer iterates through fallback providers.

## States and errors

The UI distinguishes these states:

- Verifying provider
- Provider verified; choose model
- Manual model override
- Applying model
- Applied successfully
- Apply failed; Save unavailable

Provider errors are shown at the point of verification or Apply. A failed
verification does not prevent manual model entry, but it does prevent Apply
until the provider call succeeds. A changed provider/model always returns the
form to an unapplied state.

## Testing

Tests cover:

- provider selection automatically starts verification;
- models load only after successful verification;
- manual model entry works;
- changing provider or model invalidates Apply;
- failed Apply prevents Save;
- successful Apply enables Save;
- Ollama uses its configured URL and selected model;
- the applied provider/model remains visible after reopening Settings.

## Consequences

This makes Peon's active inference configuration legible and prevents a saved
configuration from silently depending on fallback order. It removes automatic
provider fallback behavior from Peon, so provider outages must be handled by
the user selecting and applying another provider. The sidecar API and settings
schema need a focused migration from the current fallback-oriented fields.
