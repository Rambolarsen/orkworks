# Taskmaster Inference Definitions Implementation Plan

> **For agentic workers:** Use executing-plans inline for this sequential slice. Root is the sole writer; the owner requested design review, patching, then implementation. Do not dispatch implementation workers.

**Goal:** Accept, validate, edit, and persist custom inference definitions without making their executables runnable.

**Architecture:** Add a validated value type to the existing harness definition and sparse override boundary. Keep provider projection and execution unchanged until explicit trust and lifecycle integration are implemented.

**Tech Stack:** Rust/serde, existing JSON store, TypeScript renderer validation, native test runners.

**Spec:** [Reviewed adapter design](../specs/2026-09-10-custom-inference-adapters-design.md)

## Global constraints

- Input is stdin or file; output is result-json-v1; no shell-string evaluation.
- Timeout 1–120 seconds, default 60; at most 64 combined argv templates, each at most 4 KiB.
- Command is a bare executable name or absolute path; no control characters or command placeholders.
- Exactly one model placeholder in base args; effort occurs only once in a nonempty optional reasoningEffortArgs list; promptFile occurs exactly once in base args only for file input.
- Preserve schema-3 documents; schema-2 documents containing inference require an explicit version update. Existing v2 migration is unchanged.
- JSON cannot select builtin bindings or grant trust. New definitions remain inert.
- Existing launch/resume/session-ID/voice/capacity/reset behavior is unchanged. This is a capability schema extension, not a new compiled harness adapter.
- Review routing: two read-only design reviewers, correctness and coverage, one round; root reconciles and owns edits. No commits/pushes/PR/publication without the human gate.

## Task 1: Rust definition and persistence boundary

**Files:** create `crates/orkworksd/src/harness/inference.rs`; modify `harness.rs`, `harness/definition.rs`, `harness/store.rs`; tests in the latter definition/store modules.

**Interfaces:** consume `parse_custom_definition`, `HarnessDefinition::apply_patch`, `HarnessStore::mutate`, and `resolve_document`. Produce `InferenceCapability`, a validated serializable command capability held as `Option<InferenceCapability>` and replaced/cleared by `Option<Option<InferenceCapability>>` patches.

- [ ] Add a failing definition test parsing an inference-only (no Peon) custom harness and asserting the serialized capability survives, including the default timeout:

```rust
let raw = br#"{"id":"custom-infer","name":"Custom","launch":{"kind":"command-template","command":"tool","args":[],"modelPrefix":null},"inference":{"kind":"command","command":"custom-infer","args":["--model","{model}"],"input":"stdin","output":"result-json-v1"}}"#;
let parsed = parse_custom_definition(raw).unwrap();
assert_eq!(serde_json::to_value(parsed).unwrap()["inference"]["timeoutSecs"], 60);
```

- [ ] Run `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml inference_definition -- --test-threads=4`; confirm unknown-field rejection is the initial failure.
- [ ] Implement strict deserialization through a validated raw struct (`deny_unknown_fields`), immutable private fields, a closed command kind, input/output enums, and placeholder validation. Register the optional field in every custom/patch wire layer and legacy constructors. Serialize absent inference fields by omission.
- [ ] Add tests rejecting partial patches, builtin/trust fields, invalid counts/braces/input/output/timeouts, relative paths, controls, excessive UTF-8 bytes, duplicate keys, and schema-2 inference. Test null clear, absent preserve, and launch-kind replacement with inference in the same patch.
- [ ] Reproduce the launch-kind early-return bug with that test, then let patch application continue through all fields before validation.
- [ ] Persist a custom definition through `StoreFixture::v2()` and `store.mutate`; reload with `store.load`, assert the inference JSON is preserved and the custom no-Peon definition is absent from runnable provider projection.
- [ ] Run all harness tests and Rust build/format checks; preserve unrelated edits.

## Task 2: Existing JSON editor compatibility

**Files:** modify `apps/desktop/src/harnessTypes.ts`; create focused `apps/desktop/src/inferenceDefinition.ts` and `apps/desktop/tests/inferenceDefinition.test.ts` if needed to avoid expanding the existing validator further.

**Interfaces:** consume `parseHarnessDraft(text, mode)` and return its existing diagnostic shape; add the optional inference type without importing Electron code.

- [ ] Add a failing test through the actual editor parser:

```typescript
const parsed = parseHarnessDraft(JSON.stringify({
  inference: { kind: "command", command: "custom-infer", args: ["--model", "{model}"], input: "stdin", output: "result-json-v1" },
}), "override");
assert.deepEqual(parsed.diagnostics, []);
```

- [ ] Run `rtk node --experimental-strip-types --test tests/inferenceDefinition.test.ts` from `apps/desktop`; confirm the existing unknown-field failure.
- [ ] Add complete/patch allowlist entries and a dedicated validator for the same schema. Require complete inference objects even in override mode; accept null and omission. Keep backend validation authoritative.
- [ ] Test valid stdin/file definitions, malformed/duplicate/unknown fields, placeholder placement, UTF-8 limits, and preserved editor output using literal fixtures.
- [ ] Run desktop tests/typecheck and docs build, then the repository verification helper. Record exact results and remaining activation work; do not claim custom adapters are runnable.

## Remaining full-feature work (not delivered by this slice)

Trust persistence/privileged approval UI; custom runner and result decoder; provenance-aware provider projection without Peon; static/free-text model selection; adapter/trust cache identity and atomic acceptance; built-in capability migration; process/lifecycle regression tests. Issue #503 remains open.

## Execution record — 2026-09-10

Tasks 1 and 2 are implemented and verified. The checklists above preserve the original test-first procedure; this execution record records their completion.

- Two independent read-only design reviews completed before implementation. Findings were incorporated into the design, authoritative knowledge spec, and ADR 0055.
- Initial Rust and editor tests failed on the unknown inference field. Additional tests reproduced empty-command acceptance, schema-2 acceptance, and the launch-kind patch early return before their fixes.
- Strict immutable Rust validation, editor validation, replacement/clear semantics, schema-version checks, and actual store persistence are implemented. Store tests confirm a custom inference-only definition is not projected as a runnable provider. The editor's derived-field stripping preserves inference.
- Focused verification: 209 harness tests passed; desktop type-check and the three new editor tests passed.
- Full verification: `RUST_TEST_THREADS=4 bash scripts/verify-repo.sh` exited 0. Rust: 1,091 unit tests passed, one ignored, plus three integration/script tests passed. Desktop: 687 tests passed. Rust format/build, desktop type-check/build, docs build, diff checks, documentation currency, and worktree currency passed.
- No provider was invoked, credentials accessed, custom executable launched, or trust granted. No commit, push, or PR was created. Activation and its security/lifecycle verification remain pending under issue #503.
