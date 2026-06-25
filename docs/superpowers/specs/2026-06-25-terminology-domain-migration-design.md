# OrkWorks Terminology and Domain Boundary Migration Design

## Summary

Migrate OrkWorks terminology so product copy is understandable to end users while the codebase keeps a clear technical boundary between coding tools, model providers, models, and running sessions.

This migration explicitly excludes launch profiles for now. The change focuses on terminology, schema compatibility, UI copy, and documentation without changing how sessions are started or how recommendations are scored.

## Goals

- Use `Coding tool` in user-facing UI for Claude Code, Codex CLI, OpenCode, Gemini CLI, Aider, and similar applications.
- Keep `Harness` as the canonical internal abstraction for coding-tool integrations.
- Use `Model provider` only for inference services or local inference runtimes.
- Keep running work represented as `Agent session` in product copy where the distinction matters, while allowing `Session` in code where the module context is already clear.
- Preserve compatibility with existing persisted session and capacity metadata.
- Stabilize overlapping in-flight desktop terminology changes before broadening the migration.

## Non-goals

- No launch-profile implementation or management UI.
- No PTY/session execution redesign.
- No new harness adapters or model-provider integrations.
- No recommendation algorithm changes beyond terminology fixes.
- No Git workflow control changes.

## Current Problems

The desktop app currently conflates at least three concepts:

- `provider` is used both for coding tools and for inference services
- `harness` appears in some internal APIs, but user-facing copy still labels the same object as `Provider`
- session details and settings surfaces mix provider/model/coding-tool language in ways that make the UI harder to understand

This ambiguity leaks into UI labels, frontend types, Electron settings and IPC contracts, API field names, persisted session metadata, documentation, and examples.

## Terminology Boundary

### Coding tool

Users should see `Coding tool`, not `Harness`, when selecting or inspecting applications such as Claude Code, Codex CLI, OpenCode, Gemini CLI, Aider, and future CLI coding applications.

Internal code may continue to use `Harness` for this same concept because it is the integration abstraction: command construction, resume strategy, executable availability, and per-tool launch behavior.

Examples:

```text
Coding tool: OpenCode
Coding tool availability
Coding tool configuration
```

### Harness

`Harness` remains an internal term. It is acceptable in Rust modules, TypeScript types, IPC payloads, and config where the audience is maintainers or compatibility requires the existing name.

Do not replace internal `HarnessConfig`, `HarnessDefinition`, or `harness` wire fields as part of this migration unless a compatibility alias is required for a user-facing boundary.

### Model provider

Use `Model provider` only for the service or runtime supplying model inference. Examples include Anthropic, OpenAI, OpenRouter, Google, Azure OpenAI, and Ollama.

Provider settings and provider state should not describe CLI coding tools. They describe inference availability, fallback, model listing, and Peon inference routing.

### Agent session

Use `Agent session` where product copy needs to distinguish a running unit of work from a generic UI session or app session. Keep `Session` where the surrounding UI already makes the context clear, especially dense panels such as the sessions list.

## UI Copy Migration

Replace user-visible occurrences of `Harness` with `Coding tool` where the value represents Claude Code, Codex CLI, OpenCode, Gemini CLI, Aider, or similar tools.

Preferred replacements:

| Existing copy | New copy |
| --- | --- |
| Harness | Coding tool |
| Harness configuration | Coding tool configuration |
| Harness availability | Coding tool availability |
| Harness/model | Coding tool / model |
| Provider, when referring to a CLI coding app | Coding tool |
| Provider, when referring to inference services | Model provider |

Session detail and launch surfaces should distinguish these concepts when values are known:

```text
Coding tool:
Model:
Model provider:
Provider state:
```

## Data Compatibility

Persisted metadata must keep loading existing fields. Existing session metadata and capacity files may contain `harness`, `provider`, and provider state fields.

The migration may add compatibility aliases at API or DTO boundaries, but it must not break existing persisted metadata. Canonical writes should use the smallest set of new field names needed for clarity, and any alias must be covered by tests.

This design does not require a one-time metadata rewrite. If a future metadata migration becomes necessary, it should be planned separately.

## Implementation Strategy

1. Inventory current `harness`, `provider`, `model`, and `session` user-facing strings and API fields.
2. Add characterization coverage before broader renames so current behavior is locked down.
3. Update UI copy to `Coding tool` and `Agent session` phrasing where appropriate.
4. Tighten provider wording so inference services are consistently represented as `Model provider`.
5. Add compatibility aliases only where UI or API terminology changes would otherwise break existing settings or metadata.
6. Update README, AGENTS.md, and relevant architecture docs with the terminology boundary.
7. Verify execution behavior is unchanged.

## Testing

Testing should prove that this migration changes language and compatibility only:

- UI text tests assert selectors display `Coding tool` where the user chooses or inspects CLI coding applications.
- Settings and IPC tests prove existing provider and harness settings still load.
- API tests prove existing `harness` and provider metadata still round-trip.
- Session sort and terminal launch behavior remain unchanged.
- Rust tests prove persisted metadata compatibility if any backend field aliases are added.

## Acceptance Criteria

- Standard product screens say `Coding tool` for CLI coding applications.
- Internal code continues to use `Harness` for the integration abstraction.
- Inference services are consistently represented as `Model provider`.
- Running work is called `Agent session` where disambiguation matters.
- Existing metadata/config still load.
- Canonical writes use any new field names introduced by the migration.
- No launch-profile object, endpoint, or management UI is introduced.
- No PTY/session execution behavior changes.
