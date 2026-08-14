# Provider Settings Migration Implementation Plan

> For agentic workers: REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement task-by-task.

**Goal:** Stop default and stale Peon settings from invoking retired Gemini while preserving canonical GitHub Copilot CLI Peon inference.

**Architecture:** Electron-main owns durable settings migration by inspecting raw provider entries before normalization loses intent, persisting only repairs, and syncing repaired settings before inference. Rust canonicalizes the legacy Copilot ID, independently filters every received provider payload against resolved Peon definitions, and makes prompt transport explicit in the harness Peon capability.

**Tech Stack:** Electron main TypeScript, Node test runner, Rust/Axum sidecar unit tests.

## Global Constraints

- Keep electron/ and src/ provider types independently owned; update both.
- Do not alter Google authentication or add Antigravity Peon inference.
- Configurable providers are opencode, claude-code, codex, aider, copilot, and ollama only.
- Remove persisted Gemini provider entries before normalization and exclude retired harnesses from the sidecar registry. Local migration retains revision.

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

Call loadSettingsForStartup(tempDir). Assert: revision is unchanged; Gemini is absent; the returned settings and serialized settings.json contain canonical `copilot` rather than `gh-copilot`. Add a duplicate test in which saved canonical `copilot` wins over its legacy duplicate. Add a second test whose Gemini entry differs only by fallbackOrder: it is still removed and persisted. Add a write-failure test that keeps repaired settings active. Update fresh-default and write canonicalization assertions to require six IDs and canonical Copilot.

- [ ] **Step 2: Verify RED**

Run: cd apps/desktop && node --experimental-strip-types --test tests/electronSettingsMemory.test.ts

Expected: FAIL because startup migration retains Gemini and throws if its persistence write fails.

- [ ] **Step 3: Implement the minimum migration**

In both providerTypes.ts files define ProviderId as the six Peon-capable IDs listed in Global Constraints. Remove gh-copilot and gemini from VALID_PROVIDER_IDS, add canonical copilot to defaults, and remove Gemini from defaults.

Before normalizeProviderEntry runs, add migrateRawProviderSettings(value), which clones only the providers payload, rewrites raw gh-copilot entries to copilot unless a canonical copilot entry already exists, and removes every raw Gemini entry.

Do not sort, renumber, or change any other raw entry in this helper. Make readSettingsWithMigration parse once, run the helper, normalize its output, and expose its migration flag. loadSettingsForStartup attempts to write normalized settings only when that flag is true and continues with repaired in-memory settings if the write fails. In main.ts initialize currentSettings with loadSettingsForStartup(app.getPath("userData")) before startSidecar() and syncSavedProviderSettings().

- [ ] **Step 4: Verify GREEN and commit**

Run: cd apps/desktop && node --experimental-strip-types --test tests/electronSettingsMemory.test.ts

Expected: PASS, covering persisted migration, revision preservation, idempotence, removal of custom Gemini, and best-effort write failures.

Run:

    git add apps/desktop/electron/providerTypes.ts apps/desktop/src/providerTypes.ts apps/desktop/electron/settingsMemory.ts apps/desktop/electron/main.ts apps/desktop/tests/electronSettingsMemory.test.ts
    git commit -m "fix: migrate retired Peon providers"

### Task 2: Restore canonical Copilot as a Peon provider

**Files:**

- Modify: crates/orkworksd/resources/harnesses-v2.json
- Modify: crates/orkworksd/src/harness/definition.rs
- Modify: crates/orkworksd/src/harness/registry.rs
- Modify: crates/orkworksd/src/harness/store.rs
- Modify: crates/orkworksd/src/providers.rs
- Test: crates/orkworksd/src/harness/registry.rs and crates/orkworksd/src/providers.rs tests

**Interfaces:**

- Copilot's harness adapter uses `copilot --available-tools= --allow-all-tools --no-custom-instructions -s -p <prompt>`; it has no model override, resume configuration, capacity detector, or new session-ID source.
- Represent stdin versus final-argument prompt delivery in `PeonCapability` and `ProviderDefinition`; existing adapters default to stdin.
- Consume ProviderSettingsPayload through POST /settings/providers.
- Produce apply_settings() that canonicalizes `gh-copilot`, preserves a pre-existing `copilot` on collision, then stores only IDs returned by self.definitions().

- [ ] **Step 1: Write the failing production-catalog test**

First add a registry test asserting the production catalog provides canonical Copilot with argument prompt transport and the exact no-tool arguments. Add a provider test whose fake registry contains Copilot, then supplies `gh-copilot`, canonical `copilot`, and antigravity. Assert canonical Copilot wins on collision and only canonical Copilot reaches inference. Do not add a fake definition for Antigravity.

- [ ] **Step 2: Verify RED**

Run: cargo test --manifest-path crates/orkworksd/Cargo.toml providers::tests::apply_settings_discards_legacy_and_non_peon_provider_ids -- --exact

Expected: FAIL because Copilot has no Peon capability and apply_settings discards every legacy ID.

- [ ] **Step 3: Implement and verify GREEN**

Add an argument prompt transport with a safe stdin default to the harness capability and provider definition. Configure canonical Copilot as a Peon provider with the documented no-tool, silent, non-interactive flags and `-p` as the last fixed argument. Canonicalize the payload before filtering: a supplied canonical `copilot` suppresses `gh-copilot`; otherwise rewrite legacy `gh-copilot` to `copilot`. Retain existing applied-revision and response behavior.

Run: cargo test --manifest-path crates/orkworksd/Cargo.toml providers -- --nocapture

Expected: PASS; canonical Copilot reaches fallback execution with its argument-delivered prompt, while Antigravity and arbitrary unknown IDs cannot reach execution or /providers.

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
