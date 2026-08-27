# Task 1 report

## Status

Task 1 is complete in this worktree.

The required Rust settings contract, normalization helpers, resolver tests,
and the authorized mechanical `model: None` compile-site additions are present
and verified. During this session, no additional source-code edits were needed;
the worktree already matched the brief, so this report was refreshed against
fresh verification.

## Requirements coverage

### `crates/orkworksd/src/providers.rs`

- `ProviderSettingsEntry` includes `model: Option<String>` with
  `#[serde(default)]` for backward-compatible deserialization.
- Model normalization trims non-empty values and converts empty or
  whitespace-only strings to `None`.
- `resolve_provider_model(entry, global_model)` prefers the per-provider model,
  then the global `peon_model`, then no model.
- `ProviderManager::apply_settings` normalizes both the global model and each
  provider entry model before publishing settings.
- Provider tests cover:
  - deserializing old payloads without `model`
  - deserializing explicit model strings without rewriting them on load
  - resolver precedence and whitespace handling
  - `apply_settings` trimming and whitespace-to-`None` normalization

### Authorized mechanical compile-site updates

The requested `model: None` literal additions are present, with no behavior
change, in:

- `crates/orkworksd/src/http/session_handlers.rs`
- `crates/orkworksd/src/runtime/peon_runtime.rs`
- `crates/orkworksd/src/runtime/terminal_runtime.rs`

## Verification run on August 25, 2026

### Focused Rust tests

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml providers::tests -- --nocapture
```

Observed result: `27 passed, 0 failed, 794 filtered out`.

### Repo closeout checks

```bash
bash .claude/hooks/doc-check.sh
bash .claude/hooks/worktree-check.sh
```

Observed result: both commands exited cleanly with no output.

## Self-review

- Confirmed the worktree is clean after verification.
- Confirmed the authorized compile-site additions are present at the user-named
  locations.
- Confirmed the report now reflects the current verified state instead of stale
  commit metadata.

## Concerns

- The focused provider test command is green. No Task 1 functional concerns
  remain from this session.

## Fix

### Files

- `crates/orkworksd/src/providers.rs` — resolve the entry model override once
  per inference attempt, reuse it for CLI arguments and `ProviderObservation`,
  and pass it explicitly through `ProviderRunner`, `CompositeRunner`,
  `ProcessRunner`, `HttpRunner`, and `FakeRunner`.
- `crates/orkworksd/src/providers.rs` — add behavioral coverage for actual
  invocation/observation, whitespace-to-global fallback, Ollama request model,
  unsupported HTTP providers, and the preserved Ollama no-model error.

### Tests

```text
CARGO_TARGET_DIR=/tmp/orkworks-provider-model-selection-target cargo test --manifest-path crates/orkworksd/Cargo.toml --quiet
PASS: 826 tests, 0 failed
```

The focused provider suite also passed: `32 passed, 0 failed`. Rust emitted
only the two pre-existing warnings noted by the prior report.

### Commit

- `15919b7` — `fix: honor per-provider Peon model selection`

### Concerns

- No model-selection concerns remain. Full-suite verification used loopback
  access for the Ollama behavioral test and a temporary Cargo target directory
  because the worktree's existing target lock is not writable in the sandbox.

## Review fix

### Changes

- Electron Ollama URL normalization now removes multiple trailing slashes
  before URL parsing, matching Rust normalization for values such as
  \`http://localhost:11434//\`.
- Electron and Rust tests cover the multiple-trailing-slash case.
- The provider-settings migration preservation test now asserts an unrelated
  Ollama provider entry survives in memory and on disk, including its model,
  enabled, default capacity, and override capacity fields.

### Verification

\`\`\`text
node --experimental-strip-types --test tests/electronSettingsMemory.test.ts
PASS: 37 tests, 0 failed

cargo test --manifest-path crates/orkworksd/Cargo.toml providers::tests -- --nocapture
PASS: 43 tests, 0 failed
\`\`\`

The full Electron test command reached 465 passing tests but had one unrelated
existing \`tests/terminalLinks.test.ts\` module-resolution failure. The full
Rust suite reached all 866 tests but was stopped after existing runtime tests
continued running without completion for more than two minutes; no failure was
reported before stopping it.

### Commit

- Pending commit for this review-fix pass.
