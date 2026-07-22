# Harness Capability System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the split harness config/adapter implementation with one resolved capability registry, migrate every built-in, and add secure workspace-scoped integration status/install/uninstall flows.

**Architecture:** A versioned embedded JSON resource and sparse v2 user document resolve into one immutable Rust registry shared by launch, resume, Peon, providers, capacity, signals, and integration management. Closed Rust enums provide the exceptional tool-specific behavior; filesystem mutations run through authenticated Electron-main confirmation, canonical workspace containment, ownership-aware config editors, and durable write-before-publish transactions.

**Tech Stack:** Rust 2021, Axum 0.7, Tokio, serde/serde_json, sha2, git2, Electron 39, React 19, TypeScript 5.9, Node built-in test runner, pnpm 11.

## Global Constraints

- Invoke `skills/starting-work/` before code, create one agent-owned branch/PR, and use a worktree if checkout ownership or concurrency requires it.
- Update `specs/orkworks-mvp.md` and ADR 0026 before implementation code; specs remain authoritative.
- Use pnpm for Node package management; do not use npm or yarn.
- Do not add a runtime plugin platform, dynamic libraries, WebAssembly, downloaded integration code, arbitrary user hook commands, or user-wide integration installation.
- `apps/desktop/electron/` and `apps/desktop/src/` must not import from one another; duplicate IPC contract types intentionally.
- The renderer submits only a harness ID or mutation intent. It never receives the sidecar mutation secret, handler IDs/config, reporter paths, or absolute integration paths.
- Electron main owns the native confirmation and attaches `x-orkworks-integration-token` only after acceptance. Reporter/terminal children never inherit that token.
- Status is read-only and preserves independent `enabled`, `toolDetected`, `registration`, `ownership`, `activation`, `coverage`, and typed diagnostic fields.
- Install/uninstall is workspace-only, explicit, idempotent, ownership-safe, canonically contained, no-follow, and never silently edits tracked/shareable project configuration or `.gitignore`.
- External-edit protection is best-effort optimistic concurrency with a final revision check and documented residual cross-process race; do not claim portable compare-and-swap semantics.
- Native voice remains pass-through metadata only. OrkWorks never captures, proxies, or stores audio.
- Migrate Claude Code, OpenCode, Codex, Gemini CLI, Aider, interactive GitHub Copilot CLI, and generic shell together. `copilot` replaces stock `gh-copilot`; historical session IDs remain readable through an alias.
- A handler cannot claim `Full` coverage until an exact upstream config/payload fixture and supported-version or feature-probe contract passes. Unknown remains unknown.
- Follow TDD for every code task. Run a lightweight review after each checkpoint and a medium-or-higher `/code-review` before PR handoff because this changes security, persistence, protocol, and lifecycle behavior.

---

## File map

### Sidecar domain and persistence

- Create `crates/orkworksd/resources/harnesses-v2.json`: all built-in declarative definitions and shipped-v1 fingerprints.
- Refactor `crates/orkworksd/src/harness.rs`: retain shared command/resume value types and declare focused submodules; remove `HarnessAdapter` after consumers move.
- Create `crates/orkworksd/src/harness/definition.rs`: serde contracts, sparse patch types, closed signal/integration bindings, validation, and public projection.
- Create `crates/orkworksd/src/harness/registry.rs`: immutable `ResolvedHarnessRegistry`, merge/resolve logic, aliases, and shared catalog handle.
- Create `crates/orkworksd/src/harness/store.rs`: v1/v2 parsing, migration diagnostics, revision-checked atomic persistence, and write-before-publish mutation transaction.
- Create `crates/orkworksd/src/harness/integration.rs`: lifecycle status types, mutation intent, path confinement, ownership, and optimistic file transaction primitives.
- Create `crates/orkworksd/src/harness/integrations/{mod,claude,codex,opencode,gemini,copilot,aider}.rs`: tool-specific status/install/uninstall handlers.
- Create `crates/orkworksd/src/harness/signals.rs`: exact event envelopes, code-owned signal contracts, payload validation, normalized metadata commands, and version/feature gates.
- Delete `crates/orkworksd/src/harness_registry.rs` after all consumers use the new registry.

### Sidecar HTTP/runtime

- Replace `crates/orkworksd/src/http/hook_handlers.rs` with `crates/orkworksd/src/http/integration_handlers.rs` and `crates/orkworksd/src/http/harness_signal_handlers.rs`.
- Modify `crates/orkworksd/src/http/{mod,harness_handlers,session_handlers}.rs`, `crates/orkworksd/src/main.rs`, `crates/orkworksd/src/providers.rs`, `crates/orkworksd/src/metadata.rs`, and runtime consumers to use one registry snapshot.
- Create `crates/orkworksd/scripts/report-harness-event.sh` and `crates/orkworksd/scripts/report-harness-event.ps1`; retire the Claude-only reporter after compatibility tests pass.

### Electron and renderer

- Create `apps/desktop/electron/harnessIntegration.ts`: status fetch plus main-owned confirm-and-mutate flow.
- Modify `apps/desktop/electron/{main,preload,settingsMemory,providerTypes}.ts` and matching tests for token handling and `gh-copilot` migration.
- Modify `apps/desktop/src/harnessTypes.ts`, `apps/desktop/src/orkworksWindow.d.ts`, `apps/desktop/src/App.css`, and `apps/desktop/src/components/SettingsModal.tsx`; create `apps/desktop/src/components/HarnessIntegrationRow.tsx`.
- Extend `apps/desktop/tests/{api,newSessionDialogState,dockview,electronSettingsMemory}.test.*`; create `apps/desktop/tests/harnessIntegration.test.ts`.

### Product docs

- Modify `specs/orkworks-mvp.md`, `specs/native-harness-voice-support.md`, `docs/adr/README.md`, `docs/agents/architecture.md`, `README.md`, `AGENTS.md`, and `skills/adding-harness/SKILL.md`.
- Create `docs/adr/0026-resolved-harness-capability-registry.md` and `docs/agents/harness-integration-contracts.md`.

---

### Task 1: Make the architecture authoritative and reconcile the board

**Files:**
- Modify: `specs/orkworks-mvp.md`
- Modify: `specs/native-harness-voice-support.md`
- Create: `docs/adr/0026-resolved-harness-capability-registry.md`
- Modify: `docs/adr/README.md`
- Create: `docs/agents/harness-integration-contracts.md`
- Reference: `docs/superpowers/specs/2026-07-22-harness-capability-system-design.md`

**Interfaces:**
- Consumes: approved reviewed design and current primary vendor hook/plugin documentation.
- Produces: authoritative capability/lifecycle contract, ADR 0026, exact evidence table used by `SignalContract` fixtures, and one umbrella implementation issue.

- [ ] **Step 1: Write the spec and ADR changes before code**

Add the reviewed design's scope, security boundary, v2 configuration shape, status axes, migration guarantees, and per-tool coverage rules to `specs/orkworks-mvp.md`. Replace obsolete complete-config examples in `specs/native-harness-voice-support.md` with a sparse v2 override while keeping voice pass-through. Record this decision in ADR 0026:

```markdown
# Resolved harness capability registry

- Status: accepted
- Deciders: OrkWorks maintainers
- Date: 2026-07-22

## Decision

OrkWorks resolves embedded declarative built-ins plus sparse user overrides into one immutable registry. Declarative closed capability variants implement common behavior; closed compiled Rust bindings implement only verified tool protocols. All consumers read the same published snapshot.

Workspace integration mutations require Electron-main confirmation and sidecar mutation authority, canonical no-follow workspace containment, ownership-aware edits, and durable write-before-publish transactions. The renderer and reporter processes never receive mutation authority.

## Consequences

Adding a simple coding tool is one definition plus tests. Protocol-specific support requires a compiled binding and primary-source contract fixture. User configuration cannot introduce executable integration code or authority-bearing paths. Legacy v1 arrays remain readable and migrate on the next successful save.
```

- [ ] **Step 2: Pin each vendor contract or force a truthful downgrade**

In `docs/agents/harness-integration-contracts.md`, record one row per tool with: primary URL and retrieval date, exact config file and JSON/JS shape, event name, payload fields/types, minimum version evidence or feature probe, normalized metadata write, clear/staleness rule, trust/enablement prerequisite, and no-op rule. Use this explicit decision rule:

```text
primary schema + reproducible fixture + version/tag evidence => verified contract
primary schema + fixture but no version/tag evidence         => feature-probed contract
documented event name without stable payload schema          => limited / activation unknown
no local-only or already ignored/untracked target            => unsupported for installation
```

For OpenCode, keep `session.created` disabled until its upstream TypeScript event payload is pinned; `OPENCODE_SESSION_ID` remains the only high-confidence session-ID source. For any other unresolved field, downgrade the claimed coverage rather than writing an inferred mapping.

- [ ] **Step 3: Verify docs fail for deliberate bad links and pass for the real edits**

Run:

```bash
rtk pnpm --dir docs docs:build
rtk bash .claude/hooks/doc-check.sh
rtk git diff --check
```

Expected: the new files parse and all new links resolve. If the known generated `apm_modules/leonardomso/rust-skills/AGENTS.md` malformed-tag failure remains, run the isolated VitePress renderer against each changed Markdown file and record that unrelated failure in the PR.

- [ ] **Step 4: Create one implementation issue and reconcile existing issues**

Create the umbrella issue with this exact scope:

```bash
rtk gh issue create --title "Implement resolved harness capability registry and workspace integrations" --body "Implements docs/superpowers/specs/2026-07-22-harness-capability-system-design.md and ADR 0026 in one migration. Acceptance: one resolved registry; v2 sparse overrides; durable v1 migration; interactive copilot replacement; authenticated main-confirmed integration lifecycle; canonical containment; ownership-safe uninstall; exact signal contracts; all built-ins migrated; renderer generic integration UI; full Rust/desktop/docs verification. Reconcile #23 #71 #103 #104 #105 #107 #108 #180 #187 #188 in the PR description; #106 Hermes remains out of scope because Hermes is not a current built-in."
```

Comment on #23, #71, #103-#105, #107-#108, #180, #187, and #188 that the umbrella issue supersedes their overlapping acceptance criteria once merged. Comment on #106 that it remains separate and is not silently pulled into built-in scope. Do not close any issue until its acceptance criteria are demonstrably satisfied.

```bash
for issue_number in 23 71 103 104 105 107 108 180 187 188; do
  rtk gh issue comment "$issue_number" --body "The resolved harness capability registry umbrella issue supersedes the overlapping acceptance criteria here once its PR merges. This issue stays open until the PR proves each applicable criterion and records any remaining follow-up."
done
rtk gh issue comment 106 --body "Hermes is not a current OrkWorks built-in and remains outside the approved harness-registry migration. This issue stays separate; the umbrella implementation does not claim or close its acceptance criteria."
```

- [ ] **Step 5: Commit the authoritative docs checkpoint**

```bash
rtk git add specs/orkworks-mvp.md specs/native-harness-voice-support.md docs/adr/0026-resolved-harness-capability-registry.md docs/adr/README.md docs/agents/harness-integration-contracts.md
rtk git commit -m "docs: specify resolved harness capability registry"
```

Expected: one docs-only commit before implementation code.

---

### Task 2: Define and validate declarative harness definitions

**Files:**
- Create: `crates/orkworksd/resources/harnesses-v2.json`
- Create: `crates/orkworksd/src/harness/definition.rs`
- Create: `crates/orkworksd/src/harness/registry.rs`
- Modify: `crates/orkworksd/src/harness.rs`
- Test: unit modules in the three Rust files

**Interfaces:**
- Consumes: exact contract/coverage decisions from Task 1.
- Produces: `HarnessDefinition`, `HarnessPatch`, `SessionSignalBinding`, `IntegrationBinding`, `ResolvedHarnessRegistry`, `HarnessCatalog`, `resolve_document()`, and `public_harnesses()`.

- [ ] **Step 1: Write failing definition/resource tests**

Add tests that parse every built-in, assert IDs `claude-code`, `opencode`, `codex`, `gemini`, `aider`, `copilot`, and `generic-shell`, reject unknown binding variants, reject authority-bearing custom bindings, and pin launch/resume/capacity/voice capability presence. The core assertion shape is:

```rust
#[test]
fn embedded_builtins_are_complete_and_valid() {
    let document = BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap();
    let resolved = resolve_document(&document, &HarnessUserDocument::default()).unwrap();
    assert_eq!(
        resolved.ids().collect::<Vec<_>>(),
        vec!["claude-code", "opencode", "codex", "gemini", "aider", "copilot", "generic-shell"],
    );
    assert!(matches!(resolved.get("codex").unwrap().integration, Some(IntegrationBinding::Codex)));
    assert!(resolved.get("generic-shell").unwrap().integration.is_none());
}
```

- [ ] **Step 2: Run the focused test and confirm the module/resource is missing**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml harness::definition::tests::embedded_builtins_are_complete_and_valid -- --exact`

Expected: FAIL because `harness::definition`, the types, and embedded resource do not exist.

- [ ] **Step 3: Add closed serde contracts**

Implement these exact public-in-crate shapes in `harness/definition.rs`:

```rust
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HarnessDefinition {
    pub id: String,
    pub name: String,
    pub launch: LaunchCapability,
    pub default_model: Option<String>,
    pub resume: Option<ResumeCapability>,
    pub models: Option<ModelCapability>,
    pub peon: Option<PeonCapability>,
    pub capacity: Option<CapacityCapability>,
    pub session_signals: Option<SessionSignalBinding>,
    pub integration: Option<IntegrationBinding>,
    pub voice: Option<VoiceCapability>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(crate) enum LaunchCapability {
    CommandTemplate { command: String, args: Vec<String>, model_prefix: Option<String> },
    PlatformShell { login: bool },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(crate) enum SessionSignalBinding { Claude, Codex, OpenCode, Gemini, Copilot, Aider }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(crate) enum IntegrationBinding { Claude, Codex, OpenCode, Gemini, Copilot, Aider }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResumeCapability {
    pub exact: Option<CommandTemplate>,
    pub latest_cwd: Option<CommandTemplate>,
    pub latest_repo: Option<CommandTemplate>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(crate) enum ModelCapability {
    Static { models: Vec<String> },
    Command { command: String, args: Vec<String> },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PeonCapability {
    pub command_override: Option<String>,
    pub args: Vec<String>,
    pub model_arg_template: Option<String>,
    pub supports_model: bool,
    pub timeout_secs: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(crate) enum CapacityCapability {
    TerminalPatterns { limit_patterns: Vec<String> },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VoiceCapability {
    pub native_voice: bool,
    pub requires_microphone_permission: bool,
    pub orkworks_dictation: bool,
    pub orkworks_voice_commands: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BuiltinDocument {
    pub version: u32,
    pub builtins: Vec<HarnessDefinition>,
    pub legacy_snapshots: Vec<LegacyBuiltinSnapshot>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LegacyBuiltinSnapshot {
    pub schema_version: u32,
    pub harness_id: String,
    pub sha256: String,
    pub definition: HarnessDefinition,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct HarnessUserDocument {
    pub version: u32,
    #[serde(default)]
    pub overrides: BTreeMap<String, HarnessPatch>,
    #[serde(default)]
    pub custom: Vec<HarnessDefinition>,
}

impl Default for HarnessUserDocument {
    fn default() -> Self {
        Self { version: 2, overrides: BTreeMap::new(), custom: Vec::new() }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HarnessPatch {
    pub name: Option<String>,
    pub launch: Option<LaunchPatch>,
    pub default_model: Option<Option<String>>,
    pub resume: Option<Option<ResumePatch>>,
    pub models: Option<Option<ModelCapability>>,
    pub peon: Option<Option<PeonPatch>>,
    pub capacity: Option<Option<CapacityCapability>>,
    pub session_signals: Option<Option<SessionSignalBinding>>,
    pub integration: Option<Option<IntegrationBinding>>,
    pub voice: Option<Option<VoicePatch>>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LaunchPatch {
    pub kind: Option<String>,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub model_prefix: Option<Option<String>>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResumePatch {
    pub exact: Option<Option<CommandTemplate>>,
    pub latest_cwd: Option<Option<CommandTemplate>>,
    pub latest_repo: Option<Option<CommandTemplate>>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PeonPatch {
    pub command_override: Option<Option<String>>,
    pub args: Option<Vec<String>>,
    pub model_arg_template: Option<Option<String>>,
    pub supports_model: Option<bool>,
    pub timeout_secs: Option<u64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VoicePatch {
    pub native_voice: Option<bool>,
    pub requires_microphone_permission: Option<bool>,
    pub orkworks_dictation: Option<bool>,
    pub orkworks_voice_commands: Option<bool>,
}
```

`LegacyBuiltinSnapshot` stores a schema/version label, harness ID, SHA-256 fingerprint, and the complete historical `HarnessDefinition`. Use `Option<T>` for omitted required/scalar fields and `Option<Option<T>>` only at optional capability boundaries.

- [ ] **Step 4: Add deterministic patch/validation rules**

Implement and test:

```rust
impl HarnessDefinition {
    pub(crate) fn apply_patch(&self, patch: &HarnessPatch) -> Result<Self, HarnessDiagnostic>;
    pub(crate) fn validate(&self, origin: DefinitionOrigin) -> Result<(), Vec<HarnessDiagnostic>>;
}

pub(crate) enum DefinitionOrigin { Builtin, Override, Custom }
```

Arrays replace; nested objects merge; changing a tagged `kind` requires a complete replacement value; IDs are immutable; `null` is legal only for optional capabilities; custom IDs cannot collide with built-ins/aliases; authority-bearing bindings reject `Custom`. Tests must cover each rule as a round-trip JSON fixture rather than only constructing Rust structs.

- [ ] **Step 5: Populate the one embedded resource**

Move every value from `builtin_harness_configs()` and `builtin_adapters()` into `harnesses-v2.json`, preserving live launch behavior, supported exact/latest resume behavior, Peon config, model lists, capacity patterns, and voice metadata. Use interactive Copilot:

```json
{
  "id": "copilot",
  "name": "GitHub Copilot CLI",
  "launch": { "kind": "command-template", "command": "copilot", "args": [], "modelPrefix": null },
  "sessionSignals": { "kind": "copilot" },
  "integration": { "kind": "copilot" }
}
```

The resource also contains the exact shipped v1 stock snapshots/fingerprints used by Task 3; generic shell uses a declarative platform-shell variant rather than serializing the developer's current `$SHELL` value.

- [ ] **Step 6: Implement immutable resolution and one shared catalog**

In `harness/registry.rs`, provide:

```rust
pub(crate) struct ResolvedHarnessRegistry {
    ordered: Vec<ResolvedHarness>,
    by_id: HashMap<String, usize>,
    aliases: HashMap<String, String>,
    diagnostics: Vec<HarnessDiagnostic>,
    providers: Vec<ProviderDefinition>,
}

pub(crate) struct ResolvedHarness {
    pub definition: HarnessDefinition,
    pub origin: DefinitionOrigin,
    pub effective_capabilities: BTreeSet<CapabilityName>,
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CapabilityName {
    Launch,
    ResumeExact,
    ResumeLatestCwd,
    ResumeLatestRepo,
    Models,
    Peon,
    Capacity,
    NativeSessionId,
    Attention,
    Lifecycle,
    Voice,
    WorkspaceIntegration,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HarnessDiagnostic {
    pub harness_id: Option<String>,
    pub code: String,
    pub message: String,
}

pub(crate) type HarnessCatalog = Arc<std::sync::RwLock<Arc<ResolvedHarnessRegistry>>>;

pub(crate) fn resolve_document(
    builtins: &BuiltinDocument,
    user: &HarnessUserDocument,
) -> Result<ResolvedHarnessRegistry, Vec<HarnessDiagnostic>>;
```

An invalid custom entry is excluded with a diagnostic. An invalid override retains its untouched built-in with a diagnostic. Provider definitions are derived during resolution and stored in the same immutable snapshot.

- [ ] **Step 7: Run the registry tests and commit**

Run:

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml harness::definition
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml harness::registry
rtk cargo fmt --manifest-path crates/orkworksd/Cargo.toml -- --check
```

Expected: all definition/resource/patch/validation tests PASS and formatting is clean.

```bash
rtk git add crates/orkworksd/resources/harnesses-v2.json crates/orkworksd/src/harness.rs crates/orkworksd/src/harness/definition.rs crates/orkworksd/src/harness/registry.rs
rtk git commit -m "feat: define resolved harness registry"
```

---

### Task 3: Add v1 migration and durable write-before-publish storage

**Files:**
- Create: `crates/orkworksd/src/harness/store.rs`
- Modify: `crates/orkworksd/src/harness/definition.rs`
- Modify: `crates/orkworksd/src/harness/registry.rs`
- Modify: `crates/orkworksd/src/http/harness_handlers.rs`
- Modify: `crates/orkworksd/src/main.rs`
- Test: unit tests in `harness/store.rs` and handler tests in `http/harness_handlers.rs`

**Interfaces:**
- Consumes: `HarnessCatalog`, embedded stock snapshots, `resolve_document()`.
- Produces: `HarnessStore::load()`, `HarnessStore::mutate()`, v1 diagnostics, atomic v2 persistence, and CRUD responses that never expose unpersisted state.

- [ ] **Step 1: Write failing migration and transaction tests**

Cover: missing file; valid v2; each known stock v1 snapshot; sparse customized v1 built-in; unknown historical snapshot conservative freeze; invalid entry isolation; stock/custom/conflicting/malformed `gh-copilot`; injected serialization/write/rename failure; edit-between-check-and-rename; and publish only after successful replacement.

The decisive persistence test is:

```rust
#[test]
fn failed_replace_leaves_disk_and_live_catalog_unchanged() {
    let fixture = StoreFixture::v2();
    let before = fixture.catalog.read().unwrap().clone();
    fixture.writer.fail_next_replace();
    assert!(fixture.store.mutate(&fixture.catalog, |doc| {
        doc.overrides.entry("codex".into()).or_default().name = Some("Changed".into());
        Ok(())
    }).is_err());
    assert!(Arc::ptr_eq(&before, &fixture.catalog.read().unwrap()));
    assert_eq!(fixture.read_document().overrides, HarnessUserDocument::default().overrides);
}
```

- [ ] **Step 2: Run the focused tests and verify failure**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml harness::store -- --nocapture`

Expected: FAIL because `HarnessStore` and transaction seams do not exist.

- [ ] **Step 3: Implement supported v1/v2 loading and migration**

Use these contracts:

```rust
pub(crate) struct HarnessStore {
    path: PathBuf,
    builtins: Arc<BuiltinDocument>,
    write_lock: Mutex<()>,
    writer: Arc<dyn AtomicWriter>,
}

pub(crate) trait AtomicWriter: Send + Sync {
    fn replace_if_revision(
        &self,
        target: &Path,
        expected_revision: Option<[u8; 32]>,
        contents: &[u8],
    ) -> Result<(), HarnessStoreError>;
}

#[derive(Debug)]
pub(crate) enum HarnessStoreError {
    Io(std::io::Error),
    Parse(serde_json::Error),
    RevisionChanged,
    Validation(Vec<HarnessDiagnostic>),
    Mutation(HarnessDiagnostic),
}

pub(crate) struct LoadedHarnesses {
    pub document: HarnessUserDocument,
    pub registry: Arc<ResolvedHarnessRegistry>,
    pub source_revision: Option<[u8; 32]>,
    pub migrated_from_v1: bool,
}

impl HarnessStore {
    pub(crate) fn load(&self) -> Result<LoadedHarnesses, HarnessStoreError>;
    pub(crate) fn mutate<F>(&self, catalog: &HarnessCatalog, change: F)
        -> Result<Arc<ResolvedHarnessRegistry>, HarnessStoreError>
    where F: FnOnce(&mut HarnessUserDocument) -> Result<(), HarnessDiagnostic>;
}
```

`load()` hashes the exact bytes, parses v1 or v2, and migrates only in memory. Known stock fields inherit current built-ins; customized fields become sparse patches; unknown historical snapshots freeze explicit values and emit a diagnostic. Stock `gh-copilot` becomes unmodified `copilot`; custom legacy commands remain an override or uniquely named custom definition with a diagnostic. `gh-copilot` remains a read-only display alias.

- [ ] **Step 4: Implement the durable mutation order**

`mutate()` must: take the store lock; re-read supported v1/v2 bytes; revision-check; migrate in memory; apply the closure; resolve and validate; serialize one complete v2 document; create a temporary file in the same directory; flush it; final-check the source revision; replace atomically; then swap the exact resolved `Arc` into `HarnessCatalog`. Return every filesystem error. Do not call `std::fs::write` directly from handlers.

- [ ] **Step 5: Convert CRUD handlers to sparse documents and errors**

`POST /harnesses` accepts a complete declarative custom definition. `PUT /harnesses/:id` accepts `HarnessPatch` for built-ins and a complete replacement for custom definitions. `DELETE` removes only custom definitions or a built-in override, never the built-in. All three call `HarnessStore::mutate`; a failed save returns a structured error and leaves the catalog unchanged.

- [ ] **Step 6: Run migration/handler tests and commit**

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml harness::store
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml http::harness_handlers
rtk git diff --check
rtk git add crates/orkworksd/src/harness/store.rs crates/orkworksd/src/harness/definition.rs crates/orkworksd/src/harness/registry.rs crates/orkworksd/src/http/harness_handlers.rs crates/orkworksd/src/main.rs
rtk git commit -m "feat: persist versioned harness definitions"
```

Expected: migration corpus and failure injection tests PASS; no handler publishes a failed write.

---

### Task 4: Move every runtime consumer to the resolved snapshot

**Files:**
- Modify: `crates/orkworksd/src/main.rs`
- Modify: `crates/orkworksd/src/http/session_handlers.rs`
- Modify: `crates/orkworksd/src/providers.rs`
- Modify: `crates/orkworksd/src/runtime/peon_runtime.rs`
- Modify: `crates/orkworksd/src/runtime/session_runtime.rs`
- Modify: `crates/orkworksd/src/runtime/terminal_runtime.rs`
- Modify: `crates/orkworksd/src/session_view.rs`
- Modify: `crates/orkworksd/src/metadata.rs`
- Delete: `crates/orkworksd/src/harness_registry.rs`
- Test: existing Rust tests plus focused launch/resume/provider/capacity tests

**Interfaces:**
- Consumes: `HarnessCatalog`, `ResolvedHarness::build_launch()`, `build_resume()`, `providers()`, and declarative capacity patterns.
- Produces: one live code path with no `HarnessAdapter`, capability booleans, duplicated built-ins, or harness-ID switching in generic consumers.

- [ ] **Step 1: Rewrite existing tests against one definition source before deleting code**

Table-test each built-in's launch with/without model, exact/latest resume options, Peon provider projection, and capacity patterns. Pin the former drift case:

```rust
#[test]
fn opencode_launch_and_resume_share_one_definition() {
    let harness = test_registry().get("opencode").unwrap();
    let launch = harness.build_launch("/repo", Some("qwen3"));
    assert_eq!(launch.program, "opencode");
    assert_eq!(launch.args, ["--model", "ollama/qwen3"]);
    let resume = harness.build_resume(ResumeStrategy::Exact, "/repo", Some("ses_1"), None).unwrap();
    assert_eq!(resume.program, "opencode");
    assert_eq!(resume.args, ["--session", "ses_1"]);
}
```

- [ ] **Step 2: Run the focused test and see it fail on the old split API**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml opencode_launch_and_resume_share_one_definition -- --exact`

Expected: FAIL because the resolved methods are not yet used by session handlers.

- [ ] **Step 3: Replace AppState's split fields with the catalog/store**

Use:

```rust
struct AppState {
    // existing session/workspace/peon/retention fields
    harness_catalog: harness::registry::HarnessCatalog,
    harness_store: harness::store::HarnessStore,
    providers: providers::ProviderManager,
    bound_port: AtomicU16,
}
```

Remove `adapters` and `harnesses`. Construct the store, load once, create the shared catalog, and pass the catalog clone to `ProviderManager`; provider reads must derive from the same current registry `Arc`, not a copied vector.

- [ ] **Step 4: Migrate launch, resume, Peon, providers, and capacity**

Change `resolve_session_launch` to accept `&ResolvedHarnessRegistry`, resolve the requested ID/alias once, and call `ResolvedHarness::build_launch`. Resume options come from the definition and captured memory. Peon/model/provider/capacity consumers read capability data from the same snapshot. A missing ID falls back only to `generic-shell`; no consumer switches on `claude-code`, `codex`, or another harness ID.

- [ ] **Step 5: Delete the split adapter implementation**

Remove `HarnessAdapter`, `HarnessAdapterConfig`, `HarnessCapabilities`, `LaunchRequest`, `builtin_harness_configs()`, `builtin_adapters()`, `resolve_adapter_harness_id()`, and their dead tests. Delete `harness_registry.rs`. Retain neutral `CommandSpec`, `ResumeStrategy`, `ResumeMemory`, and template rendering under `harness.rs`/submodules.

- [ ] **Step 6: Run the complete Rust suite and commit the runtime checkpoint**

```bash
rtk cargo fmt --manifest-path crates/orkworksd/Cargo.toml
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml
rtk cargo clippy --manifest-path crates/orkworksd/Cargo.toml --all-targets -- -D warnings
rtk git add crates/orkworksd/src
rtk git commit -m "refactor: route harness consumers through one registry"
```

Expected: all Rust tests PASS, clippy is clean, and `rtk rg 'HarnessAdapter|builtin_adapters|builtin_harness_configs' crates/orkworksd/src` returns no matches.

---

### Task 5: Build safe generic integration lifecycle primitives

**Files:**
- Create: `crates/orkworksd/src/harness/integration.rs`
- Create: `crates/orkworksd/src/harness/integrations/mod.rs`
- Modify: `crates/orkworksd/src/harness.rs`
- Test: unit tests in `harness/integration.rs`

**Interfaces:**
- Consumes: `IntegrationBinding`, current workspace root, git2 status/ignore checks.
- Produces: lifecycle status types, `IntegrationHandler`, canonical target validation, ownership matching, and `ConfigFileTransaction` used by every tool handler.

- [ ] **Step 1: Write the security/failure corpus first**

Add tests for missing workspace, regular nested target, missing leaf with existing parent, file symlink escape, directory symlink escape, workspace switched before replace, tracked target, untracked-but-not-ignored target, already ignored dedicated target, malformed JSON, ownership mismatch, edit between final check and replace, and injected replace failure. Under Windows, create junction/reparse-point fixtures; under Unix, use `std::os::unix::fs::symlink`.

```rust
#[cfg(unix)]
#[test]
fn target_validation_rejects_directory_symlink_escape() {
    let workspace = tempdir().unwrap();
    let outside = tempdir().unwrap();
    symlink(outside.path(), workspace.path().join(".codex")).unwrap();
    let error = ValidatedWorkspaceTarget::new(workspace.path(), Path::new(".codex/hooks.json")).unwrap_err();
    assert_eq!(error.code(), "workspace_escape");
}
```

- [ ] **Step 2: Run the focused security test and verify failure**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml harness::integration -- --nocapture`

Expected: FAIL because lifecycle/path transaction types do not exist.

- [ ] **Step 3: Define the truthful lifecycle contract**

Implement these serialized shapes:

```rust
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum IntegrationRegistration { Unsupported, Absent, Installed, Drifted, Error }

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum IntegrationOwnership { None, OrkWorks, Ambiguous }

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum IntegrationActivation { Active, NeedsTrust, Disabled, Unknown, NotApplicable }

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum IntegrationCoverage { Full, Limited, None }

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IntegrationStatus {
    pub harness_id: String,
    pub enabled: bool,
    pub tool_detected: bool,
    pub registration: IntegrationRegistration,
    pub ownership: IntegrationOwnership,
    pub activation: IntegrationActivation,
    pub coverage: IntegrationCoverage,
    pub diagnostics: Vec<IntegrationDiagnostic>,
    pub confirmation: Option<IntegrationConfirmation>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IntegrationDiagnostic {
    pub code: String,
    pub message: String,
    pub action: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IntegrationConfirmation {
    pub tool_name: String,
    pub workspace_label: String,
    pub coverage_summary: String,
    pub relative_paths: Vec<String>,
    pub executable_code_warning: bool,
}
```

`IntegrationConfirmation` contains sanitized tool name, workspace scope, coverage text, and repo-relative paths only.

- [ ] **Step 4: Define the narrow handler boundary**

```rust
pub(crate) trait IntegrationHandler: Send + Sync {
    fn status(&self, ctx: &IntegrationContext<'_>) -> Result<IntegrationStatus, IntegrationError>;
    fn install(&self, ctx: &IntegrationContext<'_>) -> Result<IntegrationStatus, IntegrationError>;
    fn uninstall(&self, ctx: &IntegrationContext<'_>) -> Result<IntegrationStatus, IntegrationError>;
}

pub(crate) fn handler(binding: &IntegrationBinding) -> &'static dyn IntegrationHandler;

pub(crate) struct IntegrationContext<'a> {
    pub workspace: &'a Path,
    pub orkworks_root: &'a Path,
    pub enabled: bool,
    pub detected_tool: Option<&'a DetectedTool>,
    pub reporter_assets: &'a ReporterAssetResolver,
}

pub(crate) struct DetectedTool {
    pub executable: PathBuf,
    pub version: Option<String>,
    pub compatible: bool,
}

pub(crate) struct ReporterAssetResolver {
    pub source_dir: PathBuf,
    pub stable_dir: PathBuf,
}

impl ReporterAssetResolver {
    pub(crate) fn reconcile(&self, asset_name: &str) -> Result<PathBuf, IntegrationError>;
}

#[derive(Debug)]
pub(crate) enum IntegrationError {
    NoWorkspace,
    UnsafeTarget { code: &'static str, message: String },
    InvalidConfig(String),
    OwnershipAmbiguous,
    RevisionChanged,
    Io(std::io::Error),
}
```

`handler()` is an exhaustive match over the closed enum. It is not a string-keyed registry. `IntegrationContext` supplies canonical workspace, OrkWorks metadata root, active harness enabled state, detected tool/version result, and stable reporter asset resolver; definitions cannot override those values.

- [ ] **Step 5: Implement canonical target and file transaction primitives**

`ValidatedWorkspaceTarget::new(workspace, relative)` canonicalizes the workspace and nearest existing ancestor, rejects absolute/parent paths and symlink/reparse escapes, checks git tracking/ignore policy, and captures a workspace identity. `ConfigFileTransaction::commit()` creates the temporary file beside the target, flushes, final-checks source hash/stat plus workspace identity/containment, retains a recoverable backup when allowed, and replaces:

```rust
impl ValidatedWorkspaceTarget {
    pub(crate) fn new(workspace: &Path, relative: &Path) -> Result<Self, IntegrationError>;
    pub(crate) fn require_local_or_ignored_untracked(&self) -> Result<(), IntegrationError>;
    pub(crate) fn relative_path(&self) -> &Path;
}

impl ConfigFileTransaction {
    pub(crate) fn open(target: ValidatedWorkspaceTarget) -> Result<Self, IntegrationError>;
    pub(crate) fn current_bytes(&self) -> &[u8];
    pub(crate) fn commit(self, replacement: &[u8]) -> Result<(), IntegrationError>;
}
```

A detected external edit returns `RevisionChanged`; the residual race after final check is documented in the type's rustdoc.

- [ ] **Step 6: Run security tests and commit**

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml harness::integration
rtk cargo fmt --manifest-path crates/orkworksd/Cargo.toml -- --check
rtk git add crates/orkworksd/src/harness.rs crates/orkworksd/src/harness/integration.rs crates/orkworksd/src/harness/integrations/mod.rs
rtk git commit -m "feat: add safe harness integration lifecycle"
```

Expected: the security corpus passes on the host; platform-gated tests compile on all CI targets.

---

### Task 6: Implement every built-in integration handler

**Files:**
- Create: `crates/orkworksd/src/harness/integrations/claude.rs`
- Create: `crates/orkworksd/src/harness/integrations/codex.rs`
- Create: `crates/orkworksd/src/harness/integrations/opencode.rs`
- Create: `crates/orkworksd/src/harness/integrations/gemini.rs`
- Create: `crates/orkworksd/src/harness/integrations/copilot.rs`
- Create: `crates/orkworksd/src/harness/integrations/aider.rs`
- Modify: `crates/orkworksd/src/harness/integrations/mod.rs`
- Modify: `crates/orkworksd/src/harness/registry.rs`
- Test: fixture modules beside each handler

**Interfaces:**
- Consumes: `IntegrationHandler`, `ConfigFileTransaction`, stable asset resolver, Task 1 contract evidence.
- Produces: status/install/uninstall for all built-ins, with shell unsupported and Aider limited.

- [ ] **Step 1: Write a shared conformance test matrix before handlers**

For every supported handler, run the same fixture cases: absent; unrelated config; installed; partial/drifted; malformed; install twice; uninstall twice; install then uninstall; ambiguous edit; unrelated-key preservation; unsupported version; disabled/trust unknown; POSIX/Windows command rendering; and no eligible local file. Use an explicit table:

```rust
for case in integration_cases() {
    let first = case.handler.install(&case.context).unwrap();
    let second = case.handler.install(&case.context).unwrap();
    assert_eq!(first.registration, IntegrationRegistration::Installed, "{}", case.name);
    assert_eq!(second.registration, IntegrationRegistration::Installed, "{}", case.name);
    assert_eq!(case.unrelated_after(), case.unrelated_before(), "{}", case.name);
}
```

- [ ] **Step 2: Run conformance tests and verify all handler variants fail**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml harness::integrations -- --nocapture`

Expected: FAIL because the concrete handlers do not exist.

- [ ] **Step 3: Implement JSON hook handlers with owned markers**

Claude uses `.claude/settings.local.json`; Copilot uses `.github/copilot/settings.local.json`. Codex uses an eligible dedicated `.codex/hooks.json` only when local-only or already ignored/untracked. Gemini uses an eligible `.gemini/settings.json` under the same policy. Each entry contains a stable OrkWorks marker/schema version and invokes only the code-owned stable reporter command. Parse the existing JSON object before mutation, merge only the owned entries, and remove only exact owned entries on uninstall.

Each module exposes one zero-sized handler:

```rust
pub(crate) struct ToolHookContract {
    pub harness_id: &'static str,
    pub relative_path: &'static str,
    pub ownership_marker: &'static str,
    pub coverage: IntegrationCoverage,
}

pub(crate) struct JsonHookHandler {
    contract: ToolHookContract,
    merge: fn(&mut serde_json::Value, &Path) -> Result<(), IntegrationError>,
    remove: fn(&mut serde_json::Value) -> Result<IntegrationOwnership, IntegrationError>,
}

impl JsonHookHandler {
    pub(crate) const fn new(
        contract: ToolHookContract,
        merge: fn(&mut serde_json::Value, &Path) -> Result<(), IntegrationError>,
        remove: fn(&mut serde_json::Value) -> Result<IntegrationOwnership, IntegrationError>,
    ) -> Self {
        Self { contract, merge, remove }
    }
}

pub(crate) static HANDLER: JsonHookHandler = JsonHookHandler::new(
    ToolHookContract {
        harness_id: "claude-code",
        relative_path: ".claude/settings.local.json",
        ownership_marker: "orkworks:harness-integration:v2:claude-code",
        coverage: IntegrationCoverage::Full,
    },
    merge_claude_entries,
    remove_claude_entries,
);
```

Use concrete contract values from `docs/agents/harness-integration-contracts.md`; do not copy this Claude path/coverage to another module.

- [ ] **Step 4: Implement OpenCode's dedicated plugin conservatively**

Create/remove only `.opencode/plugins/orkworks.js`, only if it is already ignored and untracked. The generated file includes the ownership marker and subscribes only to event payloads pinned in Task 1. If payload types remain unverified, status is `coverage = Limited`, `activation = Unknown`, and the plugin forwards only verified fields. Never modify `.gitignore`.

- [ ] **Step 5: Implement Aider as workspace metadata plus launch augmentation**

Aider install/uninstall toggles a versioned flag in OrkWorks workspace metadata rather than editing repository config. Its resolved launch capability adds exactly one `--notifications-command <stable reporter>` pair when enabled and does not duplicate an existing OrkWorks-owned pair. Status reports limited coverage and no native session ID.

- [ ] **Step 6: Keep generic shell explicitly unsupported**

`ResolvedHarness::integration_status()` returns registration `Unsupported`, ownership `None`, activation `NotApplicable`, coverage `None`, and a diagnostic explaining that generic shell exposes no deterministic integration mechanism. POST/DELETE later return conflict without touching disk.

- [ ] **Step 7: Run the handler matrix and commit**

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml harness::integrations
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml aider_launch
rtk git diff --check
rtk git add crates/orkworksd/src/harness/integrations crates/orkworksd/src/harness/registry.rs
rtk git commit -m "feat: add built-in harness integration handlers"
```

Expected: all handler fixtures pass; tracked/shareable configs are never modified; Aider is limited; shell is unsupported.

---

### Task 7: Generalize stable reporters and normalized signal ingestion

**Files:**
- Create: `crates/orkworksd/src/harness/signals.rs`
- Create: `crates/orkworksd/src/http/harness_signal_handlers.rs`
- Create: `crates/orkworksd/scripts/report-harness-event.sh`
- Create: `crates/orkworksd/scripts/report-harness-event.ps1`
- Modify: `crates/orkworksd/src/http/mod.rs`
- Modify: `crates/orkworksd/src/main.rs`
- Modify: `crates/orkworksd/src/metadata.rs`
- Modify: `crates/orkworksd/src/runtime/terminal_runtime.rs`
- Delete after compatibility coverage: `crates/orkworksd/scripts/report-claude-session-from-hook.sh`
- Test: signal contract fixtures and reporter content tests

**Interfaces:**
- Consumes: `SessionSignalBinding`, exact Task 1 payload fixtures, existing metadata priority/merge functions.
- Produces: `HarnessEventEnvelope`, `NormalizedHarnessSignal`, `translate_event()`, generic reporter endpoint, stable cross-platform assets, and no-op-outside-OrkWorks behavior.

- [ ] **Step 1: Write failing payload/ordering tests for each tool**

For Claude, Codex, Gemini, and Copilot, fixture exact start/prompt/attention/end payloads. For OpenCode, fixture only pinned upstream TypeScript shapes; reject unverified fields. For Aider, fixture ready notification. Cover missing correlation, wrong type, unknown event, late timestamp, session already ended, source precedence, and outside-OrkWorks reporter invocation.

```rust
#[test]
fn late_codex_stop_cannot_restore_cleared_attention() {
    let mut state = SignalState::with_attention_cleared_at("2026-07-22T12:01:00Z");
    let signal = translate_event(&SessionSignalBinding::Codex, codex_stop("2026-07-22T12:00:00Z")).unwrap();
    assert_eq!(state.apply(signal), SignalApplyResult::IgnoredStale);
    assert_eq!(state.attention(), None);
}
```

- [ ] **Step 2: Run the signal tests and verify failure**

Run:

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml harness::signals -- --nocapture
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml http::harness_signal_handlers -- --nocapture
```

Expected: FAIL because generic translation/ingestion does not exist.

- [ ] **Step 3: Implement closed signal contracts**

```rust
pub(crate) enum NormalizedHarnessSignal {
    SessionId { value: String, source: &'static str, confidence: f64, at: DateTime<Utc> },
    AttentionSet { observed_status: &'static str, attention: &'static str, source: &'static str, confidence: f64, at: DateTime<Utc> },
    AttentionClear { source: &'static str, confidence: f64, at: DateTime<Utc> },
    SessionEnd { outcome: String, source: &'static str, confidence: f64, at: DateTime<Utc> },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HarnessEventEnvelope {
    pub event_name: String,
    pub emitted_at: DateTime<Utc>,
    pub payload: serde_json::Value,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SignalError {
    UnsupportedEvent,
    InvalidPayload(String),
    InvalidCorrelation,
    Stale,
}

pub(crate) fn translate_event(
    binding: &SessionSignalBinding,
    envelope: HarnessEventEnvelope,
) -> Result<Vec<NormalizedHarnessSignal>, SignalError>;
```

Use exhaustive binding matches and exact typed payload structs. Apply signals through existing metadata merge/source precedence, adding timestamp rejection where needed. Do not let lifecycle events implicitly become attention events unless the contract table explicitly maps them.

- [ ] **Step 4: Add the generic unauthenticated reporter endpoint with narrow authority**

Add `POST /sessions/:id/harness-events/:harnessId`. It accepts only a bounded JSON envelope, verifies that the live session resolved to that harness/alias, validates the code-owned payload contract, and can mutate only normalized session metadata. It cannot install integrations, access paths, or receive the integration token. Missing `ORKWORKS_SESSION_ID`/`ORKWORKS_PORT` in a reporter is a successful no-op.

- [ ] **Step 5: Package and self-heal stable reporters**

Both reporter assets read stdin when the tool supplies JSON, bound network calls with a timeout, post only to `127.0.0.1:$ORKWORKS_PORT`, and return success outside OrkWorks. Install copies them to `~/.orkworks/hook-scripts/` and refreshes the stable copy on every reconcile. Extend release packaging tests so both scripts ship beside the sidecar source assets.

- [ ] **Step 6: Preserve the existing Claude marker during migration**

Recognize the legacy Claude Notification entry and reporter name as OrkWorks-owned compatibility state. Status shows installed; reconcile upgrades it to v2 owned entries; uninstall removes only the recognized legacy/new owned commands. Delete the old script only after its recognition and upgrade fixtures pass.

- [ ] **Step 7: Run signals, metadata, reporter, and packaging tests; commit**

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml harness::signals
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml harness_signal_handlers
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml metadata
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml reporter
cd apps/desktop && rtk node --experimental-strip-types --test tests/packageRelease.test.mjs
cd ../.. && rtk git add crates/orkworksd apps/desktop/tests/packageRelease.test.mjs apps/desktop/scripts
rtk git commit -m "feat: ingest normalized harness lifecycle signals"
```

Expected: all exact payload and stale/precedence tests pass; reporters are bounded no-ops outside OrkWorks.

---

### Task 8: Add authenticated routes and Electron-main confirmation

**Files:**
- Create: `crates/orkworksd/src/http/integration_handlers.rs`
- Modify: `crates/orkworksd/src/http/mod.rs`
- Modify: `crates/orkworksd/src/main.rs`
- Delete: `crates/orkworksd/src/http/hook_handlers.rs`
- Create: `apps/desktop/electron/harnessIntegration.ts`
- Modify: `apps/desktop/electron/main.ts`
- Modify: `apps/desktop/electron/preload.ts`
- Modify: `apps/desktop/src/orkworksWindow.d.ts`
- Modify: `apps/desktop/tests/dockview.test.ts`
- Create: `apps/desktop/tests/harnessIntegration.test.ts`
- Test: Rust route tests plus Electron helper tests

**Interfaces:**
- Consumes: integration handlers/status, `ORKWORKS_OPEN_PLAN_TOKEN` startup secret pattern.
- Produces: generic GET/POST/DELETE routes and preload `getHarnessIntegrationStatus(id)` / `requestHarnessIntegrationMutation(id, operation)` where mutation always invokes main-owned confirmation.

- [ ] **Step 1: Write route and Electron boundary tests first**

Rust tests cover GET without auth, POST/DELETE missing/wrong/correct auth, auth rejection before handler lookup/filesystem access, unknown ID, unsupported handler, workspace switch, and persistence error. Electron tests inject fetch/dialog fakes and assert cancellation sends no mutation request and acceptance adds the header only in main.

```ts
test("cancellation never attaches authority or calls mutation route", async () => {
  let mutationCalls = 0;
  const absentCodexStatus: HarnessIntegrationStatus = {
    harnessId: "codex",
    enabled: true,
    toolDetected: true,
    registration: "absent",
    ownership: "none",
    activation: "needs_trust",
    coverage: "full",
    diagnostics: [],
    confirmation: {
      toolName: "Codex",
      workspaceLabel: "workspace",
      coverageSummary: "Lifecycle and attention",
      relativePaths: [".codex/hooks.json"],
      executableCodeWarning: true,
    },
  };
  const deps: IntegrationDeps = {
    baseUrl: "http://127.0.0.1:1",
    token: "secret",
    fetch: async (_input, init) => {
      if (init?.method === "POST" || init?.method === "DELETE") mutationCalls += 1;
      return new Response(JSON.stringify(absentCodexStatus), { status: 200 });
    },
    confirm: async () => ({ response: 1, checkboxChecked: false }),
  };
  const result = await requestHarnessIntegrationMutation(deps, "codex", "install");
  assert.equal(result.cancelled, true);
  assert.equal(mutationCalls, 0);
});
```

- [ ] **Step 2: Run focused boundary tests and verify failure**

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml integration_handlers
cd apps/desktop && rtk node --experimental-strip-types --test tests/harnessIntegration.test.ts
```

Expected: FAIL because generic routes/main helper do not exist.

- [ ] **Step 3: Register the generic routes with mutation auth**

Add:

```text
GET    /workspace/harness-integrations/:harnessId
POST   /workspace/harness-integrations/:harnessId
DELETE /workspace/harness-integrations/:harnessId
```

GET returns the full sanitized status. POST/DELETE compare `x-orkworks-integration-token` in constant-time-compatible fixed-length bytes before resolving a handler or reading a target. Reuse the per-sidecar random token but give the header a distinct name; continue excluding it from PTY child environments.

- [ ] **Step 4: Move confirmation and authority into Electron main**

`harnessIntegration.ts` exports:

```ts
export async function getHarnessIntegrationStatus(deps: IntegrationDeps, harnessId: string): Promise<HarnessIntegrationStatus>;
export async function requestHarnessIntegrationMutation(
  deps: IntegrationDeps,
  harnessId: string,
  operation: "install" | "uninstall",
): Promise<HarnessIntegrationMutationResult>;

export interface HarnessIntegrationStatus {
  harnessId: string;
  enabled: boolean;
  toolDetected: boolean;
  registration: "unsupported" | "absent" | "installed" | "drifted" | "error";
  ownership: "none" | "ork_works" | "ambiguous";
  activation: "active" | "needs_trust" | "disabled" | "unknown" | "not_applicable";
  coverage: "full" | "limited" | "none";
  diagnostics: Array<{ code: string; message: string; action?: string }>;
  confirmation?: {
    toolName: string;
    workspaceLabel: string;
    coverageSummary: string;
    relativePaths: string[];
    executableCodeWarning: boolean;
  };
}

export interface HarnessIntegrationMutationResult {
  cancelled: boolean;
  status: HarnessIntegrationStatus;
}

export interface IntegrationDeps {
  baseUrl: string;
  token: string;
  fetch: typeof globalThis.fetch;
  confirm: (options: Electron.MessageBoxOptions) => Promise<Electron.MessageBoxReturnValue>;
}
```

The mutation helper GETs sanitized status, builds a native `dialog.showMessageBox` from its confirmation descriptor, returns cancelled without POST/DELETE when rejected, and only then attaches the token. Preload passes only `harnessId` and `operation` to IPC. Remove renderer `window.confirm` and Claude-only IPC methods.

- [ ] **Step 5: Keep Electron/renderer contracts duplicated and sanitized**

Define identical structural types independently in `electron/harnessIntegration.ts` and `src/harnessTypes.ts`; do not import across the boundary. Update `orkworksWindow.d.ts` with only the two harness-ID methods. Tests assert preload source contains no token or path field.

- [ ] **Step 6: Run route, Electron, type, and regression tests; commit**

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml integration_handlers
cd apps/desktop && rtk node --experimental-strip-types --test tests/harnessIntegration.test.ts tests/dockview.test.ts tests/planOpener.test.ts
rtk pnpm exec tsc --noEmit
cd ../.. && rtk git add crates/orkworksd/src/http crates/orkworksd/src/main.rs apps/desktop/electron apps/desktop/src/orkworksWindow.d.ts apps/desktop/tests
rtk git commit -m "feat: secure harness integration mutations"
```

Expected: unauthenticated direct mutation is rejected before disk access; renderer cancellation produces no mutation; plan opening remains green.

---

### Task 9: Migrate Copilot preferences and render generic integration state

**Files:**
- Modify: `apps/desktop/electron/providerTypes.ts`
- Modify: `apps/desktop/electron/settingsMemory.ts`
- Modify: `apps/desktop/tests/electronSettingsMemory.test.ts`
- Modify: `crates/orkworksd/src/metadata.rs`
- Modify: `crates/orkworksd/src/http/session_handlers.rs`
- Modify: `apps/desktop/src/harnessTypes.ts`
- Modify: `apps/desktop/src/api.ts`
- Modify: `apps/desktop/src/App.tsx`
- Modify: `apps/desktop/src/newSessionDialogState.ts`
- Create: `apps/desktop/src/components/HarnessIntegrationRow.tsx`
- Modify: `apps/desktop/src/components/SettingsModal.tsx`
- Modify: `apps/desktop/src/components/NewSessionDialog.tsx`
- Modify: `apps/desktop/src/components/DockviewApp.tsx`
- Modify: `apps/desktop/src/components/SessionListPanel.tsx`
- Modify: `apps/desktop/src/App.css`
- Modify: `apps/desktop/tests/api.test.ts`
- Modify: `apps/desktop/tests/newSessionDialogState.test.ts`
- Modify: `apps/desktop/tests/harnessIntegration.test.ts`

**Interfaces:**
- Consumes: public harness descriptors, lifecycle status IPC, v1 alias/migration revision.
- Produces: restartable `gh-copilot` preference migration and truthful per-active-tool integration rows.

- [ ] **Step 1: Write migration and presentation tests first**

Cover stock provider settings, custom order/state, `peonModel`, conflicting `copilot`, malformed/partial migration, workspace `activeHarnessIds`, historical session `gh-copilot`, every status-axis combination, mutation cancellation/error, and post-mutation status refresh.

```ts
test("legacy provider preference keeps order while renaming gh-copilot", () => {
  const settings = normalizeProviderSettings({ providers: [{ id: "gh-copilot", enabled: false, fallbackOrder: 2 }] });
  assert.equal(settings.providers.find((p) => p.id === "copilot")?.fallbackOrder, 2);
  assert.equal(settings.providers.find((p) => p.id === "copilot")?.enabled, false);
  assert.equal(settings.providers.some((p) => p.id === "gh-copilot"), false);
});
```

- [ ] **Step 2: Run migration/UI tests and verify failure**

```bash
cd apps/desktop && rtk node --experimental-strip-types --test tests/electronSettingsMemory.test.ts tests/harnessIntegration.test.ts tests/newSessionDialogState.test.ts
```

Expected: FAIL because `copilot` is not a valid provider/harness ID and generic status UI does not exist.

- [ ] **Step 3: Make `copilot` migration restartable across owned stores**

Electron settings normalization maps `gh-copilot` to `copilot`, preserving enabled/order/default/override values and `peonModel`, records migration revision 2, and handles an existing `copilot` collision with a diagnostic instead of overwriting. Sidecar workspace memory maps active selection on load/save and preserves historical session metadata through the read-only registry alias. Each component recognizes already-migrated state so interruption between stores is safe.

- [ ] **Step 4: Replace the Claude-only UI with one reusable row**

`HarnessIntegrationRow` accepts:

```ts
export interface HarnessDescriptor {
  id: string;
  name: string;
  defaultModel: string | null;
  models: string[];
  capabilities: string[];
  origin: "builtin" | "custom";
  diagnostics: Array<{ code: string; message: string }>;
}

interface HarnessIntegrationRowProps {
  harness: HarnessDescriptor;
  status: HarnessIntegrationStatus | null;
  busy: boolean;
  onMutate: (operation: "install" | "uninstall") => Promise<void>;
}
```

Render enabled/tool-detected, registration, ownership, activation/trust, coverage, and diagnostics independently. Show install for supported+absent, reconcile for drifted+owned, uninstall for installed/owned, no unsafe action for ambiguous/error, limited copy for Aider/OpenCode as applicable, and unsupported explanation for shell. Never display an absolute path or handler identifier.

Update `listHarnesses()` to return `Promise<HarnessDescriptor[]>` and migrate `App`, new-session state/dialog, Dockview, session list, and Settings props from the old writable `HarnessConfig` projection to `HarnessDescriptor`. Treat `defaultModel: null` as no default and use `capabilities` membership rather than removed boolean/config fields.

- [ ] **Step 5: Refresh authoritative status after every intent**

Settings loads status for each active coding tool, calls the preload mutation-intent method, and always re-fetches that harness status after acceptance, cancellation, or failure. It never optimistically writes `Installed`. A workspace/harness change cancels stale requests using an incrementing request ID.

- [ ] **Step 6: Run all desktop tests/type/build and commit**

```bash
cd apps/desktop
rtk node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
rtk pnpm exec tsc --noEmit
rtk pnpm build
cd ../..
rtk git add apps/desktop crates/orkworksd/src/metadata.rs crates/orkworksd/src/http/session_handlers.rs
rtk git commit -m "feat: show generic coding tool integrations"
```

Expected: desktop tests, type-check, and build pass; Claude-only IPC/UI names are absent; stock/new Copilot launch is interactive.

---

### Task 10: Synchronize docs, run full verification, review, and hand off

**Files:**
- Modify: `docs/agents/architecture.md`
- Modify: `docs/agents/domain-entities.md` only if session metadata fields/vocabulary changed
- Modify: `README.md`
- Modify: `AGENTS.md`
- Modify: `skills/adding-harness/SKILL.md`
- Modify: `docs/agents/harness-integration-contracts.md`
- Modify: umbrella issue and reconciled issue comments/closures

**Interfaces:**
- Consumes: completed implementation and exact final contracts.
- Produces: current contributor/user docs, adding-harness checklist, verified branch, medium review evidence, and PR-ready issue state.

- [ ] **Step 1: Update architecture and harness-authoring guidance**

Document the new module/resource layout, v2 sparse config, catalog publication, generic routes, main-owned confirmation, stable reporter directory, status axes, integration eligibility policy, and `copilot` alias. Update `skills/adding-harness/SKILL.md` so a new harness requires: definition entry; launch/no-model behavior; resume strategies; Peon/model/capacity config; voice pass-through; exact signal fixture; integration status/install/uninstall ownership fixtures; minimum version/feature probe; and truthful unsupported/limited state.

- [ ] **Step 2: Prove documentation and code contain no stale architecture**

Run:

```bash
rtk rg -n 'HarnessAdapter|builtin_harness_configs|builtin_adapters|getClaudeCodeHookStatus|installClaudeCodeHook|gh copilot suggest' crates apps docs README.md AGENTS.md skills/adding-harness
rtk bash .claude/hooks/doc-check.sh
rtk bash .claude/hooks/worktree-check.sh
rtk pnpm --dir docs docs:build
rtk git diff --check
```

Expected: only intentional migration/history references match; doc/worktree checks pass; docs build passes except any explicitly recorded pre-existing generated APM failure.

- [ ] **Step 3: Run the complete verification matrix from a clean build state**

```bash
rtk cargo fmt --manifest-path crates/orkworksd/Cargo.toml -- --check
rtk cargo clippy --manifest-path crates/orkworksd/Cargo.toml --all-targets -- -D warnings
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml
cd apps/desktop
rtk pnpm install --frozen-lockfile
rtk node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
rtk pnpm exec tsc --noEmit
rtk pnpm build
cd ../..
rtk git status --short
```

Expected: all Rust/desktop checks pass and status contains only the intended documentation edits for this task.

- [ ] **Step 4: Commit documentation and request the required code review**

```bash
rtk git add docs/agents README.md AGENTS.md skills/adding-harness/SKILL.md
rtk git commit -m "docs: document harness capability extensions"
```

Run `/code-review` at medium effort or higher. Address every critical/important finding with a new failing test first; document intentional minor findings in the PR description. Re-run the full verification matrix after the final patch.

- [ ] **Step 5: Update issues only from evidence and open the PR**

Mark the umbrella issue acceptance boxes from passing tests. Close #187 and #188 only when sparse override and dead adapter acceptance are proven; close or supersede #23, #71, #103-#105, #107-#108, and #180 only where every criterion is satisfied; leave #106 open/out of scope. Open one PR, include ADR/spec links, migration/security test evidence, code-review disposition, and the known docs-build exception only if it still reproduces on current main.

- [ ] **Step 6: Final currency checks**

```bash
rtk bash .claude/hooks/doc-check.sh
rtk bash .claude/hooks/worktree-check.sh
rtk git status --short
```

Expected: no missing docs, no agent-owned stale worktree, and a clean working tree after the final commit.
