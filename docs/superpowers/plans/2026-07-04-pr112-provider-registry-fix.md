# PR112 Provider Registry Fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ensure Peon provider definitions are derived from the loaded harness configuration, including disk overrides and custom harnesses with `peon` config.

**Architecture:** `ProviderManager` should build its registry from a caller-supplied harness config slice. `main` should load harnesses once, use that same vector for both `AppState.harnesses` and `ProviderManager`, and retain the existing explicit registry injection for tests.

**Tech Stack:** Rust, Axum sidecar, existing `cargo test --manifest-path crates/orkworksd/Cargo.toml` workflow.

## Global Constraints

- Keep the fix scoped to `crates/orkworksd`.
- Do not change frontend provider settings types in this patch.
- Use TDD: add the failing Rust test before production code.
- Preserve existing `ProviderManager::for_tests_with_registry` injection for isolated provider tests.

---

### Task 1: Derive Provider Registry From Loaded Harness Configs

**Files:**
- Modify: `crates/orkworksd/src/providers.rs`
- Modify: `crates/orkworksd/src/main.rs`

**Interfaces:**
- Produces: `ProviderManager::new_with_harnesses(harnesses: &[harness_registry::HarnessConfig]) -> ProviderManager`
- Consumes: `harness_registry::HarnessConfig.peon`

- [x] **Step 1: Write the failing test**

Add a test in `crates/orkworksd/src/providers.rs` that constructs a custom `HarnessConfig` with `peon: Some(HarnessPeonConfig { ... })`, creates a `ProviderManager` from that harness list, and asserts `list_models("custom-ai")` returns the static models from the custom harness.

- [x] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml provider_manager_uses_supplied_harness_peon_configs`

Expected: FAIL because the constructor does not yet accept supplied harness configs or still ignores custom harnesses.

- [x] **Step 3: Write minimal implementation**

Change provider derivation to accept `&[HarnessConfig]`, add `ProviderManager::new_with_harnesses`, keep `ProviderManager::new` as a built-in default wrapper for existing tests, and update `main` to call `load_harnesses()` once before constructing `AppState`.

- [x] **Step 4: Run focused tests**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml provider_manager_uses_supplied_harness_peon_configs`

Expected: PASS.

- [x] **Step 5: Run Rust suite**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml`

Expected: PASS.
