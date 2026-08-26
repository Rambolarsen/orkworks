# Provider Model Selection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox ( [ ] ) syntax for tracking.

**Goal:** Allow each model provider to select its own Peon model while retaining the existing global model as a fallback, so a model valid for Ollama is not sent to unrelated providers.

**Architecture:** Add an optional model field to every provider settings entry. A single Rust resolver trims values and applies provider override -> global peonModel -> provider default precedence; the resolved value is used for both process arguments and Ollama HTTP requests, and is recorded in observations. Electron mirrors and preserves the field, while the settings UI exposes one editable model field per provider plus the global fallback.

**Tech Stack:** Rust (orkworksd, serde, reqwest), Electron main TypeScript settings persistence, React/TypeScript renderer, Node test runner, Cargo tests.

## Global Constraints

- Preserve backwards compatibility: omitted entry model fields deserialize as null.
- Treat null, empty, and whitespace-only model values as no override/fallback; trim non-empty values consistently in Electron and Rust.
- Do not translate or validate manually entered model IDs against another provider; provider-specific suggestions are advisory only.
- Ollama remains pass-through HTTP through the configured base URL; OrkWorks does not capture, proxy, or store voice/audio.
- Keep the Electron-main/renderer boundary intact: duplicate shared settings types in the existing main and renderer files rather than importing across the boundary.
- Use TDD for implementation, pnpm for desktop commands, and update docs/ADRs only where the existing contract requires it.

---

## File Map

- Modify crates/orkworksd/src/providers.rs: add the entry field, normalization/resolution helper, explicit runner model parameter, Ollama request model handling, observation model, and Rust tests.
- Modify apps/desktop/src/providerTypes.ts: add nullable model to the renderer settings entry type.
- Modify apps/desktop/electron/providerTypes.ts: add the matching nullable model to the Electron settings entry type.
- Modify apps/desktop/electron/settingsMemory.ts: add default/null normalization and preserve normalized entry models through read-normalize-save.
- Modify apps/desktop/src/components/ProviderSettingsSection.tsx: render provider-specific model controls with provider-scoped suggestions and a clear-to-fallback interaction.
- Modify apps/desktop/src/components/SettingsModal.tsx: label the global model as the fallback, pass model data/callbacks to the provider section, and route Ollama candidate selection to the Ollama override.
- Modify apps/desktop/src/providerPresentation.ts only if a pure display/update helper is needed by the component; keep model precedence out of UI-only helpers.
- Modify apps/desktop/tests/electronSettingsMemory.test.ts: persistence, normalization, and legacy compatibility coverage.
- Modify apps/desktop/tests/peonModelPicker.test.ts and/or add a focused provider-model settings test: payload shape and provider override updates.
- Modify existing provider/settings tests that construct ProviderSettingsEntry literals so they include model: null and assert the new behavior.
- Modify docs/superpowers/specs/2026-08-14-provider-settings-migration-design.md: document that the original global model remains a fallback and entry-level overrides are now part of the persisted v1 shape.

## Interfaces Shared Between Tasks

Rust will expose one internal resolver used by inference and tests:

    fn resolve_provider_model(
        entry: &ProviderSettingsEntry,
        global_model: Option<&str>,
    ) -> Option<String>

It returns the trimmed non-empty entry override, otherwise the trimmed non-empty global model, otherwise None. run_inference_with_timeout passes that resolved value to the runner as Option<&str>; process runners ignore it and HttpRunner uses it for Ollama.

The renderer will update a provider immutably through the existing settings persistence path:

    onProviderModelChange(providerId: ProviderId, model: string | null): void

Clearing an input sends null, which means “use the global fallback,” not “disable this provider.”

## Task 1: Add the Rust model contract and resolver

**Files:**
- Modify: crates/orkworksd/src/providers.rs:78-128, 689-735
- Test: crates/orkworksd/src/providers.rs test module near TestEntryBuilder and settings tests

**Interfaces:**
- Consumes: existing ProviderSettingsEntry, ProviderSettingsPayload, and apply_settings normalization.
- Produces: ProviderSettingsEntry::model: Option<String> and resolve_provider_model(...) for all later inference paths.

- [ ] Step 1: Extend the failing serde and normalization tests.

Add tests that deserialize an old payload without model as None, deserialize model: "  llama3  ", and verify apply_settings stores the trimmed value while converting whitespace-only to None. Extend TestEntryBuilder with:

    fn model(mut self, value: Option<&'static str>) -> Self {
        self.model = value;
        self
    }

and include model: Option<String> in the built entry. Keep existing test payloads source-compatible by defaulting the builder field to None.

- [ ] Step 2: Run the focused Rust tests and confirm failure.

Run:

    cargo test --manifest-path crates/orkworksd/Cargo.toml providers::tests -- --nocapture

Expected: FAIL because ProviderSettingsEntry has no model field and the resolver/normalization assertions are not implemented.

- [ ] Step 3: Implement the nullable field and one normalization/resolution path.

Add:

    #[serde(default)]
    pub model: Option<String>,

to ProviderSettingsEntry. Implement the resolver exactly as specified above, trimming the selected candidate and returning None for empty/whitespace values. Apply the same normalizer to incoming global and entry values in ProviderManager::apply_settings before publishing the settings snapshot. Do not infer or rewrite model IDs.

- [ ] Step 4: Run the focused Rust tests and verify they pass.

Run the same cargo test command. Expected: PASS, including old-payload compatibility and whitespace normalization.

- [ ] Step 5: Commit the Rust settings contract.

    git add crates/orkworksd/src/providers.rs
    git commit -m "feat: add per-provider Peon model settings"

## Task 2: Route the resolved model through CLI and Ollama execution

**Files:**
- Modify: crates/orkworksd/src/providers.rs:387-590, 981-1120, 1329-1365
- Test: crates/orkworksd/src/providers.rs provider runner and inference tests

**Interfaces:**
- Consumes: resolve_provider_model(...) and ProviderSettingsEntry::model from Task 1.
- Produces: ProviderRunner::run(..., model: Option<&str>, ...) behavior shared by CompositeRunner, ProcessRunner, HttpRunner, and FakeRunner.

- [ ] Step 1: Write failing execution tests.

Add coverage for all required precedence and transport cases:

    #[test]
    fn provider_override_wins_and_is_recorded_in_observation() {
        let result = run_with_entry_model("copilot", Some("provider-model"), Some("global-model"));
        assert!(result.args_contain("provider-model"));
        assert_eq!(result.observation.provider_model.as_deref(), Some("provider-model"));
    }

    #[test]
    fn global_model_is_used_when_provider_override_is_absent() {
        let result = run_with_entry_model("copilot", None, Some("global-model"));
        assert!(result.args_contain("global-model"));
    }

    #[test]
    fn whitespace_override_falls_back_to_global_model() {
        let result = run_with_entry_model("copilot", Some("   "), Some("global-model"));
        assert!(result.args_contain("global-model"));
    }

    #[test]
    fn ollama_runner_uses_resolved_provider_model_not_global_snapshot() {
        let request = run_ollama_with_entry_model(Some("provider-model"), Some("global-model"));
        assert_eq!(request.model, "provider-model");
    }

    #[test]
    fn provider_without_model_support_receives_no_model_argument() {
        let result = run_with_entry_model("aider", Some("provider-model"), Some("global-model"));
        assert!(!result.args_contain("provider-model"));
        assert!(!result.args_contain("global-model"));
    }

The Ollama test must exercise the actual HttpRunner request body and assert model equals the resolved entry override even when the global field differs. The observation assertion must check the same resolved model.

- [ ] Step 2: Run the focused tests and verify they fail for the bug.

Run:

    cargo test --manifest-path crates/orkworksd/Cargo.toml providers::tests -- --nocapture

Expected: FAIL because inference currently reads only settings.peon_model and HttpRunner independently reads the global snapshot.

- [ ] Step 3: Change the runner interface and use one resolved value.

Add model: Option<&str> to ProviderRunner::run. Pass it through CompositeRunner to both concrete runners. ProcessRunner ignores it. In run_inference_with_timeout, compute:

    let resolved_model = resolve_provider_model(entry, settings.peon_model.as_deref());
    let model_arg = if definition.supports_model {
        resolved_model.as_deref().and_then(|model| {
            definition.model_arg_template.as_deref()
                .map(|template| template.replace("{model}", model))
        })
    } else {
        None
    };

Pass resolved_model.as_deref() to the runner and set ProviderObservation.provider_model to resolved_model. In HttpRunner, remove the read of settings.peon_model and require the explicit model parameter for Ollama, preserving the existing “no Ollama model selected in Peon settings” failure when it is absent.

- [ ] Step 4: Update the fake runner and run the focused tests.

Update every ProviderRunner implementation and invocation, then run the same Cargo command. Expected: PASS, including the Ollama request and observation assertions.

- [ ] Step 5: Run the full Rust validation.

    cargo fmt --manifest-path crates/orkworksd/Cargo.toml -- --check
    cargo test --manifest-path crates/orkworksd/Cargo.toml

Expected: formatting check and all Rust tests PASS.

- [ ] Step 6: Commit the execution-path change.

    git add crates/orkworksd/src/providers.rs
    git commit -m "fix: resolve Peon models per provider"

## Task 3: Preserve the field across Electron settings persistence

**Files:**
- Modify: apps/desktop/src/providerTypes.ts
- Modify: apps/desktop/electron/providerTypes.ts
- Modify: apps/desktop/electron/settingsMemory.ts:114-140, 300-320
- Test: apps/desktop/tests/electronSettingsMemory.test.ts

**Interfaces:**
- Consumes: persisted JSON with optional provider model from the Rust contract.
- Produces: normalized ProviderSettingsEntry.model: string | null in both Electron and renderer settings types.

- [ ] Step 1: Add failing persistence tests.

Extend the existing settings-memory fixtures and add tests that:

1. read a provider entry with model: "  llama3  " as model: "llama3";
2. read whitespace-only or non-string models as null;
3. preserve the normalized model through save then read;
4. read old settings without the field as null; and
5. preserve the top-level legacy peonModel migration behavior unchanged.

Assert the complete normalized entry, not merely raw JSON, so a future normalizer cannot silently drop the field.

- [ ] Step 2: Run the focused Electron tests and verify failure.

From apps/desktop/ run:

    pnpm exec tsx --test tests/electronSettingsMemory.test.ts

Expected: FAIL because the type and normalizeProviderEntry currently omit model.

- [ ] Step 3: Implement mirrored types, defaults, and normalization.

Add model: string | null to both ProviderSettingsEntry interfaces and add model: null to every DEFAULT_PROVIDER_SETTINGS.providers entry. In normalizeProviderEntry, normalize with the same semantics as Rust:

    const model = typeof raw.model === "string" ? raw.model.trim() || null : null;

Return model in the reconstructed entry. Do not use normalizePeonModel for this field; its legacy top-level migration scans old peonModel shapes and must remain a separate compatibility path.

- [ ] Step 4: Run the focused tests and then all desktop unit tests.

    pnpm exec tsx --test tests/electronSettingsMemory.test.ts
    pnpm test

Expected: both commands PASS.

- [ ] Step 5: Commit persistence support.

    git add apps/desktop/src/providerTypes.ts apps/desktop/electron/providerTypes.ts apps/desktop/electron/settingsMemory.ts apps/desktop/tests/electronSettingsMemory.test.ts
    git commit -m "feat: persist provider-specific Peon models"

## Task 4: Add provider-specific settings controls

**Files:**
- Modify: apps/desktop/src/components/ProviderSettingsSection.tsx
- Modify: apps/desktop/src/components/SettingsModal.tsx:430-530
- Modify: apps/desktop/src/providerPresentation.ts only if a pure immutable update helper is introduced
- Test: apps/desktop/tests/peonModelPicker.test.ts, plus a focused pure helper/component-source test following existing project conventions

**Interfaces:**
- Consumes: ProviderSettingsEntry.model, providerModels, and the existing persistProviderSettings callback.
- Produces: global fallback editing and provider-specific override editing with no cross-provider model copy.

- [ ] Step 1: Add failing settings behavior tests.

Cover that the payload sent by a provider model edit contains only the selected entry’s model, that clearing it sends null, and that the global field still serializes as peonModel. If the existing test style cannot mount this component, extract and test a pure update helper with this exact contract:

    function updateProviderModel(
      settings: ProviderSettings,
      providerId: ProviderId,
      value: string,
    ): ProviderSettings

The helper trims value and writes null for empty input while leaving every other provider unchanged.

- [ ] Step 2: Run the focused desktop test and verify failure.

    pnpm exec tsx --test tests/peonModelPicker.test.ts

Expected: FAIL for the new provider override assertions/helper.

- [ ] Step 3: Implement the UI with provider-scoped suggestions.

Relabel the existing global card to “Default Peon model” and explain that it is used when a provider override is blank. Extend ProviderSettingsSection props with:

    providerModels: Record<string, string[]>;
    onProviderModelChange: (providerId: ProviderId, model: string | null) => void;

Render one editable text input per provider entry. Use a stable datalist ID derived from the provider ID and only that provider’s providerModels[id] values. Keep manual IDs accepted. On change update the draft; on blur persist the changed settings. A blank input means “Use default Peon model.” Do not disable the control for providers whose model list is currently unavailable.

Move the Ollama verification candidate action to update only the Ollama entry’s model override, while keeping the global fallback action available through the global field. Update visible copy so users understand that choosing an Ollama model does not pin that model for Copilot, Claude, or other providers.

- [ ] Step 4: Run the focused test and TypeScript checks.

    pnpm exec tsx --test tests/peonModelPicker.test.ts
    pnpm exec tsc --noEmit

Expected: PASS. The TypeScript check must confirm both Electron and renderer settings objects satisfy the expanded entry type.

- [ ] Step 5: Commit the renderer controls.

    git add apps/desktop/src/components/ProviderSettingsSection.tsx apps/desktop/src/components/SettingsModal.tsx apps/desktop/src/providerPresentation.ts apps/desktop/tests/peonModelPicker.test.ts
    git commit -m "feat: add provider-specific model controls"

## Task 5: Synchronize documentation and verify the complete change

### Companion scope note

The user-requested `docs/agents/subagent-model-policy.md` remains part of this
change as a companion policy document. It is intentionally retained alongside
the provider-model implementation rather than removed as out-of-scope; its
delegated-model guidance applies to the implementation and review work that
produced this plan.

**Files:**
- Modify: docs/superpowers/specs/2026-08-14-provider-settings-migration-design.md
- Modify: docs/superpowers/specs/2026-08-25-provider-model-selection-design.md only if implementation clarifies a contract detail
- Test/verification: repository-wide commands below

- [ ] Step 1: Update the authoritative provider-settings design.

Document the v1-compatible persisted shape and precedence in the existing migration design:

    provider.model (optional) > peonModel (global fallback) > provider default

State that model values are trimmed, blank values clear the override, suggestions are provider-scoped, and Ollama receives the same resolved value through its HTTP runner. Link to the dated design doc for the implementation rationale.

- [ ] Step 2: Run repository verification.

    git diff --check
    cargo fmt --manifest-path crates/orkworksd/Cargo.toml -- --check
    cargo test --manifest-path crates/orkworksd/Cargo.toml
    cd apps/desktop && pnpm test && pnpm exec tsc --noEmit && pnpm run build
    cd ../..
    bash .claude/hooks/doc-check.sh
    bash .claude/hooks/worktree-check.sh

Expected: every command exits successfully; doc-check reports no unresolved trigger; worktree-check is reviewed and only this owned worktree is cleaned up after merge.

- [ ] Step 3: Perform the required manual review gate.

Run the repository’s /code-review lightweight review for the code-changing PR. Address every finding or document why it is intentional, then re-run the relevant focused test and git diff --check.

- [ ] Step 4: Commit documentation and hand off for PR review.

    git add docs/superpowers/specs/2026-08-14-provider-settings-migration-design.md docs/superpowers/specs/2026-08-25-provider-model-selection-design.md
    git commit -m "docs: define provider-specific model precedence"

Open one PR for the logical change, include the test output and the explicit compatibility/precedence decision, and wait for the manual review gate before merge.

## Self-Review Checklist

- [x] Spec coverage: schema, precedence, whitespace semantics, CLI arguments, Ollama HTTP body, observations, persistence, UI, tests, and docs each have an explicit task.
- [x] Placeholder scan: no TODO, TBD, or unspecified “add tests” steps remain; each implementation step names files, interfaces, commands, and expected outcomes.
- [x] Type consistency: Rust resolver and runner model parameter are defined before use; TypeScript model: string | null is mirrored before UI work; the UI callback contract matches the update helper.
- [x] Review findings incorporated: Ollama no longer reads the global snapshot, observations use the resolved model, Electron normalization preserves the field, and whitespace semantics are shared.
