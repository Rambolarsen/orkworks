# Session Details Debug IDs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a persisted Settings toggle that reveals OrkWorks and harness session IDs in the Details panel for debugging.

**Architecture:** Extend desktop app settings with a `debug.showSessionIds` flag, save it through Electron IPC, and conditionally render two read-only fields in the Details panel using data the renderer already has.

**Tech Stack:** React, TypeScript, Electron IPC, Node test runner

---

### Task 1: Add debug settings persistence

**Files:**
- Modify: `apps/desktop/electron/settingsMemory.ts`
- Modify: `apps/desktop/electron/main.ts`
- Modify: `apps/desktop/electron/preload.ts`
- Modify: `apps/desktop/src/appSettingsTypes.ts`
- Modify: `apps/desktop/src/orkworksWindow.d.ts`
- Test: `apps/desktop/tests/electronSettingsMemory.test.ts`

- [ ] Add failing tests for default and persisted `debug.showSessionIds`
- [ ] Add settings normalization and default values
- [ ] Add a dedicated `save-debug-settings` IPC path
- [ ] Re-run the focused settings tests

### Task 2: Wire the Settings UI

**Files:**
- Modify: `apps/desktop/src/components/SettingsModal.tsx`
- Test: `apps/desktop/tests/dockview.test.ts`

- [ ] Add a failing UI/source test for the new checkbox and save call
- [ ] Add a Settings section with a persisted `Show debug metadata` checkbox
- [ ] Re-run the focused UI/source tests

### Task 3: Gate debug IDs in Details

**Files:**
- Modify: `apps/desktop/src/App.tsx`
- Modify: `apps/desktop/src/components/DockviewApp.tsx`
- Modify: `apps/desktop/src/components/SessionDetailPanel.tsx`
- Test: `apps/desktop/tests/terminology.test.ts`

- [ ] Add a failing test asserting the Details panel includes gated ID labels and `Not captured`
- [ ] Thread the debug flag through the existing props
- [ ] Render the two ID fields only when enabled
- [ ] Re-run the focused Details tests
