# Harness Version Status Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ensure installed integrations with incompatible coding-tool versions report an honest unknown state without misleading users to run `/hooks` or claiming the integration is active.

**Architecture:** Preserve the existing `IntegrationActivation::Unknown` protocol value and `unsupported_tool_version` diagnostic. Change the shared JSON-hook handler and OpenCode handler to use that state for incompatible versions, then make the renderer suppress both trust and success confirmation while that diagnostic is present.

**Tech Stack:** Rust/Axum sidecar, serde JSON integration status, React/TypeScript renderer, Node test runner, Cargo tests.

## Global Constraints

- Do not add a new activation enum variant or protocol field.
- Keep Codex's compatible-version `NeedsTrust` behavior unchanged.
- Preserve the existing `unsupported_tool_version` diagnostic and message.
- Do not add voice, microphone, audio capture, or unrelated harness changes.
- Use pnpm for desktop package tasks and Cargo for sidecar tasks.

---

### Task 1: Correct shared JSON-hook version status

**Files:**
- Modify: `crates/orkworksd/src/http/integration_handlers.rs:464-513`
- Modify: `crates/orkworksd/src/harness/integrations/mod.rs:250-268`

**Interfaces:**
- Consumes: existing `IntegrationContext`, `DetectedTool`, and `IntegrationActivation` types.
- Produces: incompatible installed JSON-hook integrations serialize `activation` as `"unknown"` while retaining the existing diagnostic.

- [ ] **Step 1: Extend the existing HTTP regression setup to cover an installed fragment and define the desired state**

In `min_version_gating_marks_a_below_threshold_binary_as_needing_trust`, install the Copilot integration before requesting status, rename the test to `min_version_gating_marks_an_installed_below_threshold_binary_as_unknown`, and change the expected activation to:

```rust
assert_eq!(body["activation"], "unknown");
```

Keep the existing assertion that a diagnostic has code `unsupported_tool_version`.

- [ ] **Step 2: Run the focused test and verify it fails for the intended reason**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml min_version_gating_marks_an_installed_below_threshold_binary_as_unknown -- --exact
```

Expected: FAIL because the current shared handler returns `needs_trust`.

- [ ] **Step 3: Change the shared handler's incompatible-version branch**

In `JsonHookHandler::status_from_document`, retain the diagnostic push and replace only the branch result:

```rust
} else if ctx.detected_tool.is_some_and(|tool| !tool.compatible) {
    diagnostics.push(IntegrationDiagnostic {
        code: "unsupported_tool_version".into(),
        message: "The detected coding tool version is not eligible for this integration."
            .into(),
        action: None,
    });
    IntegrationActivation::Unknown
```

- [ ] **Step 4: Run the focused test and verify it passes**

Run the same focused Cargo test. Expected: PASS.

- [ ] **Step 5: Update the generic integration unit expectation**

In the existing `resolved_generic_shell_status...` test in `crates/orkworksd/src/harness/integrations/mod.rs`, change the incompatible Gemini activation expectation from `IntegrationActivation::NeedsTrust` to `IntegrationActivation::Unknown`. Keep its absent-registration assertion: the activation diagnostic is still tested independently of registration.

- [ ] **Step 6: Run the shared integration tests**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml harness::integrations
cargo test --manifest-path crates/orkworksd/Cargo.toml http::integration_handlers
```

Expected: PASS.

- [ ] **Step 7: Commit the shared-handler change**

```bash
git add crates/orkworksd/src/harness/integrations/mod.rs crates/orkworksd/src/http/integration_handlers.rs
git commit -m "fix: report unsupported harness versions as unknown"
```

### Task 2: Correct OpenCode and add direct regression coverage

**Files:**
- Modify: `crates/orkworksd/src/harness/integrations/opencode.rs:130-142, 189-end`

**Interfaces:**
- Consumes: OpenCode's existing `status_from_bytes`, `IntegrationContext`, and `DetectedTool` test helpers.
- Produces: an installed OpenCode plugin with an incompatible detected version reports `Unknown` plus `unsupported_tool_version`.

- [ ] **Step 1: Add a failing OpenCode test for the installed incompatible state**

Add a unit test beside the existing OpenCode integration tests that:

1. Creates a gitignored workspace and resolver using the existing helpers.
2. Installs the handler so `.opencode/plugins/orkworks-session-reporter.js` is present.
3. Constructs `DetectedTool { executable: PathBuf::from("opencode"), version: Some("unsupported".into()), compatible: false }`.
4. Builds a context with `detected_tool: Some(&detected)`.
5. Calls `HANDLER.status(&ctx)` and asserts:

```rust
assert_eq!(status.registration, IntegrationRegistration::Installed);
assert_eq!(status.activation, IntegrationActivation::Unknown);
assert!(status.diagnostics.iter().any(|d| d.code == "unsupported_tool_version"));
```

- [ ] **Step 2: Run the new test and verify it fails**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml opencode::tests::status_reports_unknown_for_installed_unsupported_version -- --exact
```

Expected: FAIL because OpenCode currently returns `NeedsTrust`.

- [ ] **Step 3: Change OpenCode's incompatible-version branch**

In `status_from_bytes`, keep the diagnostic and replace only:

```rust
IntegrationActivation::NeedsTrust
```

with:

```rust
IntegrationActivation::Unknown
```

- [ ] **Step 4: Run the OpenCode test and the full sidecar test suite**

Run the focused test, then:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml
```

Expected: PASS with no new warnings.

- [ ] **Step 5: Commit the OpenCode change**

```bash
git add crates/orkworksd/src/harness/integrations/opencode.rs
git commit -m "fix: report unsupported OpenCode versions as unknown"
```

### Task 3: Suppress misleading renderer confirmation

**Files:**
- Modify: `apps/desktop/src/components/HarnessIntegrationSection.tsx:118-132`
- Modify: `apps/desktop/src/harnessIntegrationPresentation.ts`
- Modify: `apps/desktop/tests/harnessIntegrationSection.test.ts`

**Interfaces:**
- Consumes: the existing `IntegrationStatus.diagnostics` array and `unsupported_tool_version` code.
- Produces: an installed unsupported integration keeps the uninstall action and diagnostic message, but renders neither the `/hooks` trust text nor the normal success confirmation.

- [ ] **Step 1: Add a failing presentation regression test**

In `apps/desktop/tests/harnessIntegrationSection.test.ts`, import a new `shouldShowInstalledConfirmation` helper and add tests for both outcomes:

```ts
test("unsupported tool versions suppress installed confirmation", () => {
  assert.equal(
    shouldShowInstalledConfirmation({
      diagnostics: [{ code: "unsupported_tool_version", message: "unsupported" }],
    }),
    false,
  );
  assert.equal(
    shouldShowInstalledConfirmation({ diagnostics: [] }),
    true,
  );
});
```

The helper should accept only the diagnostic slice needed for this decision, so this test does not need React or a DOM renderer.

- [ ] **Step 2: Run the focused desktop test and verify it fails**

Run from `apps/desktop/`:

```bash
node --experimental-strip-types --test tests/harnessIntegrationSection.test.ts
```

Expected: FAIL because the helper does not exist yet.

- [ ] **Step 3: Add the renderer guard**

In `apps/desktop/src/harnessIntegrationPresentation.ts`, add:

```ts
export function shouldShowInstalledConfirmation(
  diagnostics: ReadonlyArray<{ code: string }>,
): boolean {
  return !diagnostics.some((diagnostic) => diagnostic.code === "unsupported_tool_version");
}
```

Import the helper in `HarnessIntegrationSection.tsx`. In the installed-registration branch, render the current `needs_trust`/success confirmation only when `shouldShowInstalledConfirmation(integration.status.diagnostics)` is true; leave the Uninstall button rendered. The existing diagnostics block below continues to display the authoritative version message. This retains the existing source-level assertions for `/hooks` and success copy while the new helper test pins the actual decision.

- [ ] **Step 4: Run the focused desktop test and type-check**

Run:

```bash
node --experimental-strip-types --test tests/harnessIntegrationSection.test.ts tests/providersPanel.test.ts
npx tsc --noEmit
```

Expected: PASS.

- [ ] **Step 5: Run the desktop test suite**

```bash
node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
```

Expected: PASS.

- [ ] **Step 6: Commit the renderer change**

```bash
git add apps/desktop/src/components/HarnessIntegrationSection.tsx apps/desktop/src/harnessIntegrationPresentation.ts apps/desktop/tests/harnessIntegrationSection.test.ts
git commit -m "fix: hide integration confirmation for unsupported versions"
```

### Task 4: Final verification and handoff

**Files:**
- Verify: `docs/superpowers/specs/2026-08-25-harness-version-status-design.md`
- Verify: `docs/superpowers/plans/2026-08-25-harness-version-status.md`

- [ ] **Step 1: Run repository-level checks**

```bash
cargo fmt --manifest-path crates/orkworksd/Cargo.toml -- --check
cargo test --manifest-path crates/orkworksd/Cargo.toml
cd apps/desktop && npx tsc --noEmit
cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
```

- [ ] **Step 2: Run required repository hygiene checks**

From the repository root:

```bash
bash .claude/hooks/doc-check.sh
bash .claude/hooks/worktree-check.sh
rtk git diff --check
```

- [ ] **Step 3: Review the final diff**

Confirm the diff contains only the shared/OpenCode activation changes, the focused renderer guard and tests, and the approved design/plan documents. Confirm no voice implementation or unrelated refactor was introduced.

- [ ] **Step 4: Request code review before PR handoff**

Run the repository’s required manual `/code-review` gate for changes under `apps/desktop/` and `crates/orkworksd/`, then address every finding or document why it is intentional.
