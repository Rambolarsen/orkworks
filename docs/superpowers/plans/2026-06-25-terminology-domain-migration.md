# Terminology Domain Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Update OrkWorks terminology so CLI coding applications are shown as `Coding tool`, inference services are shown as `Model provider`, agent work is described as `Agent session` where it helps clarity, and existing harness/provider metadata remains compatible.

**Architecture:** Keep `Harness` as the internal coding-tool abstraction, tighten shared types so inference-service concepts use `ModelProvider` where practical, and preserve compatibility for existing `harness`, `provider`, and `model` metadata fields through aliases or read precedence instead of blind renames. Update UI copy, selected frontend/backend types, serialization boundaries, tests, and docs without changing session execution behavior.

**Tech Stack:** React/TypeScript renderer, Electron preload/settings types, Rust sidecar DTOs and metadata handling, Node built-in test runner, cargo test, pnpm type-check.

---

### Task 1: Add Terminology Characterization Tests

**Files:**
- Modify: `apps/desktop/tests/providersPanel.test.ts`
- Modify: `apps/desktop/tests/peonModelPicker.test.ts`
- Create: `apps/desktop/tests/terminology.test.ts`

- [ ] **Step 1: Write failing copy tests**

Create `apps/desktop/tests/terminology.test.ts`:

```typescript
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

function source(path: string): string {
  return readFileSync(new URL(path, import.meta.url), "utf8");
}

test("NewSessionDialog labels the CLI selector as Coding tool", () => {
  const text = source("../src/components/NewSessionDialog.tsx");
  assert.match(text, />Coding tool</);
  assert.doesNotMatch(text, /htmlFor="nsd-harness">Provider</);
});

test("NewSessionDialog does not describe the initial prompt as sent to the provider", () => {
  const text = source("../src/components/NewSessionDialog.tsx");
  assert.match(text, /sent when the agent session starts/);
  assert.doesNotMatch(text, /sent to the provider on start/);
});

test("SessionDetailPanel distinguishes coding tool from model provider", () => {
  const text = source("../src/components/SessionDetailPanel.tsx");
  assert.match(text, />Coding tool</);
  assert.match(text, />Model provider</);
  assert.match(text, />Provider state</);
  assert.doesNotMatch(text, />Provider</);
  assert.doesNotMatch(text, />State</);
});

test("Settings provider copy refers to model providers", () => {
  const modal = source("../src/components/SettingsModal.tsx");
  const section = source("../src/components/ProviderSettingsSection.tsx");
  assert.match(modal, /Model providers/);
  assert.match(section, /Model provider/);
});
```

- [ ] **Step 2: Update existing source-level tests to the target copy**

In `apps/desktop/tests/providersPanel.test.ts`, change the SettingsModal copy assertion from:

```typescript
assert.match(source, /Providers/);
```

to:

```typescript
assert.match(source, /Model providers/);
```

In `apps/desktop/tests/peonModelPicker.test.ts`, change:

```typescript
assert.match(source, /Peon Model/);
```

to:

```typescript
assert.match(source, /Peon model/);
```

- [ ] **Step 3: Run tests to verify they fail**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/terminology.test.ts tests/providersPanel.test.ts tests/peonModelPicker.test.ts
```

Expected: failures showing current copy still uses `Provider`, `Providers`, `State`, and `Peon Model`.

- [ ] **Step 4: Commit tests**

```bash
git add apps/desktop/tests/terminology.test.ts apps/desktop/tests/providersPanel.test.ts apps/desktop/tests/peonModelPicker.test.ts
git commit -m "test(ui): lock terminology boundary"
```

### Task 2: Update Renderer UI Copy

**Files:**
- Modify: `apps/desktop/src/components/NewSessionDialog.tsx`
- Modify: `apps/desktop/src/components/SessionDetailPanel.tsx`
- Modify: `apps/desktop/src/components/SettingsModal.tsx`
- Modify: `apps/desktop/src/components/ProviderSettingsSection.tsx`

- [ ] **Step 1: Update New Session copy**

In `apps/desktop/src/components/NewSessionDialog.tsx`, change:

```tsx
<h2 id="new-session-title">New Session</h2>
```

to:

```tsx
<h2 id="new-session-title">New agent session</h2>
```

Change:

```tsx
<label className="new-session-label" htmlFor="nsd-harness">Provider</label>
```

to:

```tsx
<label className="new-session-label" htmlFor="nsd-harness">Coding tool</label>
```

Change:

```tsx
placeholder="Optional — sent to the provider on start"
```

to:

```tsx
placeholder="Optional - sent when the agent session starts"
```

- [ ] **Step 2: Update session details copy**

In `apps/desktop/src/components/SessionDetailPanel.tsx`, change:

```tsx
return <EmptyState message="Select a session to see details." />;
```

to:

```tsx
return <EmptyState message="Select an agent session to see details." />;
```

Change the `Provider` label for `providerContext.provider` to:

```tsx
<div className="session-detail-label">Coding tool</div>
```

Change the `Model` label for `providerContext.model` to:

```tsx
<div className="session-detail-label">Model provider</div>
```

Change the `State` label for `providerContext.state` to:

```tsx
<div className="session-detail-label">Provider state</div>
```

- [ ] **Step 3: Update Settings provider copy**

In `apps/desktop/src/components/SettingsModal.tsx`, replace visible settings headings:

```tsx
Providers
Peon Model
```

with:

```tsx
Model providers
Peon model
```

In `apps/desktop/src/components/ProviderSettingsSection.tsx`, replace visible generic provider text with `Model provider` or `model provider` where the copy describes inference fallback, enabled state, model listing, or provider override controls.

- [ ] **Step 4: Run focused UI terminology tests**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/terminology.test.ts tests/providersPanel.test.ts tests/peonModelPicker.test.ts
```

Expected: all tests pass.

- [ ] **Step 5: Commit UI copy**

```bash
git add apps/desktop/src/components/NewSessionDialog.tsx apps/desktop/src/components/SessionDetailPanel.tsx apps/desktop/src/components/SettingsModal.tsx apps/desktop/src/components/ProviderSettingsSection.tsx
git commit -m "fix(ui): clarify coding tool and model provider copy"
```

### Task 3: Add Compatibility Coverage For Shared Types And Session Metadata

**Files:**
- Modify: `apps/desktop/src/api.ts`
- Modify: `apps/desktop/tests/api.test.ts`
- Modify: `crates/orkworksd/src/main.rs`
- Modify: `crates/orkworksd/src/providers.rs`
- Modify: `crates/orkworksd/src/metadata.rs`
- Modify: Rust tests in the touched backend modules

- [ ] **Step 1: Add failing compatibility tests**

Extend desktop and Rust tests so they prove the migration boundary instead of only checking copy:

```typescript
test("SessionInfo accepts canonical harnessId/modelProviderId/modelId fields", () => {
  const session: SessionInfo = {
    id: "session-1",
    label: "Test session",
    harnessId: "opencode",
    modelProviderId: "openrouter",
    modelId: "deepseek/deepseek-reasoner",
    status: "running",
    cwd: "/tmp/project",
    created_at: "2026-06-25T10:00:00Z",
    memoryState: "live",
    resumeStrategy: "none",
  };
  assert.equal(session.harnessId, "opencode");
  assert.equal(session.modelProviderId, "openrouter");
  assert.equal(session.modelId, "deepseek/deepseek-reasoner");
});
```

Add Rust coverage for legacy/new field precedence in the session metadata reader:

```rust
assert_eq!(session.harness_id.as_deref(), Some("opencode"));
assert_eq!(session.model_provider_id.as_deref(), Some("openrouter"));
assert_eq!(session.model_id.as_deref(), Some("deepseek/deepseek-reasoner"));
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/api.test.ts
cargo test --manifest-path crates/orkworksd/Cargo.toml metadata
```

Expected: failures because canonical `harnessId` / `modelProviderId` / `modelId` fields are not yet recognized everywhere.

- [ ] **Step 3: Implement compatibility-safe type and schema changes**

Make these changes with minimal churn:

- In `apps/desktop/src/api.ts`, add optional canonical fields `harnessId`, `modelProviderId`, and `modelId` to `SessionInfo` while retaining legacy `harness`, `provider`, and `model`.
- In backend DTOs and metadata readers, accept both legacy and canonical field names with documented precedence.
- Keep existing endpoints such as `/harnesses` and `/providers` unless a concrete compatibility benefit justifies anything broader.
- Only rename internal `Provider*` abstractions where they actually model inference services rather than coding tools.

- [ ] **Step 4: Re-run focused compatibility tests**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/api.test.ts
cargo test --manifest-path crates/orkworksd/Cargo.toml metadata
```

Expected: canonical and legacy fields both load correctly.

- [ ] **Step 5: Commit compatibility work**

```bash
git add apps/desktop/src/api.ts apps/desktop/tests/api.test.ts crates/orkworksd/src/main.rs crates/orkworksd/src/providers.rs crates/orkworksd/src/metadata.rs
git commit -m "feat: add compatibility-safe terminology schema aliases"
```

### Task 4: Update Documentation

**Files:**
- Modify: `README.md`
- Modify: `AGENTS.md`
- Modify: `docs/agents/architecture.md`
- Add: `docs/superpowers/specs/2026-06-25-terminology-domain-migration-design.md`
- Add: `docs/superpowers/plans/2026-06-25-terminology-domain-migration.md`

- [ ] **Step 1: Update README terminology**

In `README.md`, update the feature list so new sessions are launched with a selected `coding tool` in user-facing prose, while preserving `harness config` where it describes the internal file/API:

```markdown
- New agent sessions can be launched with a selected coding tool, optional model override, and optional initial prompt; harness definitions are loaded from the sidecar's built-ins plus `~/.orkworks/harnesses.json`
- Session details show read-only `Coding tool`, `Model provider`, and `Provider state` for the selected session, sourced from session metadata.
```

Add a short terminology note near the naming table:

```markdown
User-facing UI says `Coding tool` for CLI coding applications. Internal code and metadata continue to use `harness` for that integration abstraction. `Model provider` is reserved for inference services and local inference runtimes.
```

- [ ] **Step 2: Update AGENTS.md terminology guidance**

In `AGENTS.md`, add the same terminology boundary to the Key naming section:

```markdown
User-facing UI says `Coding tool` for CLI coding applications. Internal code and metadata continue to use `harness` for that integration abstraction. `Model provider` is reserved for inference services and local inference runtimes.
```

- [ ] **Step 3: Update architecture doc**

In `docs/agents/architecture.md`, update renderer and settings prose so session details are described as `Coding tool`, `Model provider`, and `Provider state`, while backend modules still refer to `harness.rs` and `providers.rs`.

- [ ] **Step 4: Run doc diff check**

Run:

```bash
git diff -- README.md AGENTS.md docs/agents/architecture.md docs/superpowers/specs/2026-06-25-terminology-domain-migration-design.md docs/superpowers/plans/2026-06-25-terminology-domain-migration.md
```

Expected: docs use `Coding tool` for user-facing CLI-app prose and keep `harness` for internal abstraction prose.

- [ ] **Step 5: Commit docs**

```bash
git add README.md AGENTS.md docs/agents/architecture.md docs/superpowers/specs/2026-06-25-terminology-domain-migration-design.md docs/superpowers/plans/2026-06-25-terminology-domain-migration.md
git commit -m "docs: define terminology domain boundary"
```

### Task 5: Verify Behavior Is Unchanged

**Files:**
- No expected source edits unless verification exposes a missed copy string.

- [ ] **Step 1: Run full frontend tests**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
```

Expected: all tests pass.

- [ ] **Step 2: Run Rust tests**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml
```

Expected: all tests pass.

- [ ] **Step 3: Run TypeScript type-check**

Run:

```bash
cd apps/desktop && pnpm exec tsc --noEmit
```

Expected: type-check passes.

- [ ] **Step 4: Run doc currency check**

Run:

```bash
bash .claude/hooks/doc-check.sh
```

Expected: no unresolved required doc updates remain.

- [ ] **Step 5: Commit verification fixes if needed**

If any verification command exposes missed terminology copy or doc updates, fix only those files and commit:

```bash
git add <fixed-files>
git commit -m "fix: complete terminology migration verification"
```
