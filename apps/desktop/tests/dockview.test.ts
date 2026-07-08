import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import type { SessionInfo } from "../src/api.ts";
import {
  needsAttention,
  sessionAttentionStatus,
  sortSessions,
} from "../src/sessionSort.ts";

test("DockviewApp registers panels through onReady", () => {
  const source = readFileSync(new URL("../src/components/DockviewApp.tsx", import.meta.url), "utf8");

  assert.match(source, /onReady=\{\(event: DockviewReadyEvent\) =>/);
  assert.doesNotMatch(source, /defaultLayout=/);
  assert.match(source, /api\.(fromJSON|addPanel)/);
});

test("DockviewApp uses full-width single-tab mode so lone panels read like headers", () => {
  const source = readFileSync(new URL("../src/components/DockviewApp.tsx", import.meta.url), "utf8");

  assert.match(source, /singleTabMode="fullwidth"/);
});

test("DockviewApp uses a shared default tab component that hides close controls", () => {
  const source = readFileSync(new URL("../src/components/DockviewApp.tsx", import.meta.url), "utf8");

  assert.match(source, /DockviewDefaultTab/);
  assert.match(source, /defaultTabComponent=\{DockviewTab\}/);
  assert.match(source, /<DockviewDefaultTab\s+\{\.\.\.props\}\s+hideClose\s*\/>/);
});

test("App renders DockviewApp instead of the legacy three-panel layout", () => {
  const source = readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8");

  assert.match(source, /import DockviewApp from "\.\/components\/DockviewApp"/);
  assert.match(source, /<DockviewApp/);
  assert.doesNotMatch(source, /<TerminalTabs/);
  assert.doesNotMatch(source, /<LeftSidebar/);
  assert.doesNotMatch(source, /<RightSidebar/);
});

test("DockviewApp keeps all five panel ids registered (View menu hotkeys depend on it)", () => {
  const source = readFileSync(new URL("../src/components/DockviewApp.tsx", import.meta.url), "utf8");

  for (const id of ["sessions", "detail", "terminal", "capacity", "recommendations"]) {
    assert.match(source, new RegExp(`\\b${id}\\b.*:.*Panel`));
  }
});

test("DockviewApp default layout opens sessions/detail/terminal only (Capacity & Recommendations closed until they carry signal)", () => {
  const source = readFileSync(new URL("../src/components/DockviewApp.tsx", import.meta.url), "utf8");

  assert.match(source, /DEFAULT_LAYOUT_PANELS:\s*ReadonlyArray<string>\s*=\s*\["terminal",\s*"sessions",\s*"detail"\]/);
  assert.doesNotMatch(source, /DEFAULT_LAYOUT_PANELS[^=]*=[^;]*capacity/);
  assert.doesNotMatch(source, /DEFAULT_LAYOUT_PANELS[^=]*=[^;]*recommendations/);
});

test("DockviewApp migrates pre-redesign stored layouts that referenced removed panels", () => {
  const source = readFileSync(new URL("../src/components/DockviewApp.tsx", import.meta.url), "utf8");

  assert.match(source, /layoutNeedsMigration/);
  assert.match(source, /migrating stored layout/);
  assert.match(source, /!\("v" in parsed\)/);
  assert.match(source, /"capacity"/);
  assert.match(source, /"recommendations"/);
  // Post-redesign layouts are versioned, so they never match the migration
  // predicate after the user opens Capacity/Recommendations from the View menu.
  assert.match(source, /\{ v: 1, d: api\.toJSON\(\) \}/);
});

test("App and DockviewApp share one canonical default-layout builder", () => {
  const dockview = readFileSync(new URL("../src/components/DockviewApp.tsx", import.meta.url), "utf8");
  const app = readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8");

  assert.match(dockview, /export function buildDefaultLayout\(/);
  assert.match(app, /buildDefaultLayout\s*\}\s*from\s*"\.\/components\/DockviewApp"/);
  assert.match(app, /buildDefaultLayout\(api\)/);
});

test("DockviewApp exposes a right-side header action for the Sessions panel", () => {
  const source = readFileSync(
    new URL("../src/components/DockviewApp.tsx", import.meta.url),
    "utf8",
  );

  assert.match(source, /rightHeaderActionsComponent=\{SessionsHeaderActions\}/);
  assert.match(source, /activePanel\?\.id !== PANEL_DEFAULTS\.sessions\.component/);
  assert.match(source, /dockview-header-action/);
});

test("Sessions header action is gated on workspace presence and panel identity", () => {
  const source = readFileSync(
    new URL("../src/components/DockviewApp.tsx", import.meta.url),
    "utf8",
  );

  assert.match(
    source,
    /if \(!ctx\.workspace \|\| props\.activePanel\?\.id !== PANEL_DEFAULTS\.sessions\.component\) \{\s*return null;\s*\}/,
  );
});

test("App.css resolves dockview overrides through tokens, not raw hex literals", () => {
  const source = readFileSync(new URL("../src/App.css", import.meta.url), "utf8");

  assert.match(source, /\.dockview-header-action\b/);
  assert.match(source, /\.orkworks-dockview\s+\.dv-tabs-and-actions-container\b/);
  assert.match(source, /\.orkworks-dockview\s+\.dv-tab\s+\.dv-default-tab\s+\.dv-default-tab-content\b/);
  assert.match(
    source,
    /\.orkworks-dockview\s+\.dv-tabs-and-actions-container\.dv-single-tab\.dv-full-width-single-tab\s+\.dv-right-actions-container\b/,
  );
  assert.match(source, /--dv-background-color:\s*var\(--surface-1\)/);
  assert.match(source, /--dv-tabs-and-actions-container-background-color:\s*var\(--surface-2\)/);
  assert.match(source, /--dv-activegroup-visiblepanel-tab-background-color:\s*var\(--surface-2\)/);
  assert.match(source, /--dv-activegroup-hiddenpanel-tab-background-color:\s*var\(--surface-3\)/);
  assert.match(source, /\.orkworks-dockview\s+\.dv-groupview\b/);
  assert.match(source, /background:\s*var\(--surface-1\)/);
  assert.doesNotMatch(source, /#[0-9a-fA-F]{3,8}\b/);
});

test("tokens.css defines the substrate scale (color / space / state)", () => {
  const source = readFileSync(new URL("../src/styles/tokens.css", import.meta.url), "utf8");

  for (const tok of [
    "--surface-0", "--surface-1", "--surface-2",
    "--text-primary", "--text-muted", "--text-faint",
    "--state-ok", "--state-warn", "--state-error", "--state-info",
    "--attention-needs-you", "--attention-blocked", "--attention-done", "--attention-working", "--attention-failed", "--attention-idle",
    "--space-1", "--space-6",
    "--text-xs", "--text-xl",
    "--accent-focus",
  ]) {
    assert.match(source, new RegExp(`${tok}\\s*:`));
  }
});

test("global :focus-visible ring is defined and .session-list does not suppress outline", () => {
  const source = readFileSync(new URL("../src/App.css", import.meta.url), "utf8");

  assert.match(source, /:focus-visible\s*\{[^}]*outline:\s*2px\s+solid\s+var\(--accent-focus\)/);
  assert.doesNotMatch(source, /\.session-list[^}]*outline:\s*none/);
});

test("SessionDetailPanel groups content into situation/actions/facts/provenance zones", () => {
  const source = readFileSync(new URL("../src/components/SessionDetailPanel.tsx", import.meta.url), "utf8");

  assert.match(source, /detail-situation/);
  assert.match(source, /detail-actions/);
  assert.match(source, /detail-facts/);
  assert.match(source, /detail-provenance/);
  // Facts render through the DetailField primitive, so labels are props.
  for (const label of ["Directory", "Coding tool", "Model"]) {
    assert.match(source, new RegExp(`<DetailField[^>]*label="${label}"`));
  }
  assert.match(source, /<GitBranch\b/);
  assert.match(source, /Select an agent session to see details/);
  assert.match(source, /StatusIndicator/);
  assert.match(source, /attentionLabel/);
  assert.match(source, /memoryStateLabel/);
  assert.match(source, /sourceWithConfidence/);
});

test("SessionDetailPanel surfaces lifecycle, work phase, and frozen final attention metadata", () => {
  const source = readFileSync(new URL("../src/components/SessionDetailPanel.tsx", import.meta.url), "utf8");

  assert.match(
    source,
    /showDebugMetadata[\s\S]*label="Work phase"[\s\S]*label="Lifecycle"[\s\S]*label="OrkWorks session ID"/,
  );
  assert.match(source, /finalObservedStatus/);
});

test("SessionDetailPanel's action zone renders at most one move, via the shared detailActionZone derivation", () => {
  const source = readFileSync(new URL("../src/components/SessionDetailPanel.tsx", import.meta.url), "utf8");

  assert.match(source, /import\s*\{[^}]*detailActionZone[^}]*\}\s*from\s*"\.\.\/labels"/);
  assert.match(source, /actionZone\.kind === "cue"/);
  assert.match(source, /actionZone\.kind === "buttons"/);
  assert.match(source, /actionZone\.kind === "resume"/);
  assert.match(source, /<ResumeChooser\b/);
  // "Nothing at all" for a live session with no pending question — no disabled resume button left behind.
  assert.doesNotMatch(source, /session-resume-button/);
});

test("session list sorts by attention priority with lifecycle fallback", () => {
  const sessions: SessionInfo[] = [
    { id: "1", label: "s1", status: "running", cwd: "/tmp", created_at: "now", memoryState: "live", resumeStrategy: "none" },
    { id: "2", label: "s2", status: "running", observedStatus: "waiting_for_input", cwd: "/tmp", created_at: "now", memoryState: "live", resumeStrategy: "none" },
    { id: "3", label: "s3", status: "ended", cwd: "/tmp", created_at: "now", memoryState: "remembered", resumeStrategy: "none" },
    { id: "4", label: "s4", status: "running", observedStatus: "failed", cwd: "/tmp", created_at: "now", memoryState: "live", resumeStrategy: "none" },
    { id: "5", label: "s5", status: "running", observedStatus: "blocked", cwd: "/tmp", created_at: "now", memoryState: "live", resumeStrategy: "none" },
    { id: "6", label: "s6", status: "running", observedStatus: "done", cwd: "/tmp", created_at: "now", memoryState: "live", resumeStrategy: "none" },
  ];
  const sorted = sortSessions(sessions);
  assert.equal(sorted[0].id, "2"); // waiting_for_input
  assert.equal(sorted[1].id, "5"); // blocked
  assert.equal(sorted[2].id, "4"); // failed
  assert.equal(sorted[3].id, "6"); // done
  assert.equal(sorted[4].id, "1"); // running
  assert.equal(sorted[5].id, "3"); // ended
});

test("needsAttention lifecycle statuses do not trigger from raw lifecycle", () => {
  assert.equal(needsAttention("running"), false);
  assert.equal(needsAttention("ended"), false);
  assert.equal(needsAttention("creating"), false);
});

test("sessionAttentionStatus falls back to lifecycle status when no observed", () => {
  const session: SessionInfo = {
    id: "1", label: "test", status: "running", cwd: "/tmp", created_at: "now",
    memoryState: "live", resumeStrategy: "none",
  };
  assert.equal(sessionAttentionStatus(session), "running");
});

test("sessionAttentionStatus ignores observed status outside active lifecycle", () => {
  const session: SessionInfo = {
    id: "1",
    label: "ending",
    status: "running",
    lifecyclePhase: "ending",
    observedStatus: "waiting_for_input",
    cwd: "/tmp",
    created_at: "now",
    memoryState: "live",
    resumeStrategy: "none",
  };

  assert.equal(sessionAttentionStatus(session), "running");
});

test("session detail exposes resumable session action", () => {
  const panelSource = readFileSync(
    new URL("../src/components/SessionDetailPanel.tsx", import.meta.url),
    "utf8",
  );
  const labelsSource = readFileSync(new URL("../src/labels.ts", import.meta.url), "utf8");

  assert.match(panelSource, /onResumeSession/);
  assert.match(panelSource, /ResumeChooser/);
  // resumeStrategy handling now lives in the shared resumeChoices() derivation, not the component.
  assert.match(labelsSource, /resumeStrategy/);
});

test("CenterPanel keeps inactive terminals alive while switching sessions", () => {
  const source = readFileSync(
    new URL("../src/components/CenterPanel.tsx", import.meta.url),
    "utf8",
  );

  assert.doesNotMatch(source, /previousId !== sessionId[\s\S]*disposeTerminal\(previousId\)/);
});

test("session list marks remembered sessions separately from live sessions", () => {
  const source = readFileSync(
    new URL("../src/components/SessionListPanel.tsx", import.meta.url),
    "utf8",
  );

  // Remembered is the negation of "live" — inverting this condition flips the class onto the wrong sessions.
  assert.match(source, /const\s+remembered\s*=\s*s\.memoryState\s*!==\s*"live"/);
  // The remembered variable (not its inverse) drives the class — inverting the ternary would break this.
  assert.match(source, /remembered\s*\?\s*"session-row--remembered"/);
});

test("session list only offers kill for live sessions", () => {
  const source = readFileSync(
    new URL("../src/components/SessionListPanel.tsx", import.meta.url),
    "utf8",
  );

  // Kill button is gated to the live branch.
  assert.match(source, /s\.memoryState === "live" && \(\s*<button[\s\S]*session-row-kill/);
  // Kill class must be absent from the non-live (remembered) branch.
  const nonLiveBlock =
    source.match(/\{s\.memoryState !== "live" && \([\s\S]*?\)\s*\}/)?.[0] ?? "";
  assert.doesNotMatch(nonLiveBlock, /session-row-kill/);
});

test("session list routes attention through the labels module instead of raw enums", () => {
  const source = readFileSync(
    new URL("../src/components/SessionListPanel.tsx", import.meta.url),
    "utf8",
  );

  assert.match(source, /attentionLabel/);
  assert.match(source, /attentionTone/);
  assert.match(source, /import StatusIndicator from "\.\/StatusIndicator"/);
  // The row uses data-attention to drive border/icon color, never inline hex.
  assert.match(source, /data-attention=\{tone\}/);
  assert.doesNotMatch(source, /style=\{\{[^}]*#[0-9a-fA-F]{3,8}/);
});

test("EmptyState is the single empty-state primitive across the app", () => {
  const list = readFileSync(new URL("../src/components/SessionListPanel.tsx", import.meta.url), "utf8");
  const detail = readFileSync(new URL("../src/components/SessionDetailPanel.tsx", import.meta.url), "utf8");
  const terminal = readFileSync(new URL("../src/components/TerminalPanel.tsx", import.meta.url), "utf8");
  const center = readFileSync(new URL("../src/components/CenterPanel.tsx", import.meta.url), "utf8");

  for (const source of [list, detail, terminal, center]) {
    assert.match(source, /import EmptyState from "\.\/EmptyState"/);
    assert.match(source, /<EmptyState\s+message=/);
  }
});

test("App restores the last active session from the initial workspace", () => {
  const source = readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8");

  assert.match(source, /info\.lastActiveSessionId/);
  assert.match(source, /setActiveSessionId\(info\.lastActiveSessionId\)/);
});

test("preload exposes settings and hotkey capture APIs", () => {
  const source = readFileSync(new URL("../electron/preload.ts", import.meta.url), "utf8");

  assert.match(source, /getSettings:\s*\(\)/);
  assert.match(source, /ipcRenderer\.invoke\("get-settings"\)/);
  assert.match(source, /saveHotkeys:\s*\(hotkeys:/);
  assert.match(source, /ipcRenderer\.invoke\("save-hotkeys", hotkeys\)/);
  assert.match(source, /saveDebugSettings:\s*\(debug:/);
  assert.match(source, /ipcRenderer\.invoke\("save-debug-settings", debug\)/);
  assert.match(source, /setHotkeyCaptureActive:\s*\(active:/);
  assert.match(source, /ipcRenderer\.send\("orkworks:hotkey-capture-active", active\)/);
});

test("App exposes a settings titlebar entry and renders SettingsModal", () => {
  const source = readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8");

  assert.match(source, /import SettingsModal from "\.\/components\/SettingsModal"/);
  assert.match(source, /setSettingsOpen\(true\)/);
  assert.match(source, /<SettingsModal/);
});

test("SettingsModal contains hotkey edit reset default cancel and save flows", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");

  for (const text of ["Hotkeys", "Edit", "Reset", "Restore defaults", "Cancel", "Save"]) {
    assert.match(source, new RegExp(text));
  }
  assert.match(source, /acceleratorFromKeyboardEvent/);
  assert.match(source, /setHotkeyCaptureActive\(true\)/);
  assert.match(source, /setHotkeyCaptureActive\(false\)/);
});

test("SettingsModal exposes a debug metadata toggle", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");

  assert.match(source, /Show debug metadata/);
  assert.match(source, /saveDebugSettings/);
});

test("TerminalPanel no longer renders internal session tabs or duplicate kill controls", () => {
  const source = readFileSync(new URL("../src/components/TerminalPanel.tsx", import.meta.url), "utf8");

  assert.doesNotMatch(source, /liveSessions\.map/);
  assert.doesNotMatch(source, /onKillSession/);
  assert.match(source, /<CenterPanel/);
});

test("App activates shared terminal panel on session create", () => {
  const source = readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8");

  assert.match(source, /api\.getPanel\("terminal"\)/);
  assert.match(source, /panel\.api\.setActive\(\)/);
});

test("TermPanel in DockviewApp passes a single session to TerminalPanel", () => {
  const source = readFileSync(new URL("../src/components/DockviewApp.tsx", import.meta.url), "utf8");

  assert.match(source, /session=\{session\}/);
  assert.match(source, /TermPanel/);
});

test("App routes user-facing error catches through the toast feedback primitive", () => {
  const source = readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8");

  assert.match(source, /import \{ pushToast \} from "\.\/feedback"/);
  assert.match(source, /pushToast\("error", "Couldn't open workspace\."\)/);
  assert.match(source, /load app settings/);
  assert.match(source, /pushToast\("error", "Couldn't start a new session\."\)/);
  assert.match(source, /pushToast\("error", "Couldn't end session\."\)/);
  assert.doesNotMatch(source, /\/\* ignore \*\//);
});

test("SettingsModal uses default hotkeys from the main-process settings response", () => {
  const modal = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  const types = readFileSync(new URL("../src/appSettingsTypes.ts", import.meta.url), "utf8");
  const main = readFileSync(new URL("../electron/main.ts", import.meta.url), "utf8");

  assert.match(types, /defaultHotkeys:\s*HotkeySettings/);
  assert.match(main, /DEFAULT_HOTKEYS/);
  assert.match(main, /defaultHotkeys:\s*\{\s*\.\.\.DEFAULT_HOTKEYS\s*\}/);
  assert.match(modal, /const defaultHotkeys = initialSettings\.defaultHotkeys/);
  assert.doesNotMatch(modal, /const defaultHotkeys:\s*HotkeySettings\s*=\s*\{/);
});

test("App titlebar uses the canonical workspace vocabulary (no 'Folder' drift)", () => {
  const source = readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8");

  assert.match(source, /import \{ VOCAB \} from "\.\/labels"/);
  assert.match(source, /\{VOCAB\.openWorkspace\}/);
  assert.doesNotMatch(source, /Open Folder/);
});

test("Dockview keeps capacity as a non-provider surface", () => {
  const source = readFileSync(new URL("../src/components/DockviewApp.tsx", import.meta.url), "utf8");
  assert.match(source, /capacity.*Capacity/);
  assert.doesNotMatch(source, /capacity.*Providers/);
});

test("SettingsModal includes a Model providers section above Hotkeys", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(source, /Model providers/);
  assert.match(source, /providerDraft/);
  assert.match(source, /provider-model-select/);
  assert.match(source, /getProviderModels/);
});
