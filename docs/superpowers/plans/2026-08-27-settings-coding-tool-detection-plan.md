# Settings Coding-Tool Detection Status Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Keep every Coding tools row's `Detected` / `Not detected` status in its top row, remove the duplicate expanded status, and refresh the row after integration mutations.

**Architecture:** `SettingsModal` remains the owner of the per-harness refresh generation. `HarnessDetectionStatus` receives that generation and owns the row-level probe state; `HarnessIntegrationSection` keeps its local integration details but reports detection-changing mutations to the parent. CSS makes the header a single, readable flex row with the toggle pinned to the right.

**Tech Stack:** React 19, TypeScript, CSS, Node's built-in test runner, pnpm.

## Global Constraints

- Keep the existing `getHarnessIntegrationStatus(harnessId)` API and detection semantics unchanged.
- Do not change backend, IPC, persistence, or settings-schema behavior.
- Use the existing `HarnessDetectionStatus` component as the sole rendered detection indicator.
- Keep the user-facing term `Coding tool`; do not introduce new fantasy naming.
- Use `pnpm` for Node package-management tasks.
- Run `bash scripts/doc-check.sh` before handoff.

---

### Task 1: Add the detection refresh contract and row-state tests

**Files:**
- Modify: `apps/desktop/src/components/HarnessDetectionStatus.tsx`
- Modify: `apps/desktop/src/components/HarnessIntegrationSection.tsx`
- Test: `apps/desktop/tests/providersPanel.test.ts`

**Interfaces:**
- `HarnessDetectionStatus` consumes `{ harnessId: string; refreshGeneration?: number }` and re-probes when either `harnessId` or `refreshGeneration` changes.
- `HarnessIntegrationSection` consumes `{ harnessId: string; harnessName: string; harness: HarnessConfig | undefined; onDetectionChanged?: (harnessId: string) => void }`.
- Integration mutations call `onDetectionChanged?.(harnessId)` only after a successful install, uninstall, save-custom-path, or clear-custom-path operation.

- [ ] **Step 1: Add failing source-contract tests**

Add tests to `apps/desktop/tests/providersPanel.test.ts` that assert:

```ts
test("HarnessDetectionStatus supports parent-triggered refresh and accessible status text", () => {
  const source = readFileSync(new URL("../src/components/HarnessDetectionStatus.tsx", import.meta.url), "utf8");
  assert.match(source, /refreshGeneration/);
  assert.match(source, /Coding tool detection status/);
  assert.match(source, /aria-live="polite"/);
});

test("HarnessIntegrationSection reports successful detection-changing mutations", () => {
  const source = readFileSync(new URL("../src/components/HarnessIntegrationSection.tsx", import.meta.url), "utf8");
  assert.match(source, /onDetectionChanged/);
  assert.match(source, /onDetectionChanged\?\.\(harnessId\)/);
});
```

- [ ] **Step 2: Run the focused tests and verify they fail**

Run from `apps/desktop/`:

```bash
node --experimental-strip-types --test tests/providersPanel.test.ts
```

Expected: FAIL because the new prop, callback, and accessibility contract are not present.

- [ ] **Step 3: Implement the minimal refresh and callback contract**

In `HarnessDetectionStatus.tsx`:

```tsx
interface HarnessDetectionStatusProps {
  harnessId: string;
  refreshGeneration?: number;
}

export default function HarnessDetectionStatus({ harnessId, refreshGeneration = 0 }: HarnessDetectionStatusProps) {
  // Keep the existing cancellation behavior and add refreshGeneration to the effect dependency list.
  // Reset to loading before each request so a mutation never leaves an old result presented as current.
}
```

Render the existing text with an accessible label and polite status semantics, for example:

```tsx
<span
  className="harness-detection-status"
  role="status"
  aria-live="polite"
  aria-label={`Coding tool detection status: ${text}`}
>
```

In `HarnessIntegrationSection.tsx`, add the callback prop and invoke it after each successful mutation, while retaining the component's existing local `setIntegration(...)` updates. Remove only the rendered `Detected` / `Not detected` block; keep integration-specific diagnostics and registration messages.

- [ ] **Step 4: Run the focused tests and verify they pass**

```bash
node --experimental-strip-types --test tests/providersPanel.test.ts
```

Expected: PASS.

- [ ] **Step 5: Commit the contract slice**

```bash
git add apps/desktop/src/components/HarnessDetectionStatus.tsx apps/desktop/src/components/HarnessIntegrationSection.tsx apps/desktop/tests/providersPanel.test.ts
git commit -m "feat: refresh coding tool detection status"
```

### Task 2: Wire one status into every Coding tools row

**Files:**
- Modify: `apps/desktop/src/components/SettingsModal.tsx`
- Test: `apps/desktop/tests/providersPanel.test.ts`
- Test: `apps/desktop/tests/dockview.test.ts`

**Interfaces:**
- `SettingsModal` owns `Record<string, number>` detection generations and passes the matching value to the row indicator and integration section.
- `HarnessIntegrationSection` notifies `SettingsModal` through `onDetectionChanged` after successful detection-changing mutations.

- [ ] **Step 1: Add failing Settings wiring assertions**

Add assertions that the Coding tools map contains one row-level indicator regardless of active state and passes the same refresh generation and callback to the two child components:

```ts
test("SettingsModal keeps detection status in every Coding tools row", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(source, /HarnessDetectionStatus harnessId=\{h\.id\}/);
  assert.match(source, /refreshGeneration=/);
  assert.match(source, /onDetectionChanged=/);
  assert.doesNotMatch(source, /activeDraft\.includes\(h\.id\).*HarnessDetectionStatus/);
});
```

Update the existing integration test to require the parent callback on `HarnessIntegrationSection`.

- [ ] **Step 2: Run focused tests and verify the new assertions fail**

```bash
node --experimental-strip-types --test tests/providersPanel.test.ts tests/dockview.test.ts
```

Expected: FAIL because the current status is conditionally omitted for enabled tools and no parent refresh generation exists.

- [ ] **Step 3: Implement parent-owned generations and unconditional row rendering**

In `SettingsModal.tsx`:

```tsx
const [detectionGenerations, setDetectionGenerations] = useState<Record<string, number>>({});

function refreshDetection(harnessId: string) {
  setDetectionGenerations((current) => ({
    ...current,
    [harnessId]: (current[harnessId] ?? 0) + 1,
  }));
}
```

Inside each coding-tool row header, render `HarnessDetectionStatus` unconditionally next to the tool name:

```tsx
<HarnessDetectionStatus
  harnessId={h.id}
  refreshGeneration={detectionGenerations[h.id] ?? 0}
/>
```

Pass the same generation and callback to the expanded integration section. Remove the old conditional comment and conditional status branch; keep the integration section conditional only on the tool being an integration harness and active.

- [ ] **Step 4: Run focused tests and verify they pass**

```bash
node --experimental-strip-types --test tests/providersPanel.test.ts tests/dockview.test.ts
```

Expected: PASS.

- [ ] **Step 5: Commit the Settings wiring slice**

```bash
git add apps/desktop/src/components/SettingsModal.tsx apps/desktop/tests/providersPanel.test.ts apps/desktop/tests/dockview.test.ts
git commit -m "feat: keep detection status in coding tool rows"
```

### Task 3: Make the top row robust and verify the desktop change

**Files:**
- Modify: `apps/desktop/src/App.css`
- Modify: `apps/desktop/src/components/SettingsModal.tsx` (only if needed for a class or accessible markup)
- Test: `apps/desktop/tests/dockview.test.ts`

**Interfaces:**
- The existing `.settings-config-item-header`, `.settings-config-item`, `.harness-detection-status`, and toggle markup remain the styling seam; no new layout dependency is introduced.

- [ ] **Step 1: Add a failing layout-contract test**

Add a source-level CSS assertion in `apps/desktop/tests/dockview.test.ts` that checks the header is a non-wrapping flex row and the toggle remains separated from the identity/status group:

```ts
test("Coding tool headers keep detection status and toggle on one top row", () => {
  const css = readFileSync(new URL("../src/App.css", import.meta.url), "utf8");
  const header = css.match(/\.settings-config-item-header\s*\{([^}]*)\}/)?.[1] ?? "";
  assert.match(header, /display:\s*flex/);
  assert.match(header, /align-items:\s*center/);
  assert.match(header, /justify-content:\s*space-between/);
  assert.match(header, /flex-wrap:\s*nowrap/);
});
```

- [ ] **Step 2: Run the layout test and verify it fails**

```bash
node --experimental-strip-types --test tests/dockview.test.ts
```

Expected: FAIL until the header explicitly defines the single-row layout.

- [ ] **Step 3: Implement the focused CSS layout**

Add or update the relevant selectors so the header and identity group remain one row, the toggle has stable space, and the status text does not wrap into the expanded body:

```css
.settings-config-item-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: nowrap;
}

.settings-config-item {
  min-width: 0;
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.harness-detection-status {
  flex-shrink: 0;
  white-space: nowrap;
}
```

Keep the existing right-aligned toggle and color semantics. If the current selectors already provide any of these declarations, retain them and add only the missing declarations.

- [ ] **Step 4: Run the full desktop verification**

From `apps/desktop/`:

```bash
npx tsc --noEmit
node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
```

Expected: type-check succeeds and all desktop tests pass.

From the repository root:

```bash
bash scripts/doc-check.sh
bash .claude/hooks/worktree-check.sh
```

Expected: documentation drift check reports no unaddressed trigger; worktree check completes with any unrelated `.serena/` state left untouched.

- [ ] **Step 5: Review the final diff and commit the layout slice**

```bash
git diff --check
git status --short
git add apps/desktop/src/App.css apps/desktop/tests/dockview.test.ts
git commit -m "style: stabilize coding tool detection row"
```

