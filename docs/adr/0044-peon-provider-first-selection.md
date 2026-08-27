# Peon provider-first selection

- Status: proposed
- Deciders: OrkWorks maintainers
- Date: 2026-08-27

## Context

Peon settings currently expose provider fallback order, a global Peon model,
and per-provider model fields. This makes it difficult to tell which provider
and model Peon is actually using. Ollama model discovery is also only visible
after verification, while the persisted selection is not consistently
prominent.

The desired interaction is explicit and provider-first: choose one provider,
verify it, choose one model, apply the choice by making a real inference call,
and save only a configuration that has applied successfully.

## Decision

Replace the fallback-oriented Peon configuration experience with a compact
provider-first form. Peon uses exactly one selected provider and model. Fallback
order and provider-specific Peon model fields are removed from the Peon UI and
do not drive Peon runtime selection.

The form uses a staged Apply/Save transaction:

1. The user selects a provider.
2. Provider verification starts automatically.
3. Model selection remains disabled while verification is in progress.
4. After successful verification, the provider's available models are loaded.
5. The user selects a discovered model or explicitly enables manual model entry.
6. Apply performs a bounded, real minimal Peon inference using the exact
   provider/model pair.
7. A successful Apply updates the active Peon runtime configuration and records
   the applied provider, model, and time. The applied timestamp is runtime-only;
   Save persists the provider/model pair but does not persist that timestamp.
8. Save persists the configuration only after Apply succeeds.

Changing either provider or model invalidates the current applied state and
disables Save until the new selection successfully applies. The currently
applied provider and model are always visible when the settings screen opens.

The v2 selection schema stores one explicit selection:

```json
{
  "peonProvider": "ollama",
  "peonModel": "llama3.2:3b"
}
```

The desktop sends this exact pair to the sidecar on Apply. The sidecar
verifies the provider and performs the bounded test inference, updating active
runtime state only on success. Existing provider discovery and invocation
adapters are reused; Peon no longer iterates through fallback providers.

Ollama remains a normal provider in the provider picker. Selecting it verifies
the configured Ollama base URL, then loads candidate models from Ollama. For
any provider whose verification succeeds but whose model discovery returns no
models, the user may enter a model name manually. Manual entry is also
available as an explicit override when discovered models are present. Apply
still makes a real inference call, so a bad manual model remains unapplyable
and cannot be saved.

ADR 0044 supersedes ADR 0017 specifically for its remaining fallback-execution
contract for Peon: the backend must no longer iterate through fallback
providers. ADR 0017 remains historically marked `superseded` for its earlier
Settings-surface decision, and its record and README index entry retain that
status; neither is relabeled to make this narrow replacement look like a new
historical superseding relationship. ADR 0044 records the replacement decision
for the Peon runtime and selection flow without rewriting historical records.

## Consequences

This makes Peon's active inference configuration legible and prevents a saved
configuration from silently depending on fallback order. It removes automatic
provider fallback behavior from Peon, so provider outages or capacity limits
must be handled by the user selecting and applying another provider. A capped,
unavailable, or otherwise failed provider cannot be silently bypassed by
fallback execution, and a failed verification or Apply leaves Save unavailable.

The sidecar API and settings schema need a focused migration from the current
fallback-oriented fields. Provider verification and the real Apply inference
become the points at which provider and capacity errors are surfaced, while
the active runtime remains unchanged until Apply succeeds.
