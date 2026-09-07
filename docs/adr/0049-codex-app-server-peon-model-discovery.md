# Codex app-server is the live Peon model catalog

- Status: accepted
- Deciders: OrkWorks maintainers
- Date: 2026-09-07

## Context

Peon's Codex model picker is backed by a static list in the built-in harness
definition. That list becomes stale as Codex adds, retires, or gates models,
and the current Peon selection has no way to carry Codex's model-specific
reasoning-effort options. The installed Codex CLI already exposes a structured
`app-server` protocol with `model/list`, including each model's supported and
default reasoning efforts.

## Decision

Declare Codex's model capability as a dedicated `codex-app-server` discovery
kind. The Rust sidecar starts the configured Codex command in stdio app-server
mode, performs the initialize/initialized handshake, requests the complete
visible model catalog, and maps the response into OrkWorks model options. A
model option carries its stable model ID, display label, supported reasoning
efforts, and optional default effort.

Peon selection persists an optional model and optional effort. A missing model
or effort means that Codex should choose its own default. Existing persisted
selections with a string model remain valid; providers other than Codex retain
their current model behavior and receive no effort argument.

Model discovery failure does not make Codex unusable: provider verification can
succeed with an empty catalog, allowing the user to apply the Codex default or
enter a model manually. The desktop keeps the last successful catalog in its
process-local cache and marks it stale when a refresh fails.

## Consequences

Codex model freshness follows the installed CLI and account instead of an
OrkWorks release. The sidecar gains a small JSON-RPC subprocess adapter and a
versioned response shape for model metadata. Peon invocation must render
Codex's optional reasoning effort through its `--config`
`model_reasoning_effort=<value>` setting. Discovery remains best-effort and
does not block the default Codex path.
