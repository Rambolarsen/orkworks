# Provider Settings Migration Implementation Plan

> For agentic workers: REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement task-by-task.

**Goal:** Stop default and stale Peon settings from invoking retired Gemini or non-Peon Copilot while preserving intentional Gemini opt-in configuration.

**Architecture:** Electron-main owns durable settings migration by inspecting raw provider entries before normalization loses intent, persisting only repairs, and syncing repaired settings before inference. Rust independently filters every received provider payload against resolved Peon definitions.

**Tech Stack:** Electron main TypeScript, Node test runner, Rust/Axum sidecar unit tests.

## Global Constraints

- Keep electron/ and src/ provider types independently owned; update both.
- Gemini stays explicit opt-in. Do not alter Google authentication or add Antigravity Peon inference.
- Configurable providers are opencode, claude-code, codex, gemini, aider, and ollama only.
- Preserve valid Gemini settings unless their raw persisted entry exactly equals the historical default. Local migration retains revision.

---

### Task 1: Add raw Electron migration and fresh defaults

**Files:**

- Modify: apps/desktop/electron/providerTypes.ts
- Modify: apps/desktop/src/providerTypes.ts
- Modify: apps/desktop/electron/settingsMemory.ts
- Modify: apps/desktop/electron/main.ts
- Test: apps/desktop/tests/electronSettingsMemory.test.ts

**Interfaces:**

- Produce readSettingsWithMigration(userDataPath): { settings: AppSettings; migrated: boolean }.
- Produce loadSettingsForStartup(userDataPath): AppSettings, writing only when migrated.
- Retain readSettings() as a non-mutating wrapper returning the settings value.

- [ ] **Step 1: Write failing migration tests**

Add a test that writes persisted provider entries containing:

    { id: "gemini", enabled: true, fallbackOrder: 3, defaultState: "unknown", overrideState: null }
    { id: "gh-copilot", enabled: true, fallbackOrder: 5, defaultState: "unknown", overrideState: null }

Call loadSettingsForStartup(tempDir). Assert: revision is unchanged; Gemini is disabled; gh-copilot is absent in the returned settings and the serialized settings.json. Add a second test whose Gemini entry differs only by fallbackOrder: it remains enabled and the raw file is unchanged. Update fresh-default and write canonicalization assertions to require six IDs, disabled Gemini, and no gh-copilot.

- [ ] **Step 2: Verify RED**

Run: cd apps/desktop && node --experimental-strip-types --test tests/electronSettingsMemory.test.ts

Expected: FAIL because loadSettingsForStartup does not exist and defaults retain enabled Gemini and gh-copilot.

- [ ] **Step 3: Implement the minimum migration**

In both providerTypes.ts files define ProviderId as the six Peon-capable IDs listed in Global Constraints. Remove gh-copilot from VALID_PROVIDER_IDS and defaults and set the Gemini default enabled field false.

Before normalizeProviderEntry runs, add migrateRawProviderSettings(value), which clones only the providers payload, removes raw gh-copilot entries, and changes enabled to false only for an exact match of:

    { id: "gemini", enabled: true, fallbackOrder: 3, defaultState: "unknown", overrideState: null }

Do not sort, renumber, or change any other raw entry in this helper. Make readSettingsWithMigration parse once, run the helper, normalize its output, and expose its migration flag. loadSettingsForStartup writes normalized settings only when that flag is true. In main.ts initialize currentSettings with loadSettingsForStartup(app.getPath("userData")) before startSidecar() and syncSavedProviderSettings().

- [ ] **Step 4: Verify GREEN and commit**

Run: cd apps/desktop && node --experimental-strip-types --test tests/electronSettingsMemory.test.ts

Expected: PASS, covering persisted migration, revision preservation, idempotence, and preserved custom Gemini.

Run:

    git add apps/desktop/electron/providerTypes.ts apps/desktop/src/providerTypes.ts apps/desktop/electron/settingsMemory.ts apps/desktop/electron/main.ts apps/desktop/tests/electronSettingsMemory.test.ts
    git commit -m "fix: migrate retired Peon providers"

### Task 2: Filter non-Peon provider payloads in the sidecar

**Files:**

- Modify: crates/orkworksd/src/providers.rs
- Test: crates/orkworksd/src/providers.rs provider-manager tests

**Interfaces:**

- Consume ProviderSettingsPayload through POST /settings/providers.
- Produce apply_settings() that stores only IDs returned by self.definitions().

- [ ] **Step 1: Write the failing production-catalog test**

Using ProviderManager::for_tests with a fake opencode provider, construct a payload containing opencode, gh-copilot, and antigravity. Call apply_settings(payload), then assert get_providers_response returns only opencode. Run inference and assert its only attempt is opencode. Do not add a fake definition for Copilot or Antigravity.

- [ ] **Step 2: Verify RED**

Run: cargo test --manifest-path crates/orkworksd/Cargo.toml providers::tests::apply_settings_discards_legacy_and_non_peon_provider_ids -- --exact

Expected: FAIL because apply_settings currently stores every supplied entry.

- [ ] **Step 3: Implement and verify GREEN**

Change apply_settings to accept a mutable payload, construct HashSet<String> from self.definitions().into_iter().map(|definition| definition.id), and retain only entries whose id is present before it records revision and replaces stored settings. Retain existing applied-revision and response behavior.

Run: cargo test --manifest-path crates/orkworksd/Cargo.toml providers -- --nocapture

Expected: PASS; legacy Copilot, Antigravity, and arbitrary unknown IDs cannot reach fallback execution or /providers.

- [ ] **Step 4: Commit**

Run:

    git add crates/orkworksd/src/providers.rs
    git commit -m "fix: reject non-Peon provider settings"

### Task 3: Verify the complete cross-component change

- [ ] **Step 1: Type-check and test desktop**

Run: cd apps/desktop && npx tsc --noEmit

Expected: PASS.

Run: cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs

Expected: PASS.

- [ ] **Step 2: Test the sidecar**

Run: cargo test --manifest-path crates/orkworksd/Cargo.toml

Expected: PASS.

- [ ] **Step 3: Run required currency checks**

Run: bash .claude/hooks/doc-check.sh

Expected: no unaddressed documentation trigger.

Run: bash .claude/hooks/worktree-check.sh

Expected: review fleet-wide flags; act only on this worktree.
