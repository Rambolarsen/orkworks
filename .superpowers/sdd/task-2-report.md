# Task 2 — Peon provider verification and staged Apply review fixes

## Scope delivered

This Task 2 follow-up fixes the provider verification review findings without
expanding the provider-first selection flow:

- `crates/orkworksd/src/providers.rs`
  - `verify_provider` now verifies connectivity/default-model execution and
    returns capability metadata even when a provider cannot deliver an
    explicitly selected model.
  - `test_and_apply` remains the only operation that rejects unsupported
    explicit model delivery, before invoking or mutating applied state.
  - Invalid inference JSON is classified as `ModelFailure` only after a fresh
    generation check, so a superseding request returns `StaleGeneration`.
  - Added deterministic coverage for that stale invalid-JSON race.
  - Added a Unix integration test that runs the real `ProcessRunner` and
    asserts the child argv contains `--model=manual-model`.
- `crates/orkworksd/src/main.rs`
  - Added a successful HTTP route test covering provider verification, staged
    Apply, applied-state retrieval, JSON serialization, and the normalized
    Ollama URL.
  - Existing route coverage continues to assert structured error serialization
    and HTTP status mapping for verification-required and malformed requests.

No desktop, protocol, or unrelated provider behavior was changed.

## Review-finding coverage

1. Default-only providers now return `ok`, `provider`, `models`, generation,
   and capability metadata from verification. Their `testInference` capability
   is false when explicit model delivery is unavailable; explicit Apply returns
   `unsupported_capability`.
2. The invalid-JSON ModelFailure branch re-checks the operation generation. The
   test hook advances the generation at the exact pre-parse boundary, making
   the expected stale result deterministic rather than sleep-based.
3. The manual-model test uses `ProcessRunner` with `sh` and captures the
   actual child positional argument, proving the rendered model flag reaches
   the process argv.
4. The route test sends real HTTP requests in order: verify, test-and-apply,
   then applied-state GET. It asserts camelCase capability/applied fields and
   the serialized response equality.

## Commands and results

The linked checkout's default Cargo target directory was not writable. The
focused test commands therefore used an isolated target under `/private/tmp`.
The route tests also require loopback binding; the first sandboxed attempt was
denied at `TcpListener::bind`, and the same focused command passed with local
network permission.

### Focused provider regressions

```bash
CARGO_TARGET_DIR=/private/tmp/orkworks-peon-provider-first-selection-target-1 rtk cargo test --manifest-path crates/orkworksd/Cargo.toml providers::tests::default_only_provider_verification_returns_capabilities_without_apply_support -- --nocapture
```

Result: **PASS** — `1 passed, 891 filtered out`.

```bash
CARGO_TARGET_DIR=/private/tmp/orkworks-peon-provider-first-selection-target-1 rtk cargo test --manifest-path crates/orkworksd/Cargo.toml providers::tests::stale_generation_wins_over_invalid_inference_failure -- --nocapture
```

Result: **PASS** — `1 passed, 891 filtered out`.

```bash
CARGO_TARGET_DIR=/private/tmp/orkworks-peon-provider-first-selection-target-1 rtk cargo test --manifest-path crates/orkworksd/Cargo.toml providers::tests::test_and_apply_passes_manual_model_in_real_process_argv -- --nocapture
```

Result: **PASS** — `1 passed, 891 filtered out`.

```bash
CARGO_TARGET_DIR=/private/tmp/orkworks-peon-provider-first-selection-target-1 rtk cargo test --manifest-path crates/orkworksd/Cargo.toml providers::tests:: -- --nocapture
```

Result: **PASS** — `67 passed, 825 filtered out`.

### Endpoint coverage

```bash
CARGO_TARGET_DIR=/private/tmp/orkworks-peon-provider-first-selection-target-1 rtk cargo test --manifest-path crates/orkworksd/Cargo.toml http::provider_handlers::tests:: -- --nocapture
```

Result: **PASS** — `2 passed, 890 filtered out`.

```bash
CARGO_TARGET_DIR=/private/tmp/orkworks-peon-provider-first-selection-target-1 rtk cargo test --manifest-path crates/orkworksd/Cargo.toml peon_provider_routes_ -- --nocapture
```

Result: **PASS** — `2 passed, 890 filtered out`.

```bash
CARGO_TARGET_DIR=/private/tmp/orkworks-peon-provider-first-selection-target-1 rtk cargo test --manifest-path crates/orkworksd/Cargo.toml peon_provider_routes_verify_apply_and_serialize_applied_state -- --nocapture
```

Result: **PASS** with local-network permission — `1 passed, 891 filtered out`.

The first attempt at that exact route test without local-network permission
failed before running the test with:
`Operation not permitted` at `std::net::TcpListener::bind("127.0.0.1:0")`.

### Repository checks

```bash
rtk git diff --check
```

Result: **PASS** — no whitespace errors.

```bash
bash scripts/doc-check.sh
bash .claude/hooks/worktree-check.sh
```

Result: both exited **0** with no findings.

```bash
rtk cargo fmt --manifest-path crates/orkworksd/Cargo.toml -- --check
```

Result: **FAIL** on pre-existing formatting differences across unrelated Rust
modules, including harness, runtime, Taskmaster, and workflow-observation
code. No unrelated formatting was applied.

## Limitations and concerns

- The real-argv test is Unix-only because it uses `sh`; the production
  invocation path remains platform-neutral and the existing Windows runner
  behavior is unchanged.
- This follow-up ran the focused provider and endpoint suites requested here,
  not the full Rust or desktop suites.
- Repository-wide formatting remains red for unrelated pre-existing diffs;
  `git diff --check` is clean for this patch.

## Current Task 2 Report — active coding tool hook toggle

### Files

- Added `apps/desktop/electron/activeHarnessIntegration.ts`
- Added `apps/desktop/tests/activeHarnessSave.test.ts`
- Modified `apps/desktop/electron/main.ts`
- Modified `apps/desktop/src/App.tsx`
- Modified `apps/desktop/src/components/SettingsModal.tsx`
- Modified `apps/desktop/tests/api.test.ts`
- Modified `docs/agents/architecture.md`

### Tests

```bash
cd apps/desktop
node --experimental-strip-types --test tests/activeHarnessSave.test.ts tests/api.test.ts
npx tsc --noEmit
cd ../..
bash scripts/doc-check.sh
bash .claude/hooks/worktree-check.sh
git diff --check
```

Results:

- Focused Task 2 desktop tests: PASS
- TypeScript: PASS
- `doc-check`: PASS after updating `docs/agents/architecture.md`
- `worktree-check`: PASS
- `git diff --check`: PASS

### Concerns

- `SettingsModal.tsx` now stops showing the unconditional `"Saved"` status for coding-tool saves, but the richer per-tool partial-failure UI still belongs to later settings tasks in the active plan.
- The stale-workspace guard is implemented entirely in Electron main by comparing the captured workspace path plus a main-owned sidecar generation token before mutations and after the batch; no Rust route changes were needed for this task because the sidecar integration handlers already revalidate workspace identity per request.
