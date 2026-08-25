# Provider-Scoped Peon Model Selection

## Problem

Provider settings currently store one global `peonModel`. During fallback,
that model ID is rendered into every harness provider that supports model
overrides. A model valid for Ollama can therefore be sent to Copilot, Claude
Code, or another provider that does not know the ID. The fallback then fails by
skipping the provider instead of using a compatible configured model.

## Decision

Keep the existing global `peonModel` as a backward-compatible fallback and add
an optional `model` override to each `ProviderSettingsEntry`.

For each enabled provider attempt, resolve the model once in a shared helper
using this order:

1. The provider entry's non-empty `model` override.
2. The global `peonModel` value.
3. No model argument, allowing the provider to choose its default.

Providers that do not support model overrides continue to receive no model
argument. Existing persisted settings remain valid because the new entry field
is optional and missing entries use the global fallback.

The resolver trims model values and treats empty or whitespace-only strings as
`null`; Electron normalization and Rust settings application enforce the same
rule. The resolved value is passed to both command-based providers and the
Ollama HTTP runner. Ollama remains the exception that requires a non-empty
resolved model to make a request; with no model it records the existing
configuration error and falls back.

## Desktop behavior

The global field is relabeled as the default Peon model and remains available
for users who want one model across providers. Each provider row gets a model
override control populated from that provider's existing `/providers/:id/models`
response, with an explicit empty option meaning “use default.” A provider with
no model list keeps the empty option and can still use the global fallback.

Saving provider settings sends the per-provider overrides and the unchanged
global fallback through the existing settings controller and sidecar endpoint.
Electron's `DEFAULT_PROVIDER_SETTINGS`, provider-entry normalizer, and
read-normalize-save path preserve the new field rather than dropping it. No
Electron/renderer boundary is crossed; the duplicated settings types are
updated independently. Provider model controls remain editable text inputs
with provider-specific datalist suggestions; clearing the input means use the
global fallback, while manually entered unknown IDs remain pass-through.

## Error handling and compatibility

An unknown or invalid provider-specific model is passed through exactly like a
user-entered global model; the provider reports the failure and fallback
continues. OrkWorks does not silently substitute a model from another provider.
Existing payloads without `model` fields deserialize and behave exactly as
before. Clearing an override stores `null`, which selects the global fallback.
Successful provider observations record the resolved model actually used,
including `null` when no model was supplied, rather than always recording the
global fallback.

## Testing

- Rust serialization tests verify missing provider models remain compatible and
  explicit provider models round-trip.
- Rust inference tests verify one shared resolver drives command arguments,
  Ollama HTTP requests, provider observations, provider overrides win, the
  global model remains the fallback, and providers without model support
  receive no model argument.
- Desktop tests verify provider-specific model choices are sourced from the
  matching provider list, clearing an override falls back to the global model,
  whitespace normalizes to `null`, and Electron read-normalize-save preserves
  both scopes.

No provider registry redesign, automatic model translation, or unrelated
fallback policy change is included.
