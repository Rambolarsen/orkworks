# Custom Harness Configuration Implementation Plan

> For agentic workers: REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox syntax for tracking.

Goal: Add safe custom harness duplication and editing so a Copilot-compatible copilot-local tool can run independently from copilot, including independent provider state and shared hook ownership.

Architecture: Keep user-editable harness JSON declarative and separate from compiled integrations. Store sidecar-owned compatibility-profile assignments beside custom definitions, derive native signals and integration bindings during registry resolution, and expose them as read-only metadata. Make harness-document revisions explicit at the HTTP boundary, then reconcile shared integrations by adapter/target rather than by harness ID. Add the Settings editor on top of the existing Coding tools controls and preserve the Electron-main confirmation boundary.

Tech Stack: Rust sidecar with Axum, serde/serde_json, the existing atomic harness store and integration handlers; Electron main/preload TypeScript; React/TypeScript renderer; existing Node and Rust test runners; VitePress documentation.

## Global Constraints

- Custom JSON cannot directly select compiled signal handlers, reporters, hook paths, or integration implementations.
- Only a server-created, allowlisted compatibility profile may derive an existing compiled binding; profile metadata is not editable JSON.
- The first profile is copilot, assigned by the sidecar duplicate flow and carried when a profiled custom harness is duplicated.
- The user harness document migrates from version 2 to version 3 with a sidecar-owned compatibilityProfiles map keyed by immutable custom harness ID.
- The sidecar rejects user-config requests larger than 256 KiB, unknown keys, duplicate object keys, malformed placeholders, and invalid capability combinations.
- Supported command placeholders are exactly {model}, {cwd}, {repoRoot}, and {harnessSessionId}.
- Create, duplicate, replace, override, reset, and delete operations require an opaque expectedRevision; stale requests return HTTP 409 with no mutation.
- Custom IDs use the existing lowercase kebab-case grammar and remain immutable after creation.
- Custom deletion is rejected when the ID is active in the current workspace; stale IDs in other workspaces are removed on their next load/save before integration reconciliation.
- Shared integration status and mutation are keyed by code-owned adapter/target identity and projected to all consuming harness rows.
- A custom harness with a Peon capability gets its own provider ID and provider settings; sharing an integration profile never shares provider state.
- Existing Coding tools toggles, detection, command-path control, hook confirmation, and Save flow remain available.
- Creating or editing a harness never installs a hook automatically.
- The renderer never receives authority to mutate workspace hook files.
- The single-active-session context model is unchanged.

---

## Task 1: Add the declarative/profile domain model and strict validation

Files:

- Create: crates/orkworksd/src/harness/compatibility.rs
- Modify: crates/orkworksd/src/harness.rs
- Modify: crates/orkworksd/src/harness/definition.rs
- Modify: crates/orkworksd/src/harness/store.rs
- Test: Rust unit tests in compatibility.rs, definition.rs, and store.rs

Interfaces:

- CompatibilityProfile is a closed enum whose first value is Copilot and whose wire form is "copilot".
- CompatibilityMetadata contains the optional profile plus derived read-only SessionSignalBinding and IntegrationBinding values.
- derive_compatibility_metadata(profile: Option<CompatibilityProfile>) -> CompatibilityMetadata is the only function that maps profile names to compiled bindings.
- HarnessUserDocument gains compatibility_profiles: BTreeMap<String, CompatibilityProfile> and version 3 serialization.
- parse_strict_json<T>(bytes: &[u8], max_bytes: usize) -> Result<T, HarnessDiagnostic> rejects oversized documents, duplicate keys, and trailing input; schema DTOs reject unknown fields.
- parse_custom_definition(bytes: &[u8]) -> Result<HarnessDefinition, Vec<HarnessDiagnostic>> parses only the user-editable custom-definition schema.

- [ ] Step 1: Write failing domain tests.

Add tests for profile derivation, custom binding rejection, duplicate-key rejection, unknown-field rejection, malformed placeholders, the 256 KiB limit, null-versus-omitted patches, and lowercase kebab-case IDs. The core assertions are:

    assert_eq!(
        derive_compatibility_metadata(Some(CompatibilityProfile::Copilot)).integration,
        Some(IntegrationBinding::Copilot)
    );
    assert_eq!(derive_compatibility_metadata(None).session_signals, None);
    assert!(parse_custom_definition(br#"{"id":"copilot-local","name":"Copilot Local","launch":{"kind":"command-template","command":"copilot-local","args":[],"modelPrefix":null},"integration":{"kind":"copilot"}}"#).is_err());
    assert!(parse_strict_json::<serde_json::Value>(br#"{"id":1,"id":2}"#, 256 * 1024).is_err());

- [ ] Step 2: Run the focused Rust tests and verify they fail.

    cargo test --manifest-path crates/orkworksd/Cargo.toml harness::compatibility harness::definition harness::store

Expected: the new profile, parser, and migration tests fail because the types and validators do not exist yet.

- [ ] Step 3: Implement the closed profile and user-document migration.

Add compatibility.rs to harness.rs. Keep HarnessDefinition capable of representing the effective built-in shape, but parse custom definitions and override patches through a user-editable validation path that rejects integration, sessionSignals, profile metadata, hook paths, reporter commands, and executable assets. Add compatibilityProfiles to the v3 document, migrate v2 documents to an empty map, and remove a profile-map entry when its custom definition is deleted.

Use stable diagnostics such as unknown_field, duplicate_key, document_too_large, custom_authority_binding, and invalid_placeholder, including a JSON field path for schema failures.

- [ ] Step 4: Run the focused tests and verify they pass.

    cargo test --manifest-path crates/orkworksd/Cargo.toml harness::compatibility harness::definition harness::store

Expected: all focused profile, strict-parser, placeholder, and v2-to-v3 migration tests pass.

- [ ] Step 5: Commit the domain-model slice.

    git add crates/orkworksd/src/harness.rs crates/orkworksd/src/harness/compatibility.rs crates/orkworksd/src/harness/definition.rs crates/orkworksd/src/harness/store.rs
    git commit -m "feat: add harness compatibility profiles"

## Task 2: Resolve profiles into the runtime registry and provider catalog

Files:

- Modify: crates/orkworksd/src/harness/registry.rs
- Modify: crates/orkworksd/src/harness/store.rs
- Modify: crates/orkworksd/src/providers.rs
- Test: registry.rs and providers.rs unit tests

Interfaces:

- ResolvedHarness exposes CompatibilityMetadata in addition to its resolved effective definition.
- resolve_document applies a built-in code-owned binding or a sidecar-owned custom profile after validating user data; it never serializes derived custom bindings back into HarnessUserDocument.
- HarnessCatalog::providers() continues returning ProviderDefinition values with custom harness IDs as provider IDs.
- ProviderManager::reconcile_harness_provider_settings() normalizes settings against the current projected provider ID set without changing valid custom entries.

- [ ] Step 1: Write failing registry/provider tests.

Cover these cases:

    let document = test_document_with_profile("copilot-local", CompatibilityProfile::Copilot);
    let resolved = resolve_document(&builtins, &document).unwrap();
    let local = resolved.get("copilot-local").unwrap();
    assert_eq!(local.compatibility.profile, Some(CompatibilityProfile::Copilot));
    assert_eq!(local.definition.integration, Some(IntegrationBinding::Copilot));
    assert_eq!(local.definition.session_signals, Some(SessionSignalBinding::Copilot));
    assert_eq!(local.definition.id, "copilot-local");
    assert!(resolved.providers().iter().any(|p| p.id == "copilot-local"));

Define the test-only fixture helper test_document_with_profile(id, profile) in the
registry test module. Also assert that a from-scratch custom has no derived
bindings, a profiled custom duplicate carries the same profile, and editing the
command does not change the profile. Assert independent provider settings by
creating entries for copilot and copilot-local with different models/orders and
verifying both survive registry refresh.

- [ ] Step 2: Run focused tests and verify they fail.

    cargo test --manifest-path crates/orkworksd/Cargo.toml harness::registry providers::tests

Expected: tests fail because registry resolution and provider settings do not consume compatibility profiles.

- [ ] Step 3: Implement runtime projection and deterministic provider normalization.

Look up the profile map by immutable custom ID, call derive_compatibility_metadata, and materialize derived bindings only in the immutable runtime snapshot. Keep built-in bindings intact. Ensure custom Peon projection uses the custom harness ID, label, launch/Peon command, and model capability.

Append newly projected providers after existing entries using the normal default policy. Preserve enabled/order/model/capacity values for unchanged IDs, remove entries for deleted or Peon-less custom harnesses, and clear the current Peon selection when its provider disappears.

- [ ] Step 4: Run focused tests and verify they pass.

    cargo test --manifest-path crates/orkworksd/Cargo.toml harness::registry providers::tests

- [ ] Step 5: Commit the runtime projection slice.

    git add crates/orkworksd/src/harness/registry.rs crates/orkworksd/src/harness/store.rs crates/orkworksd/src/providers.rs
    git commit -m "feat: project custom harness profiles and providers"

## Task 3: Add revision-aware harness CRUD and deletion semantics

Files:

- Modify: crates/orkworksd/src/harness/store.rs
- Modify: crates/orkworksd/src/http/harness_handlers.rs
- Modify: crates/orkworksd/src/main.rs
- Modify: crates/orkworksd/src/metadata.rs
- Modify: crates/orkworksd/src/session_application.rs
- Modify: crates/orkworksd/src/http/session_handlers.rs
- Test: harness_handlers.rs, store.rs, and session_application.rs tests

Interfaces:

- HarnessDocumentRevision is an opaque hex-encoded SHA-256 of the stored bytes and is exposed as documentRevision.
- HarnessStore::snapshot() -> Result<HarnessSnapshot, HarnessStoreError> returns registry, source revision, origins, stored patches, and profile metadata.
- HarnessStore::mutate_at(expected: Option<HarnessDocumentRevision>, change: impl FnOnce(&mut HarnessUserDocument) -> Result<(), HarnessDiagnostic>) -> Result<HarnessMutation, HarnessStoreError> compares the expected revision while holding the write lock and returns the new revision.
- HarnessStoreError::RevisionChanged carries the current revision; HTTP maps it to 409 and harness_config_revision_changed.
- WorkspaceMemory gains active_harness_revision; PUT /workspace/active-harnesses accepts expectedActiveHarnessRevision and increments it on success.
- POST /harnesses/:sourceId/duplicate returns a resolved snapshot/proposed ID/name/current revision without mutating.
- POST /harnesses accepts definition, expectedRevision, and optional duplicateSourceId; the sidecar derives the profile from the source.
- PUT /harnesses/:id, POST /harnesses/:id/remove-profile, and DELETE /harnesses/:id accept expectedRevision.

- [ ] Step 1: Write failing HTTP/store concurrency tests.

Test two GET snapshots followed by two writes, asserting the stale write returns 409 with harness_config_revision_changed and leaves the first result intact. Also test duplicate preview has no file/catalog mutation, final duplicate save assigns the profile, replacement preserves its profile, active current-workspace deletion is rejected, inactive deletion removes the profile map entry, v2 saves as v3, and two active-harness writers cannot overwrite one another.

- [ ] Step 2: Run focused tests and verify they fail.

    cargo test --manifest-path crates/orkworksd/Cargo.toml http::harness_handlers harness::store session_application::tests

Expected: current routes do not return revisions or accept expected revisions, and deletion does not inspect active workspace IDs.

- [ ] Step 3: Implement store and API contracts.

Wrap all harness mutations in mutate_at, preserve the existing atomic double-read write protection, compare the client revision before the mutation closure, and return the new revision on success. Return harness rows with origin, sparse override, and read-only profile metadata while preserving the effective definition used by runtime clients.

Register duplicate-preview and profile-removal routes in main.rs. Final duplicate-save requests identify only the source harness; never accept a profile value from the renderer. Reject deletion when current workspace memory contains the custom ID. Normalize missing IDs during workspace load and before active-harness persistence, then reconcile unreferenced adapters in the active workspace. Add an active-harness revision to WorkspaceMemory and require it on active-selection writes so grouped integration planning cannot publish over a concurrent workspace selection.

- [ ] Step 4: Run focused tests and verify they pass.

    cargo test --manifest-path crates/orkworksd/Cargo.toml http::harness_handlers harness::store session_application::tests

- [ ] Step 5: Commit the CRUD slice.

    git add crates/orkworksd/src/harness/store.rs crates/orkworksd/src/http/harness_handlers.rs crates/orkworksd/src/main.rs crates/orkworksd/src/session_application.rs crates/orkworksd/src/http/session_handlers.rs
    git commit -m "feat: add revision-aware harness configuration API"

## Task 4: Reconcile integrations by adapter and target

Files:

- Modify: crates/orkworksd/src/harness/integration.rs
- Modify: crates/orkworksd/src/harness/registry.rs
- Modify: crates/orkworksd/src/http/integration_handlers.rs
- Modify: crates/orkworksd/src/main.rs
- Test: integration.rs and integration_handlers.rs tests

Interfaces:

- IntegrationKey contains code-owned adapter_id and target_id.
- IntegrationConsumer contains harness_id and harness_name.
- GroupedIntegrationStatus contains IntegrationKey, consumers, and one IntegrationStatus.
- GET /workspace/integrations returns grouped statuses; adapter-key install/repair/uninstall routes execute one mutation per key.
- Existing per-harness status routes remain compatibility shims and delegate to the same adapter implementation.

- [ ] Step 1: Write failing shared-lifecycle tests.

Test Copilot Local alone, Copilot plus Copilot Local, disabling one consumer, disabling both consumers, ambiguous/foreign hooks, partial mutation failure, and workspace identity changes. Assert one status read and one mutation for a shared adapter key; the Electron task separately asserts one native confirmation.

- [ ] Step 2: Run focused integration tests and verify they fail.

    cargo test --manifest-path crates/orkworksd/Cargo.toml http::integration_handlers harness::integration

Expected: current handlers still key status and mutations by harness ID.

- [ ] Step 3: Implement grouped adapter identity and routes.

Derive IntegrationKey and consumer lists from one active registry snapshot. Use the adapter's canonical target identity, never a user-supplied path. Read status once per key, evaluate ownership/detection/activation once per key, and return per-key outcomes with consumer IDs. Leave native confirmation to Electron main. Keep workspace identity and generation revalidation around every external mutation.

When removing a profile or normalizing a stale ID, recompute desired keys first. Persist the user's active selection independently from hook files; on mutation failure retain the selection, return action-needed/cleanup-needed state, and let the next Save retry.

- [ ] Step 4: Run focused integration tests and verify they pass.

    cargo test --manifest-path crates/orkworksd/Cargo.toml http::integration_handlers harness::integration

- [ ] Step 5: Commit the integration slice.

    git add crates/orkworksd/src/harness/integration.rs crates/orkworksd/src/harness/registry.rs crates/orkworksd/src/http/integration_handlers.rs crates/orkworksd/src/main.rs
    git commit -m "feat: reconcile shared harness integrations"

## Task 5: Update Electron integration orchestration and IPC contracts

Files:

- Modify: apps/desktop/electron/activeHarnessIntegration.ts
- Modify: apps/desktop/electron/main.ts
- Modify: apps/desktop/electron/preload.ts
- Modify: apps/desktop/src/api.ts
- Modify: apps/desktop/src/App.tsx
- Modify: apps/desktop/src/harnessIntegrationPresentation.ts
- Modify: apps/desktop/src/orkworksWindow.d.ts
- Test: apps/desktop/tests/activeHarnessSave.test.ts
- Test: apps/desktop/tests/harnessIntegrationSection.test.ts

Interfaces:

- ElectronHarnessConfig includes origin, profile, and derived integration metadata without exposing mutation authority.
- IntegrationKey and GroupedIntegrationStatus are duplicated as explicit main/renderer contract types, following the boundary rule.
- ActiveHarnessSaveResult.integrations is keyed by adapter/target, with consumerHarnessIds and one outcome per grouped mutation.
- saveActiveHarnessesWithIntegrations snapshots workspace guard, harness document revision, active-selection revision, and registry definitions before planning.
- ActiveHarnessIntegrationDeps gains grouped status and grouped mutation functions while retaining the existing Electron-main confirmation callback.
- WorkspaceInfo and the main-process workspace snapshot carry activeHarnessRevision so the renderer and Electron main submit the same active-selection revision.

- [ ] Step 1: Write failing Electron tests.

Assert one confirmation and one install call for two active Copilot-compatible rows, row-level presentation for both consumers, partial-failure retention, stale-workspace results, and no duplicate uninstall when only one consumer is disabled.

- [ ] Step 2: Run focused desktop tests and verify they fail.

    cd apps/desktop
    node --experimental-strip-types --test tests/activeHarnessSave.test.ts tests/harnessIntegrationSection.test.ts

Expected: current code makes one plan per harness ID and cannot consume a grouped response.

- [ ] Step 3: Implement grouped main-process orchestration.

Have Electron main fetch the revision-bearing harness snapshot, group active rows by derived adapter/target, call one status route per group, build one native confirmation listing all consumer names and resolved paths, and call one adapter mutation per group. Map grouped outcomes back to row presentation without issuing additional hook mutations. A stale workspace or stale harness document returns a retryable result before mutation.

Keep setHarnessCommandOverride and clear on the revision-aware custom/built-in API path. Custom edits must not be sent as BuiltinPatch or replace unrelated fields.

- [ ] Step 4: Run focused desktop tests and verify they pass.

    cd apps/desktop
    node --experimental-strip-types --test tests/activeHarnessSave.test.ts tests/harnessIntegrationSection.test.ts

- [ ] Step 5: Commit the Electron contract slice.

    git add apps/desktop/electron/activeHarnessIntegration.ts apps/desktop/electron/main.ts apps/desktop/electron/preload.ts apps/desktop/electron/providerTypes.ts apps/desktop/src/harnessIntegrationPresentation.ts apps/desktop/src/orkworksWindow.d.ts apps/desktop/tests/activeHarnessSave.test.ts apps/desktop/tests/harnessIntegrationSection.test.ts
    git commit -m "feat: group shared harness integration operations"

## Task 6: Make projected provider settings dynamic and independent

Files:

- Modify: crates/orkworksd/src/providers.rs
- Modify: crates/orkworksd/src/http/provider_handlers.rs
- Modify: apps/desktop/electron/providerTypes.ts
- Modify: apps/desktop/src/providerTypes.ts
- Modify: apps/desktop/electron/settingsMemory.ts
- Modify: apps/desktop/electron/main.ts
- Modify: apps/desktop/src/components/SettingsModal.tsx
- Test: apps/desktop/tests/providerSettingsSync.test.ts
- Test: apps/desktop/tests/peonModelPicker.test.ts

Interfaces:

- ProviderId becomes a validated string at the TypeScript boundary; shipped IDs remain constants while custom harness IDs are accepted.
- Provider list responses include provider ID, label, origin, harness ID, and runtime state.
- normalizeProviderSettings(settings: ProviderSettings, definitions: ReadonlyArray<{ id: string; label: string; harnessId?: string }>) -> ProviderSettings appends missing projected providers deterministically and preserves stable-ID settings.
- Removing a projected provider clears current Peon selection and shows an unavailable-provider diagnostic; it never redirects.

- [ ] Step 1: Write failing provider lifecycle tests.

Test adding copilot-local, editing its command/name, removing its Peon capability, deleting it, restarting, and duplicating a profiled harness. Keep copilot and copilot-local enabled/order/model/capacity values independent.

- [ ] Step 2: Run focused provider tests and verify they fail.

    cargo test --manifest-path crates/orkworksd/Cargo.toml providers
    cd apps/desktop
    node --experimental-strip-types --test tests/providerSettingsSync.test.ts tests/peonModelPicker.test.ts

Expected: the TypeScript provider union rejects custom IDs and settings normalization does not create dynamic entries.

- [ ] Step 3: Implement dynamic provider normalization and UI state.

Use immutable harness IDs as provider IDs. Append new Peon-capable custom providers after existing entries with the normal default policy. Preserve settings for unchanged IDs across edits, remove entries when the capability or definition disappears, clear active provider selection when its ID is gone, and refresh the provider list after harness CRUD. Leave standalone Ollama behavior unchanged.

- [ ] Step 4: Run focused provider tests and verify they pass.

    cargo test --manifest-path crates/orkworksd/Cargo.toml providers
    cd apps/desktop
    node --experimental-strip-types --test tests/providerSettingsSync.test.ts tests/peonModelPicker.test.ts

- [ ] Step 5: Commit the provider slice.

    git add crates/orkworksd/src/providers.rs crates/orkworksd/src/http/provider_handlers.rs apps/desktop/electron/providerTypes.ts apps/desktop/src/providerTypes.ts apps/desktop/electron/settingsMemory.ts apps/desktop/electron/main.ts apps/desktop/src/components/SettingsModal.tsx apps/desktop/tests/providerSettingsSync.test.ts apps/desktop/tests/peonModelPicker.test.ts
    git commit -m "feat: preserve settings for custom harness providers"

## Task 7: Add the in-place Settings configuration editor

Files:

- Create: apps/desktop/src/components/HarnessConfigEditor.tsx
- Modify: apps/desktop/src/harnessTypes.ts
- Modify: apps/desktop/src/api.ts
- Modify: apps/desktop/src/App.tsx
- Modify: apps/desktop/src/components/SettingsModal.tsx
- Modify: apps/desktop/src/components/HarnessCommandPathControl.tsx
- Modify: apps/desktop/src/App.css
- Test: Create apps/desktop/tests/harnessConfigEditor.test.ts
- Test: Modify apps/desktop/tests/harnessCommandPathControl.test.ts

Interfaces:

- HarnessConfigEntry contains effective definition, origin, sparse override when present, documentRevision, and read-only compatibility metadata.
- listHarnesses(baseUrl) returns documentRevision and harnesses.
- duplicateHarness(baseUrl, sourceId) returns a server-resolved editable snapshot, proposed ID/name, and revision without persisting or installing hooks.
- saveHarnessConfiguration(baseUrl, request) sends editable definition plus expectedRevision and optional duplicateSourceId; it never sends profile or compiled-binding fields.
- removeHarnessProfile and deleteHarness send expectedRevision and preserve the draft on 409.
- HarnessConfigEditor accepts mode, draftText, metadata, onCancel, and onSaved and renders the read-only effective preview beside editable JSON.

- [ ] Step 1: Write failing renderer tests.

Cover list preservation, origin badges, duplicate flow, override/custom mode, effective preview, read-only profile copy, invalid JSON draft retention, revision-conflict recovery, active-delete rejection, command-path edits preserving unrelated fields, and existing toggle/hook/save controls.

- [ ] Step 2: Run focused renderer tests and verify they fail.

    cd apps/desktop
    node --experimental-strip-types --test tests/harnessConfigEditor.test.ts tests/harnessCommandPathControl.test.ts tests/harnessIntegrationSection.test.ts

Expected: the editor and metadata-aware API functions do not exist.

- [ ] Step 3: Implement API/types and editor.

Add renderer-side validation with line/column parse errors and the same conformance fixtures/error codes as the sidecar. Keep the editor as an in-place detail view with Back navigation. Show Compatibility profile: copilot (read-only) and derived integration/session signals outside the JSON textarea. Show explanatory copy for independent copies, sparse overrides, shared hook ownership, unavailable commands, deletion constraints, and partial integration failures.

Keep existing Coding tools rows, toggles, detection, command-path control, integration status, confirmation, and Save tools in the same section. Make custom command-path editing use the complete-definition/revision path rather than the built-in patch path. Disable Save configuration while local parsing or sidecar validation is invalid, but keep the draft and diagnostics visible.

- [ ] Step 4: Run focused renderer tests and verify they pass.

    cd apps/desktop
    node --experimental-strip-types --test tests/harnessConfigEditor.test.ts tests/harnessCommandPathControl.test.ts tests/harnessIntegrationSection.test.ts

- [ ] Step 5: Commit the Settings slice.

    git add apps/desktop/src/components/HarnessConfigEditor.tsx apps/desktop/src/harnessTypes.ts apps/desktop/src/api.ts apps/desktop/src/App.tsx apps/desktop/src/components/SettingsModal.tsx apps/desktop/src/components/HarnessCommandPathControl.tsx apps/desktop/src/App.css apps/desktop/tests/harnessConfigEditor.test.ts apps/desktop/tests/harnessCommandPathControl.test.ts
    git commit -m "feat: add custom harness settings editor"

## Task 8: Update architecture docs and run the complete verification gate

Files:

- Modify: docs/agents/architecture.md
- Modify: README.md
- Test: apps/desktop/tests/api.test.ts
- Test: crates/orkworksd/src/http/harness_handlers.rs
- Test: crates/orkworksd/src/http/integration_handlers.rs

- [ ] Step 1: Add API and boundary regression tests.

Ensure documented endpoints, duplicated main/preload contract types, renderer/main import boundaries, v3 migration, and grouped integration responses match the implementation. Add a test that a custom JSON payload containing an integration object is rejected even when its value is a known compiled binding. Do not change the already-clarified ADR unless the implemented wire names differ from the design.

- [ ] Step 2: Update architecture documentation.

Document the v3 harness document, sidecar-owned profile map, revision-bearing CRUD routes, grouped adapter/target integration routes, provider projection, and the unchanged Electron-main hook authority boundary. Keep the user-facing term Coding tool and internal term harness consistent with the root guide.

- [ ] Step 3: Run all repository verification commands.

From the repository root:

    cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check
    cargo test --manifest-path crates/orkworksd/Cargo.toml
    bash scripts/doc-check.sh

From apps/desktop:

    npx tsc --noEmit
    node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
    pnpm build

Expected: Rust formatting, all Rust tests, all desktop tests, type-check, desktop build pass, and doc-check emits no unaddressed trigger.

- [ ] Step 4: Inspect the final diff and run the required review gate.

    git diff --check origin/main...HEAD
    git diff --stat origin/main...HEAD

Then run /code-review low. Address findings or record why each is intentional in the PR description before handoff.

- [ ] Step 5: Commit documentation and verification updates.

    git add docs/agents/architecture.md README.md apps/desktop/tests/api.test.ts crates/orkworksd/src/http/harness_handlers.rs crates/orkworksd/src/http/integration_handlers.rs
    git commit -m "docs: document custom harness configuration contracts"
