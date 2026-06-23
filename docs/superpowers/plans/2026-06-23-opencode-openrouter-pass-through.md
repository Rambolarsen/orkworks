# OpenCode OpenRouter Pass-Through Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let OrkWorks launch OpenCode sessions with arbitrary model strings, including OpenRouter-backed identifiers, without adding OpenRouter-specific logic to OrkWorks.

**Architecture:** Keep OpenRouter entirely behind the existing `opencode` harness boundary. The backend should preserve the selected model string exactly as launched while avoiding any fake provider metadata for interactive sessions. The frontend should make the session flow clearly harness-centric and show launch context separately from Peon provider metadata.

**Tech Stack:** Rust (`axum`, `serde`), TypeScript/React, Electron preload IPC, Node built-in test runner

---

## File Structure

- `crates/orkworksd/src/main.rs`
  - Session launch resolution, session metadata initialization, and existing Rust tests for `resolve_session_launch`.
- `apps/desktop/src/components/NewSessionDialog.tsx`
  - New Session modal copy and free-form model input behavior.
- `apps/desktop/src/components/SessionDetailPanel.tsx`
  - Selected-session launch context display.
- `apps/desktop/tests/newSessionDialogState.test.ts`
  - Pure state tests for preserving opaque model strings.
- `apps/desktop/tests/dockview.test.ts`
  - Source-based UI assertions for labels and details-panel sections.
- `docs/agents/architecture.md`
  - Architecture note clarifying that interactive harness model strings are pass-through session context, while provider fields remain Peon observation metadata.

### Task 1: Fix Backend Launch Metadata Boundary

**Files:**
- Modify: `crates/orkworksd/src/main.rs`
- Test: `crates/orkworksd/src/main.rs`

- [ ] **Step 1: Write the failing Rust tests for opaque model pass-through and no fake provider context**

Add these tests near the existing `resolve_session_launch_preserves_selected_harness_id_for_generic_shell_configs` test:

```rust
    #[test]
    fn resolve_session_launch_preserves_custom_opencode_model_string() {
        let harnesses = builtin_harness_configs();

        let launch = resolve_session_launch(
            &harnesses,
            &CreateSessionRequest {
                harness_id: Some("opencode".into()),
                model: Some("openrouter/openai/gpt-4.1-mini".into()),
                initial_prompt: None,
            },
            "/repo".into(),
        );

        assert_eq!(launch.session_harness_id.as_deref(), Some("opencode"));
        assert_eq!(launch.adapter_harness_id.as_deref(), Some("opencode"));
        assert_eq!(
            launch.model.as_deref(),
            Some("openrouter/openai/gpt-4.1-mini"),
        );
        assert_eq!(launch.command.program, "opencode");
        assert_eq!(
            launch.command.args,
            vec!["--model=openrouter/openai/gpt-4.1-mini".to_string()],
        );
    }

    #[test]
    fn resolve_session_launch_does_not_infer_provider_context_from_harness() {
        let harnesses = builtin_harness_configs();

        let launch = resolve_session_launch(
            &harnesses,
            &CreateSessionRequest {
                harness_id: Some("opencode".into()),
                model: Some("openrouter/openai/gpt-4.1-mini".into()),
                initial_prompt: None,
            },
            "/repo".into(),
        );

        assert_eq!(launch.provider_id, None);
        assert_eq!(launch.provider_label, None);
    }
```

- [ ] **Step 2: Run the Rust tests to verify the provider-context assertion fails first**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml resolve_session_launch_ -- --nocapture
```

Expected:

```text
running 3 tests
test tests::resolve_session_launch_preserves_custom_opencode_model_string ... ok
test tests::resolve_session_launch_does_not_infer_provider_context_from_harness ... FAILED
```

- [ ] **Step 3: Stop populating provider metadata from the selected harness**

Update `resolve_session_launch` so harness selection only controls harness launch, not provider context:

```rust
fn resolve_session_launch(
    harnesses: &[HarnessConfig],
    req: &CreateSessionRequest,
    cwd: String,
) -> ResolvedSessionLaunch {
    if let Some(ref harness_id) = req.harness_id {
        if let Some(config) = harnesses.iter().find(|h| h.id == *harness_id) {
            let model = req.model.clone().or_else(|| {
                (!config.default_model.is_empty()).then(|| config.default_model.clone())
            });
            let model_value = model.clone().unwrap_or_default();
            let args: Vec<String> = config
                .args
                .iter()
                .filter_map(|arg| {
                    if arg.contains("{model}") && model_value.is_empty() {
                        None
                    } else {
                        Some(arg.replace("{model}", &model_value))
                    }
                })
                .collect();

            return ResolvedSessionLaunch {
                session_harness_id: Some(config.id.clone()),
                adapter_harness_id: Some(config.harness.clone()),
                model,
                command: harness::CommandSpec {
                    program: config.command.clone(),
                    args,
                    cwd,
                },
                provider_id: None,
                provider_label: None,
            };
        }
    }

    ResolvedSessionLaunch {
        session_harness_id: None,
        adapter_harness_id: None,
        model: req.model.clone(),
        command: default_shell_command(cwd),
        provider_id: None,
        provider_label: None,
    }
}
```

Delete the now-unused `provider_context_for_harness` helper entirely instead of leaving dead code behind.

- [ ] **Step 4: Run the Rust tests again to verify both pass**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml resolve_session_launch_ -- --nocapture
```

Expected:

```text
running 3 tests
test tests::resolve_session_launch_preserves_custom_opencode_model_string ... ok
test tests::resolve_session_launch_does_not_infer_provider_context_from_harness ... ok
```

- [ ] **Step 5: Commit the backend boundary fix**

```bash
git add crates/orkworksd/src/main.rs
git commit -m "fix: keep harness launch metadata separate from providers"
```

### Task 2: Make New Session Clearly Harness-Centric

**Files:**
- Modify: `apps/desktop/src/components/NewSessionDialog.tsx`
- Modify: `apps/desktop/tests/newSessionDialogState.test.ts`
- Test: `apps/desktop/tests/dockview.test.ts`

- [ ] **Step 1: Add the failing frontend tests**

Extend `apps/desktop/tests/newSessionDialogState.test.ts` with an explicit opaque-model case:

```ts
test("syncDraftWithHarnesses preserves an existing OpenCode model string verbatim", () => {
  const draft = syncDraftWithHarnesses(
    { harnessId: "opencode", model: "openrouter/openai/gpt-4.1-mini" },
    [harness("opencode", "OpenCode")],
  );

  assert.deepEqual(draft, {
    harnessId: "opencode",
    model: "openrouter/openai/gpt-4.1-mini",
  });
});
```

Extend `apps/desktop/tests/dockview.test.ts` with a source assertion for the dialog copy and free-form model input:

```ts
test("NewSessionDialog uses harness wording and keeps model entry free-form", () => {
  const source = readFileSync(
    new URL("../src/components/NewSessionDialog.tsx", import.meta.url),
    "utf8",
  );

  assert.match(source, />Harness</);
  assert.doesNotMatch(source, />Provider</);
  assert.match(source, /type="text"/);
  assert.match(source, /list="nsd-model-suggestions"/);
});
```

- [ ] **Step 2: Run the targeted frontend tests and verify the label assertion fails**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/newSessionDialogState.test.ts tests/dockview.test.ts
```

Expected:

```text
not ok ... NewSessionDialog uses harness wording and keeps model entry free-form
```

- [ ] **Step 3: Update the dialog copy without restricting model entry**

Change the label in `apps/desktop/src/components/NewSessionDialog.tsx` from `Provider` to `Harness` and leave the model input as a plain text input with datalist suggestions:

```tsx
          <div className="new-session-row">
            <label className="new-session-label" htmlFor="nsd-harness">Harness</label>
            <select
              ref={harnessSelectRef}
              id="nsd-harness"
              className="new-session-select"
              value={draft.harnessId}
              onChange={(e) => handleHarnessChange(e.target.value)}
              disabled={harnesses.length === 0}
            >
```

Keep this part unchanged so manual OpenRouter-style model strings remain valid:

```tsx
            <input
              key={draft.harnessId}
              id="nsd-model"
              className="new-session-input"
              type="text"
              list="nsd-model-suggestions"
              defaultValue={draft.model}
              onChange={(e) => setDraft((current) => ({ ...current, model: e.target.value }))}
              placeholder="default"
            />
```

- [ ] **Step 4: Run the targeted frontend tests again**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/newSessionDialogState.test.ts tests/dockview.test.ts
```

Expected:

```text
ok ... syncDraftWithHarnesses preserves an existing OpenCode model string verbatim
ok ... NewSessionDialog uses harness wording and keeps model entry free-form
```

- [ ] **Step 5: Commit the New Session UI cleanup**

```bash
git add apps/desktop/src/components/NewSessionDialog.tsx apps/desktop/tests/newSessionDialogState.test.ts apps/desktop/tests/dockview.test.ts
git commit -m "feat: clarify harness-driven session model entry"
```

### Task 3: Show Launch Context Separately From Peon Provider Metadata

**Files:**
- Modify: `apps/desktop/src/components/SessionDetailPanel.tsx`
- Modify: `apps/desktop/tests/dockview.test.ts`
- Modify: `docs/agents/architecture.md`

- [ ] **Step 1: Add the failing details-panel test**

Extend `apps/desktop/tests/dockview.test.ts` with a source assertion for launch context labels:

```ts
test("SessionDetailPanel separates launch context from provider observation metadata", () => {
  const source = readFileSync(
    new URL("../src/components/SessionDetailPanel.tsx", import.meta.url),
    "utf8",
  );

  for (const label of ["Harness", "Launch model", "Provider", "Model", "State"]) {
    assert.match(source, new RegExp(`>${label}<`));
  }

  assert.match(source, /const harnessValue = active\.harness \?\? "—"/);
  assert.match(source, /const launchModelValue = active\.model \?\? "—"/);
});
```

- [ ] **Step 2: Run the targeted frontend test and verify the new labels are missing**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/dockview.test.ts
```

Expected:

```text
not ok ... SessionDetailPanel separates launch context from provider observation metadata
```

- [ ] **Step 3: Add launch-context fields to the details panel**

Update `apps/desktop/src/components/SessionDetailPanel.tsx` to show the selected harness and exact launched model separately from provider observation data:

```tsx
  const providerContext = sessionProviderContext(active);
  const harnessValue = active.harness ?? "—";
  const launchModelValue = active.model ?? "—";
```

Render the two new sections before the existing provider block:

```tsx
      <div className="session-detail-section">
        <div className="session-detail-label">Harness</div>
        <div className="session-detail-value">{harnessValue}</div>
      </div>

      <div className="session-detail-section">
        <div className="session-detail-label">Launch model</div>
        <div className="session-detail-value">{launchModelValue}</div>
      </div>

      <div className="session-detail-section">
        <div className="session-detail-label">Provider</div>
        <div className="session-detail-value">{providerContext.provider}</div>
      </div>
```

- [ ] **Step 4: Update the architecture note for this boundary**

Add this paragraph under the `Frontend → backend API` or `Dockview panel layout` section in `docs/agents/architecture.md`:

```md
The New Session flow treats harness model strings as pass-through launch context. OpenCode sessions may use arbitrary model identifiers, including OpenRouter-backed names, and OrkWorks stores that exact string on the session as `model` without interpreting the upstream provider. The session details panel shows harness and launch model separately from `Provider` / `Model` / `State`, which remain Peon observation metadata.
```

- [ ] **Step 5: Run the targeted frontend tests again**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/dockview.test.ts
```

Expected:

```text
ok ... SessionDetailPanel separates launch context from provider observation metadata
```

- [ ] **Step 6: Manually verify remembered-session and resume behavior**

After the code changes are in place, run the desktop app and verify the exact model string survives the full session lifecycle:

```text
1. Start a new OpenCode session with model "openrouter/openai/gpt-4.1-mini".
2. Confirm the Sessions list shows "opencode (openrouter/openai/gpt-4.1-mini)".
3. End or kill the session so it becomes remembered.
4. Select the remembered session and confirm Session Details shows:
   - Harness: opencode
   - Launch model: openrouter/openai/gpt-4.1-mini
5. Resume the session and confirm the same Launch model value is still present.
```

Expected:

```text
The exact model string remains unchanged in live, remembered, and resumed views.
```

- [ ] **Step 7: Run the broader verification for desktop and Rust**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
```

Expected:

```text
# all targeted frontend tests pass
```

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml
```

Expected:

```text
test result: ok.
```

Run:

```bash
bash .claude/hooks/doc-check.sh
```

Expected:

```text
# no additional stale-doc warnings for this slice
```

- [ ] **Step 8: Commit the details-panel and doc update**

```bash
git add apps/desktop/src/components/SessionDetailPanel.tsx apps/desktop/tests/dockview.test.ts docs/agents/architecture.md
git commit -m "feat: show harness launch context in session details"
```
