# Local Model Providers: Ollama-First Design

- Date: 2026-06-23
- Status: approved

## Summary

Add a roadmap issue for local-model providers, then implement an Ollama-first child issue that prioritizes cheap local Peon inference. The first implementation should also allow Ollama-backed model strings to pass through existing session harnesses that already accept arbitrary model names, without introducing a standalone Ollama harness/runtime.

## Problem

The current provider model is oriented around hosted or CLI-backed providers already present in the built-in registry. The MVP spec explicitly calls out a "local model option where available", but there is no concrete issue that turns that direction into deliverable work.

The immediate product need is to let Peon run against a cheap Ollama model, reducing cost for session observation. Longer-term, OrkWorks should also let users use Ollama-backed models in session flows where an existing harness already supports arbitrary model strings.

## Goals

- Record the approved parent GitHub issue for local-model providers
- Record the first implementation issue for Ollama
- Prioritize cheap local Peon inference
- Support local or remote Ollama endpoints through a configurable base URL
- Include a user-facing settings surface for Ollama URL and model selection
- Allow generic model pass-through in existing harness flows where supported
- Keep the first issue small enough to ship without a full harness compatibility matrix

## Non-Goals

- Auth or TLS support for remote Ollama endpoints
- Taskmaster recommendation policy for preferring cheap local models
- A standalone Ollama session runtime or new harness type
- Guaranteed verified support across every existing harness
- Starting, stopping, or managing the Ollama daemon itself

## Recommended Issue Structure

This design produced two initial issues:

1. Parent issue: [#47 `Local models as providers`](https://github.com/Rambolarsen/orkworks/issues/47)
2. Child implementation issue: [#48 `Add Ollama provider support for Peon and existing harness model pass-through`](https://github.com/Rambolarsen/orkworks/issues/48)

This keeps the roadmap direction visible while allowing the first delivery to stay concrete and implementation-sized.

## Parent Issue Record

### Title

[#47 `Local models as providers`](https://github.com/Rambolarsen/orkworks/issues/47)

### Description

OrkWorks should support local-model backends as provider options, starting with Ollama. This work should cover both cheap local inference for Peon and the ability to use local-model-backed model strings in session flows where existing harnesses already support arbitrary model names.

This issue is the umbrella for the local-model provider direction. It should not try to land every runtime, auth mode, or harness validation path in one change.

### Acceptance Criteria

- [ ] Define the first supported local-model provider target as `Ollama`
- [ ] Add a first implementation issue for Ollama-backed Peon inference
- [ ] Add a first implementation issue for Ollama-backed session model pass-through where existing harnesses support arbitrary model strings
- [ ] Track follow-up issues for remote auth/TLS, recommendation policy, and broader harness compatibility validation
- [ ] Keep standalone Ollama session-runtime support out of scope unless a concrete CLI workflow is defined

### Notes

- Priority is cheap local Peon inference first
- "Full session" support means integration through existing harnesses first, not inventing a new standalone runtime in v1
- Recommendation policy belongs in a separate follow-up issue

## Child Issue Record

### Title

[#48 `Add Ollama provider support for Peon and existing harness model pass-through`](https://github.com/Rambolarsen/orkworks/issues/48)

### Description

Add first-class Ollama support as a local-model provider, with priority on cheap Peon inference. The first version should also allow Ollama-backed model strings to flow through existing harnesses that already accept arbitrary model names, but it should not promise full verified compatibility across every harness.

### Scope

- Configurable Ollama base URL, defaulting to `http://127.0.0.1:11434`
- Model discovery from the configured Ollama endpoint
- User settings for Ollama URL and selected model
- Peon inference via Ollama as part of the provider fallback chain
- Generic session model pass-through for existing harnesses that already support arbitrary model strings
- Validation on 1-2 harnesses only

### Acceptance Criteria

- [ ] Add `ollama` to the provider registry as a Peon-capable provider
- [ ] Add user settings for `Ollama base URL` with default `http://127.0.0.1:11434`
- [ ] Query the configured Ollama endpoint for available models
- [ ] Let the user select an Ollama model from the settings UI
- [ ] Peon can run inference through Ollama and participate in provider fallback order
- [ ] Provider state surfaces useful failures for unreachable endpoint, timeout, empty model list, or missing model
- [ ] Session configuration can pass Ollama-backed model strings through existing harness flows where arbitrary model names are already supported
- [ ] Validate pass-through behavior on 1-2 harnesses and document any remaining gaps
- [ ] Do not add auth/TLS support in this issue
- [ ] Do not add Taskmaster recommendation logic in this issue
- [ ] Do not add a standalone Ollama session runtime in this issue

## Architecture Direction

Ollama should enter the system in two roles:

- Peon provider: a cheap inference backend with enablement, health/state, model list, and fallback position
- Model source for sessions: a model catalog that can be used by existing harnesses when they already accept arbitrary model strings

The first issue should not add a new standalone harness. OrkWorks should continue launching known harness CLIs and treat Ollama as a backend/model source rather than a direct session runtime.

## Follow-Up Issues

- [#49 `Add auth/TLS support for remote Ollama endpoints`](https://github.com/Rambolarsen/orkworks/issues/49)
- [#50 `Teach Taskmaster to prefer cheap local/Ollama models where appropriate`](https://github.com/Rambolarsen/orkworks/issues/50)
- [#51 `Validate Ollama-backed model pass-through across remaining harnesses`](https://github.com/Rambolarsen/orkworks/issues/51)
- [#52 `Consider standalone Ollama runtime support if a concrete session-launch workflow emerges`](https://github.com/Rambolarsen/orkworks/issues/52)

## Risks and Open Questions

- Some harnesses may accept arbitrary model strings in theory but still require provider-specific flags or environment variables in practice
- Ollama model discovery and inference request formats need to be validated against the actual HTTP API during implementation
- Remote endpoints are intentionally unauthenticated in the first issue, so the UX must not imply secure remote operation yet
