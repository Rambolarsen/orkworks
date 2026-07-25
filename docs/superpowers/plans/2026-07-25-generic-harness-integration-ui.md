# Generic Harness Integration UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extract `SettingsModal.tsx`'s Claude-Code-only integration UI (Detected badge, Notification-hook install/uninstall, custom-path override) into a reusable `HarnessIntegrationSection` component, then mount it for Gemini CLI and GitHub Copilot CLI too (issue #217).

**Architecture:** One new self-contained component owns all the per-harness state/effects/handlers, parameterized by `harnessId`/`harnessName`/`harness` props instead of hardcoded `"claude-code"`/`"Claude Code"` literals. `SettingsModal.tsx` mounts it for an explicit three-ID allowlist. No backend changes — the routes and IPC bridge are already harness-id-parameterized.

**Tech Stack:** React/TypeScript, Electron IPC (`window.orkworks.*`, already generic). No new dependencies.

**Design doc:** `docs/superpowers/specs/2026-07-25-generic-harness-integration-ui-design.md` (reviewed and fixed — see its commit history).

**Branch:** `settings-gemini-copilot-integration-ui` (already checked out).

**Testing note:** This repo has no React component-render test infrastructure (`apps/desktop/tests/` is Node's built-in test runner over plain logic modules — confirmed by grep, no RTL/jsdom setup exists). This is a pure refactor (move + parameterize existing, already-shipped logic) plus mounting it three times instead of one, not new business logic — there's nothing here a unit test would meaningfully pin that `tsc --noEmit` and a manual browser check don't already cover. Each task's verification is type-checking; Task 4 is an explicit manual, three-harness browser walkthrough in place of an automated test suite.

---

## File Structure

- Create: `apps/desktop/src/components/HarnessIntegrationSection.tsx` — the extracted, parameterized component (state, effects, handlers, `looksAbsolute()`, and the JSX currently at `SettingsModal.tsx:436-532`).
- Modify: `apps/desktop/src/components/SettingsModal.tsx` — delete everything that moved, add the `INTEGRATION_HARNESS_IDS` allowlist, mount `HarnessIntegrationSection` for each allowlisted, active harness.

---

### Task 1: Create `HarnessIntegrationSection.tsx`

**Files:**
- Create: `apps/desktop/src/components/HarnessIntegrationSection.tsx`

No test-first step here (see the Testing note above) — this task moves and parameterizes already-shipped, already-verified logic verbatim; the risk is a transcription mistake, which `tsc --noEmit` (Step 2) and the Task 4 manual walkthrough catch.

- [ ] **Step 1: Write the component**

```tsx
import { useEffect, useState } from "react";
import type { HarnessConfig, IntegrationStatusResult } from "../harnessTypes";

// Mirrors the sole direct-reference condition in the backend probe
// (crates/orkworksd/src/harness/detect.rs::probe_installed_tool): POSIX
// absolute (`/...`), Windows drive-letter (`C:\...` / `C:/...`), or UNC
// (`\\server\...`).
function looksAbsolute(command: string): boolean {
  return command.startsWith("/") || /^[a-zA-Z]:[\\/]/.test(command) || command.startsWith("\\\\");
}

interface HarnessIntegrationSectionProps {
  harnessId: string;
  harnessName: string;
  harness: HarnessConfig | undefined;
}

export default function HarnessIntegrationSection({ harnessId, harnessName, harness }: HarnessIntegrationSectionProps) {
  const launchCommand = harness?.launch.kind === "command-template" ? harness.launch.command : null;
  const hasCustomPath = launchCommand !== null && looksAbsolute(launchCommand);
  const [integration, setIntegration] = useState<IntegrationStatusResult | null>(null);
  const [integrationBusy, setIntegrationBusy] = useState(false);
  const [customPathDraft, setCustomPathDraft] = useState<string>(() =>
    hasCustomPath && launchCommand ? launchCommand : "",
  );
  // Locally owned rather than derived from `hasCustomPath` on every render:
  // the `harness` prop only refreshes when Settings is reopened, so a
  // save/clear updates this immediately instead of leaving the Clear
  // button (and the block's visibility once detection succeeds) stuck
  // showing pre-save state until the modal is closed and reopened.
  const [customPathActive, setCustomPathActive] = useState<boolean>(() => hasCustomPath);
  const [customPathBusy, setCustomPathBusy] = useState(false);
  const [customPathError, setCustomPathError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    window.orkworks.getHarnessIntegrationStatus(harnessId).then((result) => {
      if (!cancelled) setIntegration(result);
    });
    return () => {
      cancelled = true;
    };
  }, [harnessId]);

  async function installIntegrationHandler() {
    setIntegrationBusy(true);
    try {
      setIntegration(await window.orkworks.installHarnessIntegration(harnessId));
    } finally {
      setIntegrationBusy(false);
    }
  }

  async function uninstallIntegrationHandler() {
    setIntegrationBusy(true);
    try {
      setIntegration(await window.orkworks.uninstallHarnessIntegration(harnessId));
    } finally {
      setIntegrationBusy(false);
    }
  }

  async function saveCustomPathHandler() {
    setCustomPathBusy(true);
    setCustomPathError(null);
    try {
      const result = await window.orkworks.setHarnessCommandOverride(harnessId, customPathDraft.trim());
      if (!result.ok) {
        setCustomPathError(result.error);
        return;
      }
      setCustomPathActive(true);
      setIntegration(await window.orkworks.getHarnessIntegrationStatus(harnessId));
    } catch (error) {
      setCustomPathError(error instanceof Error ? error.message : "Couldn't set the custom path.");
    } finally {
      setCustomPathBusy(false);
    }
  }

  async function clearCustomPathHandler() {
    setCustomPathBusy(true);
    setCustomPathError(null);
    try {
      const result = await window.orkworks.clearHarnessCommandOverride(harnessId);
      if (!result.ok) {
        setCustomPathError(result.error);
        return;
      }
      setCustomPathActive(false);
      setCustomPathDraft("");
      setIntegration(await window.orkworks.getHarnessIntegrationStatus(harnessId));
    } catch (error) {
      setCustomPathError(error instanceof Error ? error.message : "Couldn't clear the custom path.");
    } finally {
      setCustomPathBusy(false);
    }
  }

  return (
    <div className="settings-config-item-actions">
      {integration === null && (
        <span className="settings-config-status">checking {harnessName} integration…</span>
      )}
      {integration && !integration.ok && (
        <span className="settings-config-status">{integration.error}</span>
      )}
      {integration?.ok && (
        <span
          className={
            "settings-config-status" +
            (integration.status.toolDetected ? " settings-config-status--ok" : "")
          }
        >
          {integration.status.toolDetected ? "✓ Detected" : "Not detected"}
        </span>
      )}
      {integration?.ok && integration.status.registration === "installed" && (
        <>
          <span className="settings-config-status settings-config-status--ok">✓ Notification hook installed</span>
          <button type="button" onClick={uninstallIntegrationHandler} disabled={integrationBusy}>
            {integrationBusy ? "Removing…" : "Uninstall"}
          </button>
        </>
      )}
      {integration?.ok &&
        (integration.status.registration === "absent" ||
          integration.status.registration === "drifted") && (
          <>
            {integration.status.confirmation && (
              <p className="settings-section-copy">
                Installing will add a Notification hook to{" "}
                {integration.status.confirmation.relativePaths.join(", ")} in this
                workspace ({integration.status.confirmation.coverageSummary}).
                {integration.status.confirmation.executableCodeWarning && (
                  <> This hook runs an OrkWorks-installed script whenever {harnessName}
                  waits for input.</>
                )}
              </p>
            )}
            <button type="button" onClick={installIntegrationHandler} disabled={integrationBusy}>
              {integrationBusy
                ? "Installing…"
                : integration.status.registration === "drifted"
                  ? "Reinstall"
                  : "Install attention hook"}
            </button>
          </>
        )}
      {integration?.ok && integration.status.registration === "unsupported" && (
        <span className="settings-config-status">
          Attention hook isn't supported for this coding tool.
        </span>
      )}
      {integration?.ok && integration.status.diagnostics.length > 0 && (
        <span className="settings-config-status">
          {integration.status.diagnostics[0].message}
        </span>
      )}
      {integration?.ok &&
        (integration.status.diagnostics.some((d) => d.code === "tool_not_detected") ||
          customPathActive) && (
          <div className="settings-config-custom-path">
            <label>
              Custom path
              <input
                type="text"
                value={customPathDraft}
                onChange={(e) => setCustomPathDraft(e.target.value)}
                placeholder="/opt/homebrew/bin/claude"
                disabled={customPathBusy}
              />
            </label>
            <p className="settings-section-copy">
              This also becomes the command OrkWorks launches {harnessName} sessions with —
              make sure it points at the real binary.
            </p>
            <button
              type="button"
              onClick={saveCustomPathHandler}
              disabled={customPathBusy || !looksAbsolute(customPathDraft.trim())}
            >
              {customPathBusy ? "Saving…" : "Save"}
            </button>
            {customPathActive && (
              <button type="button" onClick={clearCustomPathHandler} disabled={customPathBusy}>
                Clear
              </button>
            )}
            {customPathError && (
              <span className="settings-config-status">{customPathError}</span>
            )}
          </div>
        )}
    </div>
  );
}
```

- [ ] **Step 2: Type-check**

Run: `cd apps/desktop && npx tsc --noEmit`
Expected: no errors. (`window.orkworks` resolves via the global `declare global` augmentation in `apps/desktop/src/orkworksWindow.d.ts` — no explicit import needed.)

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src/components/HarnessIntegrationSection.tsx
git commit -m "feat: extract HarnessIntegrationSection from SettingsModal's Claude-only block"
```

---

### Task 2: Wire it into `SettingsModal.tsx` for Claude Code, Gemini, and Copilot

**Files:**
- Modify: `apps/desktop/src/components/SettingsModal.tsx`

- [ ] **Step 1: Add the import and the harness-ID allowlist**

Change:

```typescript
import type { HarnessConfig, IntegrationStatusResult } from "../harnessTypes";
import ProviderSettingsSection from "./ProviderSettingsSection";
```

to:

```typescript
import type { HarnessConfig } from "../harnessTypes";
import ProviderSettingsSection from "./ProviderSettingsSection";
import HarnessIntegrationSection from "./HarnessIntegrationSection";
```

(`IntegrationStatusResult` is dropped — its only use in this file was the state variable removed in Step 3 below.)

Then, immediately after the now-empty spot where `looksAbsolute()` used to be defined (see Step 2 — it's being deleted, not kept here), add the allowlist near the other module-level constants:

```typescript
const INTEGRATION_HARNESS_IDS = ["claude-code", "gemini", "copilot"];
```

Place it right after the `FOCUSABLE` constant:

```typescript
const FOCUSABLE = 'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';
const INTEGRATION_HARNESS_IDS = ["claude-code", "gemini", "copilot"];
```

- [ ] **Step 2: Delete the module-level `looksAbsolute()` helper**

Delete this whole block (it moved into `HarnessIntegrationSection.tsx` in Task 1):

```typescript
// Mirrors the sole direct-reference condition in the backend probe
// (crates/orkworksd/src/harness/detect.rs::probe_installed_tool): POSIX
// absolute (`/...`), Windows drive-letter (`C:\...` / `C:/...`), or UNC
// (`\\server\...`).
function looksAbsolute(command: string): boolean {
  return command.startsWith("/") || /^[a-zA-Z]:[\\/]/.test(command) || command.startsWith("\\\\");
}
```

- [ ] **Step 3: Delete the Claude-specific state**

Find:

```typescript
  const [claudeIntegration, setClaudeIntegration] = useState<IntegrationStatusResult | null>(null);
  const [claudeIntegrationBusy, setClaudeIntegrationBusy] = useState(false);
  const hasClaudeCodeHarness = harnesses.some((h) => h.id === "claude-code");
  const claudeHarness = harnesses.find((h) => h.id === "claude-code");
  const claudeLaunchCommand =
    claudeHarness?.launch.kind === "command-template" ? claudeHarness.launch.command : null;
  const claudeHasCustomPath = claudeLaunchCommand !== null && looksAbsolute(claudeLaunchCommand);
  const [customPathDraft, setCustomPathDraft] = useState<string>(() =>
    claudeHasCustomPath && claudeLaunchCommand ? claudeLaunchCommand : "",
  );
  // Locally owned rather than derived from `claudeHasCustomPath` on every
  // render: the `harnesses` prop only refreshes when Settings is reopened,
  // so a save/clear updates this immediately instead of leaving the
  // Clear button (and the block's visibility once detection succeeds)
  // stuck showing pre-save state until the modal is closed and reopened.
  const [customPathActive, setCustomPathActive] = useState<boolean>(() => claudeHasCustomPath);
  const [customPathBusy, setCustomPathBusy] = useState(false);
  const [customPathError, setCustomPathError] = useState<string | null>(null);
```

Delete it entirely (all of it moved into `HarnessIntegrationSection.tsx` in Task 1, derived from props there instead of the `harnesses` array).

- [ ] **Step 4: Delete the Claude-specific effect and handlers**

Find and delete this whole block:

```typescript
  useEffect(() => {
    if (!hasClaudeCodeHarness) return;
    let cancelled = false;
    window.orkworks.getHarnessIntegrationStatus("claude-code").then((result) => {
      if (!cancelled) setClaudeIntegration(result);
    });
    return () => {
      cancelled = true;
    };
  }, [hasClaudeCodeHarness]);

  async function installClaudeIntegrationHandler() {
    setClaudeIntegrationBusy(true);
    try {
      setClaudeIntegration(await window.orkworks.installHarnessIntegration("claude-code"));
    } finally {
      setClaudeIntegrationBusy(false);
    }
  }

  async function uninstallClaudeIntegrationHandler() {
    setClaudeIntegrationBusy(true);
    try {
      setClaudeIntegration(await window.orkworks.uninstallHarnessIntegration("claude-code"));
    } finally {
      setClaudeIntegrationBusy(false);
    }
  }

  async function saveCustomPathHandler() {
    setCustomPathBusy(true);
    setCustomPathError(null);
    try {
      const result = await window.orkworks.setHarnessCommandOverride("claude-code", customPathDraft.trim());
      if (!result.ok) {
        setCustomPathError(result.error);
        return;
      }
      setCustomPathActive(true);
      setClaudeIntegration(await window.orkworks.getHarnessIntegrationStatus("claude-code"));
    } catch (error) {
      setCustomPathError(error instanceof Error ? error.message : "Couldn't set the custom path.");
    } finally {
      setCustomPathBusy(false);
    }
  }

  async function clearCustomPathHandler() {
    setCustomPathBusy(true);
    setCustomPathError(null);
    try {
      const result = await window.orkworks.clearHarnessCommandOverride("claude-code");
      if (!result.ok) {
        setCustomPathError(result.error);
        return;
      }
      setCustomPathActive(false);
      setCustomPathDraft("");
      setClaudeIntegration(await window.orkworks.getHarnessIntegrationStatus("claude-code"));
    } catch (error) {
      setCustomPathError(error instanceof Error ? error.message : "Couldn't clear the custom path.");
    } finally {
      setCustomPathBusy(false);
    }
  }
```

- [ ] **Step 5: Replace the Claude-only JSX block with the generic mount**

Find:

```tsx
                  {h.id === "claude-code" && activeDraft.includes(h.id) && (
                    <div className="settings-config-item-actions">
                      {claudeIntegration === null && (
                        <span className="settings-config-status">checking Claude Code integration…</span>
                      )}
                      {claudeIntegration && !claudeIntegration.ok && (
                        <span className="settings-config-status">{claudeIntegration.error}</span>
                      )}
                      {claudeIntegration?.ok && (
                        <span
                          className={
                            "settings-config-status" +
                            (claudeIntegration.status.toolDetected ? " settings-config-status--ok" : "")
                          }
                        >
                          {claudeIntegration.status.toolDetected ? "✓ Detected" : "Not detected"}
                        </span>
                      )}
                      {claudeIntegration?.ok && claudeIntegration.status.registration === "installed" && (
                        <>
                          <span className="settings-config-status settings-config-status--ok">✓ Notification hook installed</span>
                          <button type="button" onClick={uninstallClaudeIntegrationHandler} disabled={claudeIntegrationBusy}>
                            {claudeIntegrationBusy ? "Removing…" : "Uninstall"}
                          </button>
                        </>
                      )}
                      {claudeIntegration?.ok &&
                        (claudeIntegration.status.registration === "absent" ||
                          claudeIntegration.status.registration === "drifted") && (
                          <>
                            {claudeIntegration.status.confirmation && (
                              <p className="settings-section-copy">
                                Installing will add a Notification hook to{" "}
                                {claudeIntegration.status.confirmation.relativePaths.join(", ")} in this
                                workspace ({claudeIntegration.status.confirmation.coverageSummary}).
                                {claudeIntegration.status.confirmation.executableCodeWarning && (
                                  <> This hook runs an OrkWorks-installed script whenever Claude Code
                                  waits for input.</>
                                )}
                              </p>
                            )}
                            <button type="button" onClick={installClaudeIntegrationHandler} disabled={claudeIntegrationBusy}>
                              {claudeIntegrationBusy
                                ? "Installing…"
                                : claudeIntegration.status.registration === "drifted"
                                  ? "Reinstall"
                                  : "Install attention hook"}
                            </button>
                          </>
                        )}
                      {claudeIntegration?.ok && claudeIntegration.status.registration === "unsupported" && (
                        <span className="settings-config-status">
                          Attention hook isn't supported for this coding tool.
                        </span>
                      )}
                      {claudeIntegration?.ok && claudeIntegration.status.diagnostics.length > 0 && (
                        <span className="settings-config-status">
                          {claudeIntegration.status.diagnostics[0].message}
                        </span>
                      )}
                      {claudeIntegration?.ok &&
                        (claudeIntegration.status.diagnostics.some((d) => d.code === "tool_not_detected") ||
                          customPathActive) && (
                          <div className="settings-config-custom-path">
                            <label>
                              Custom path
                              <input
                                type="text"
                                value={customPathDraft}
                                onChange={(e) => setCustomPathDraft(e.target.value)}
                                placeholder="/opt/homebrew/bin/claude"
                                disabled={customPathBusy}
                              />
                            </label>
                            <p className="settings-section-copy">
                              This also becomes the command OrkWorks launches Claude Code sessions with —
                              make sure it points at the real binary.
                            </p>
                            <button
                              type="button"
                              onClick={saveCustomPathHandler}
                              disabled={customPathBusy || !looksAbsolute(customPathDraft.trim())}
                            >
                              {customPathBusy ? "Saving…" : "Save"}
                            </button>
                            {customPathActive && (
                              <button type="button" onClick={clearCustomPathHandler} disabled={customPathBusy}>
                                Clear
                              </button>
                            )}
                            {customPathError && (
                              <span className="settings-config-status">{customPathError}</span>
                            )}
                          </div>
                        )}
                    </div>
                  )}
```

Replace with:

```tsx
                  {INTEGRATION_HARNESS_IDS.includes(h.id) && activeDraft.includes(h.id) && (
                    <HarnessIntegrationSection harnessId={h.id} harnessName={h.name} harness={h} />
                  )}
```

- [ ] **Step 6: Type-check**

Run: `cd apps/desktop && npx tsc --noEmit`
Expected: no errors.

- [ ] **Step 7: Commit**

```bash
git add apps/desktop/src/components/SettingsModal.tsx
git commit -m "feat: mount HarnessIntegrationSection for Claude Code, Gemini, and Copilot"
```

---

### Task 3: Full verification pass

**Files:** none (verification only)

- [ ] **Step 1: Frontend type-check**

Run: `cd apps/desktop && npx tsc --noEmit`
Expected: no errors.

- [ ] **Step 2: Frontend test suite**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs`
Expected: all pass (no test in this suite exercises `SettingsModal`/`HarnessIntegrationSection` directly — see the Testing note at the top of this plan — but this confirms nothing else broke).

- [ ] **Step 3: Manual browser check**

Run: `cd apps/desktop && pnpm dev`

For each of Claude Code, Gemini CLI, and GitHub Copilot CLI:
1. Check its "Active" box in Settings if not already checked.
2. Confirm the Detected badge renders (✓ Detected or Not detected, matching whether that CLI is actually installed on `PATH`).
3. If the hook isn't installed, click "Install attention hook" and confirm it flips to "✓ Notification hook installed" with an "Uninstall" button.
4. Uncheck the harness's "Active" box, then recheck it — confirm the section remounts cleanly (refetches status, doesn't retain a stale busy/error state from before — this is the accepted mount-scoped behavior change from the design doc, confirm it doesn't visibly break anything).

For at least one non-Claude harness (Gemini or Copilot), also exercise the custom-path override end to end:
5. If not detected, enter a real absolute path to that tool's binary and click Save — confirm the Detected badge and hook-install state update after the refetch.
6. Reopen Settings — confirm the custom path is prefilled and a Clear button is visible.
7. Click Clear — confirm it reverts to the prior state on the next refetch.

- [ ] **Step 4: Doc and worktree checks**

Run: `bash .claude/hooks/doc-check.sh`
Run: `bash .claude/hooks/worktree-check.sh`
Address anything flagged.

- [ ] **Step 5: Open the PR**

Per `AGENTS.md`, this touches `apps/desktop/src/`, so it needs a branch + PR with a `/code-review` pass before merge (lightweight is sufficient — a pure extract-and-parameterize refactor plus mounting it three times, no new backend surface, no protocol/schema change).

```bash
git push -u origin settings-gemini-copilot-integration-ui
gh pr create --title "Add Gemini and Copilot integration UI to Settings" --body "$(cat <<'EOF'
## Summary
- Extracts SettingsModal.tsx's Claude-Code-only integration section (Detected badge, Notification-hook install/uninstall, custom-path override) into a reusable HarnessIntegrationSection component.
- Mounts it for Claude Code, Gemini CLI, and GitHub Copilot CLI (issue #217) — no backend changes needed, gemini.rs/copilot.rs already implement the same IntegrationHandler contract as claude.rs.

## Test plan
- [x] `tsc --noEmit`
- [x] `node --test` frontend suite
- [ ] Manual: Detected badge, hook install/uninstall, and custom-path override verified for all three harnesses

Closes #217.
EOF
)"
```

---

## Self-Review Notes

**Spec coverage:** the component extraction (Task 1), the allowlist-gated mounting for all three harnesses (Task 2), the accepted mount-scoped state behavior (documented in Task 1's `customPathActive` comment, matching the design doc's explicit note), and the manual three-harness verification (Task 3) all map directly to the design doc's Design and Testing sections. No backend task exists because none is needed — confirmed in the design doc and independently by this plan's file inventory (only two frontend files touched).

**Type consistency:** `HarnessIntegrationSectionProps` (`harnessId`/`harnessName`/`harness`), the renamed `integration`/`integrationBusy`/`installIntegrationHandler`/`uninstallIntegrationHandler` (from `claudeIntegration`/`claudeIntegrationBusy`/`install-`/`uninstallClaudeIntegrationHandler`), and the unchanged `customPathDraft`/`customPathActive`/`customPathBusy`/`customPathError`/`saveCustomPathHandler`/`clearCustomPathHandler` names are identical between Task 1 (where they're defined) and every place `SettingsModal.tsx` references the component in Task 2 (which only ever passes props — it never reaches into the component's internals, so no drift is possible there).

**No placeholders:** all code blocks are complete, byte-verified against the current file content before this plan was written.
