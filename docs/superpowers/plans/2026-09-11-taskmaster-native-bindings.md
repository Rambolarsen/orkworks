# Taskmaster native bindings implementation plan

> **For agentic workers:** Use executing-plans; root owns all edits.

**Goal:** Remove mutable Peon provider lookup from native Taskmaster dispatch.

**Architecture:** A closed code-owned profile enum resolves only eligible native
catalog entries. The native evaluation binding includes profile plus document
revision. Invocation builds a fixed definition for that profile and reuses the
existing CLI/HTTP transports without changing their policy or login handling.

**Tech Stack:** Rust, serde serialization only, existing runner and fixture tests.

**Spec:** [accepted adapter design](../specs/2026-09-10-custom-inference-adapters-design.md),
[ADR 0055](../../adr/0055-json-taskmaster-inference-adapters.md), issue #503.

## Constraints

- Code-owned Codex, Claude and Ollama only; JSON cannot deserialize native profiles.
- Custom wrappers keep their explicit trust path; no provider fallback.
- Retain native argv, login/configuration handling, version checks and decoders.
- Peon settings, native CLI policy and installed credentials remain unchanged.
- No real providers or GitHub writes; custom scheduling remains inactive.
- Root sole writer; two read-only reviewers, one round; no commit/push/merge.

## Task 1: Native profile binding

Files: new `crates/orkworksd/src/providers/native_inference.rs`, `providers.rs`,
`taskmaster/provider_catalog.rs`, `taskmaster/runtime/inference.rs`, `evaluator.rs`.

- [x] Add origin/override tests for closed native resolution and refusal of
  custom definitions, explicit inference clears and execution-affecting overrides.
- [x] Add a runner fixture test: mutate a manager's Peon command/args/timeout,
  invoke the captured code-owned profile, and assert only the fixed CLI command
  and inference arguments are executed. Observe missing native dispatch API.
- [x] Implement serializable (not deserializable) native enum and pure resolver.
  Fixed CLI definitions retain the existing 30-second native timeout; Ollama
  retains its existing code-owned HTTP definition and selected endpoint.
- [x] Carry profile in native snapshot binding/cache key. Reject unsupported
  transport kinds explicitly. Dispatch via the bound profile, not a provider ID.
- [x] Preserve legacy invocation helper only for existing test characterization;
  native production dispatch uses the new typed entry point.

## Task 2: Verification and checkpoint

- [x] Run Taskmaster/provider tests and repository verification.
- [x] Correctness reviewer checks authority/origin and dispatch data flow;
  coverage reviewer checks missing mutations and native behavior regressions.
- [x] Patch verified findings, document results and remaining activation/Windows
  work locally. No extra review round or external publication.

## Execution and review record

Initial runner/resolver tests exposed the missing native enum and typed dispatch
interface. The implementation now binds Codex, Claude or Ollama plus document
revision and constructs fixed inference definitions without reading mutable Peon
registry entries. Custom availability remains execution-inactive.

The correctness review found no confirmed defect in this slice. Its Ollama-ID
shadowing concern preserves existing behavior: a custom same-ID entry follows
custom trust/readiness, never native fallback. This is now documented; reserving
new IDs would be a separate policy change.

Coverage review led to conflicting Peon provider/model/effort/endpoint sentinels,
retired native resolver/catalog coverage, and routing the isolated real-process
Codex/Claude login-preservation fixture through the production typed entry point.
Ollama dispatch is checked with an empty Peon registry and an explicit endpoint.
The proposed negative deserialization test was not added: the enum has no
Deserialize implementation and no JSON input route; adding a compile-fail harness
solely to assert an absent derive is outside this bounded slice.

Focused Taskmaster tests passed (94 passed, two ignored helpers). The first full
verification with default test concurrency failed three one-second process
fixtures: model discovery timed out, closed-stdin returned timeout rather than
broken pipe, and a child did not create its PID file before timeout. The remaining
failure passed alone. Full verification with the previous checkpoint's
`RUST_TEST_THREADS=4` setting then passed, including all 695 desktop tests and the
application/docs builds. This supports timing sensitivity, not a proven underlying
fix; no production timeout or test deadline was changed.

Post-review provider verification passed (101 passed, one ignored helper),
including typed Codex/Claude process fixtures and all new native tests. Native
Windows validation and custom scheduling activation coverage remain pending. No real
providers, credentials, user settings, commits or external issue writes were used.
