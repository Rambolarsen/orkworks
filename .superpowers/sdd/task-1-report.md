# Task 1 report

## Status

Completed after the authorized scope expansion to the compile-site
`ProviderSettingsEntry` literals.

## Files changed

- `crates/orkworksd/src/providers.rs`
- `crates/orkworksd/src/http/session_handlers.rs`
- `crates/orkworksd/src/runtime/peon_runtime.rs`
- `crates/orkworksd/src/runtime/terminal_runtime.rs`
- `.superpowers/sdd/task-1-report.md`

## Implementation summary

### `crates/orkworksd/src/providers.rs`

- Added `ProviderSettingsEntry::model: Option<String>` with `#[serde(default)]`
  for backward-compatible deserialization.
- Added shared model normalization helpers that trim non-empty values and
  convert empty/whitespace-only strings to `None`.
- Added `resolve_provider_model(entry, global_model)` with the required
  precedence:
  - entry override
  - global `peon_model`
  - no model
- Normalized both top-level `peon_model` and per-entry `model` values inside
  `ProviderManager::apply_settings`.
- Extended the provider tests to cover:
  - old payloads without `model`
  - explicit `model: "  llama3  "` deserialization
  - resolver precedence
  - trimming/whitespace normalization in `apply_settings`
- Updated the two existing `ProviderSettingsEntry` test literals in this file to
  include `model: None`.

### Mechanical compile-site updates

Added `model: None` only, with no behavior change, to the authorized
`ProviderSettingsEntry` literals in:

- `crates/orkworksd/src/http/session_handlers.rs`
- `crates/orkworksd/src/runtime/peon_runtime.rs`
- `crates/orkworksd/src/runtime/terminal_runtime.rs`

## Focused test command

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml providers::tests -- --nocapture
```

## Focused test output

```text
warning: function `with_fake_home` is never used
warning: unused return value of `into_response` that must be used
warning: `orkworksd` (bin "orkworksd" test) generated 2 warnings
Finished `test` profile [unoptimized + debuginfo] target(s) in 14.11s
Running unittests src/main.rs (crates/orkworksd/target/debug/deps/orkworksd-7ec3f49f5134a4cf)

running 27 tests
...
test providers::tests::apply_settings_trims_provider_and_global_models_and_clears_whitespace ... ok
...
test providers::tests::resolve_provider_model_prefers_entry_then_global_then_none ... ok
test providers::tests::provider_settings_entry_deserializes_missing_model_as_none ... ok
test providers::tests::provider_settings_entry_deserializes_explicit_model_string ... ok
...
test result: ok. 27 passed; 0 failed; 0 ignored; 0 measured; 794 filtered out; finished in 0.01s
```

## Self-review

- Verified the only non-`providers.rs` source edits are mechanical `model: None`
  additions to the newly authorized compile sites.
- Verified the new behavior remains confined to `providers.rs`.
- Verified the focused Rust test target passes after the scope expansion.

## Commits

- Rust settings contract commit: `481c0f4e75bbea8092e1fea77e29f1ce52427d38` (`feat: add per-provider Peon model settings`)
- Report commit: pending

## Concerns

- The focused test command still emits two pre-existing warnings unrelated to
  Task 1:
  - unused helper `with_fake_home` in `src/main.rs`
  - ignored `into_response()` result in `src/http/session_handlers.rs`
