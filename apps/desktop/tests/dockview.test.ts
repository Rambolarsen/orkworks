import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import type { SessionInfo } from "../src/api.ts";
import {
  needsAttention,
  sessionAttentionStatus,
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

test("TerminalPanel uses read-only replay only for dead sessions", () => {
  const source = readFileSync(new URL("../src/components/TerminalPanel.tsx", import.meta.url), "utf8");

  assert.match(source, /renderTerminalPresentation/);
  assert.match(source, /<HistoricalTerminal sessionId=\{session\.id\} \/>/);
  assert.match(source, /<CenterPanel[\s\S]*?backendStatus=\{backendStatus\}[\s\S]*?sessionId=\{session\.id\}[\s\S]*?\/>/);
});

test("HistoricalTerminal loads output without opening an interactive terminal transport", () => {
  const source = readFileSync(new URL("../src/components/HistoricalTerminal.tsx", import.meta.url), "utf8");

  assert.match(source, /getTerminalOutput\(baseUrl, sessionId\)/);
  assert.match(source, /loadTerminalReplay/);
  assert.doesNotMatch(source, /WebSocket/);
  assert.doesNotMatch(source, /ensureTerminal/);
});

test("HistoricalTerminal labels only the current successful fixed-grid replay", () => {
  const source = readFileSync(new URL("../src/components/HistoricalTerminal.tsx", import.meta.url), "utf8");

  assert.match(source, /Recorded at \{replay\.size\.cols\} × \{replay\.size\.rows\}/);
  assert.match(source, /replay\.sessionId === sessionId && replay\.state === "loaded" && replay\.size/);
  assert.match(source, /setReplay\(\{ sessionId, state: result, size: result === "loaded" \? loadedSize : null \}\)/);
  assert.match(source, /<div className="terminal-shell">\s*\{replay\.sessionId/);
  // The cue inserts on the loading→loaded transition. Without `key="terminal"`, React
  // reconciles positionally and repurposes the terminal-container DOM node as the cue,
  // dropping the imperatively-mounted xterm output. Pin this regression.
  assert.match(source, /<div key="terminal" ref=\{containerRef\}/);
  assert.match(source, /<div key="cue" className="historical-terminal-size"/);
});

test("DockviewApp keeps all five panel ids registered (View menu hotkeys depend on it)", () => {
  const source = readFileSync(new URL("../src/components/DockviewApp.tsx", import.meta.url), "utf8");

  for (const id of ["sessions", "detail", "terminal", "capacity", "recommendations"]) {
    assert.match(source, new RegExp(`\\b${id}\\b.*:.*Panel`));
  }
});

test("Review is a reusable Terminal-group tab, including after a restored layout omitted it", () => {
  const dockview = readFileSync(new URL("../src/components/DockviewApp.tsx", import.meta.url), "utf8");
  const app = readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8");

  assert.match(dockview, /review:\s*ReviewTab/);
  assert.match(dockview, /review:\s*\{ component: "review", title: "Review", position: \{ referencePanel: "terminal" \} \}/);
  assert.match(app, /api\.getPanel\("review"\) \?\? api\.addPanel\(/);
  assert.match(app, /position: \{ referencePanel: "terminal" \}/);
});

test("ReviewTab only fetches plan content when the active session has an openable plan", () => {
  // Regression: ReviewTab used to pass ctx.activeSessionId straight through
  // regardless of hasOpenablePlan, so switching to any plan-less session
  // while the Review tab stayed open re-fired getPlanContent, which the
  // sidecar correctly rejects (no plan_path) but Electron logs as an
  // "Error occurred in handler for 'get-plan-content'" every single time.
  const source = readFileSync(new URL("../src/components/DockviewApp.tsx", import.meta.url), "utf8");

  assert.match(source, /session\?\.hasOpenablePlan \? ctx\.activeSessionId : null/);
});

test("A readable plan keeps the Details review card visible even without another action", () => {
  const source = readFileSync(new URL("../src/components/SessionDetailPanel.tsx", import.meta.url), "utf8");

  assert.match(source, /active\.recommendation \|\| actionZone\.kind !== "none" \|\| active\.hasOpenablePlan/);
});

test("ReviewPanel exposes retry after a content request fails", () => {
  const source = readFileSync(new URL("../src/components/ReviewPanel.tsx", import.meta.url), "utf8");

  assert.match(source, /if \(error\) return <EmptyState message="This plan is no longer available\."/);
  assert.match(source, /action=\{\{ label: "Retry", onClick: load \}\}/);
});

test("ReviewPanel renders plan content as Markdown rather than raw preformatted text", () => {
  const source = readFileSync(new URL("../src/components/ReviewPanel.tsx", import.meta.url), "utf8");

  assert.match(source, /import ReactMarkdown, \{ type Components \} from "react-markdown"/);
  assert.match(source, /<ReactMarkdown remarkPlugins=\{\[remarkGfm\]\} components=\{markdownComponents\}>\{content\}<\/ReactMarkdown>/);
  assert.doesNotMatch(source, /<pre className="review-plan-content">\{content\}<\/pre>/);
});

test("ReviewPanel routes Markdown links through the safe external-link bridge instead of letting them navigate the renderer", () => {
  // Regression: a plan/spec doc's relative link (e.g. to another spec file)
  // rendered as an ordinary <a href> would otherwise be same-origin in dev
  // and pass electron/externalLinks.ts's will-navigate allow-check,
  // replacing the whole app window instead of doing nothing.
  const source = readFileSync(new URL("../src/components/ReviewPanel.tsx", import.meta.url), "utf8");

  assert.match(source, /if \(href\?\.startsWith\("#"\)\) return <a href=\{href\}>\{children\}<\/a>;/);
  assert.match(source, /event\.preventDefault\(\);/);
  assert.match(source, /window\.orkworks\.openExternalLink\(href\)/);
  assert.match(source, /const markdownComponents: Components = \{ a: ReviewLink \};/);
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

test("settings modal layout keeps a stable top anchor and bounded scroll region", () => {
  const source = readFileSync(new URL("../src/App.css", import.meta.url), "utf8");
  const backdrop = source.match(/\.settings-backdrop\s*\{([^}]*)\}/)?.[1] ?? "";
  const modal = source.match(/\.settings-modal\s*\{([^}]*)\}/)?.[1] ?? "";
  const content = source.match(/\.settings-content\s*\{([^}]*)\}/)?.[1] ?? "";

  assert.match(backdrop, /display:\s*flex/);
  assert.match(backdrop, /align-items:\s*flex-start/);
  assert.match(backdrop, /justify-content:\s*center/);
  assert.match(backdrop, /padding-top:\s*48px/);
  assert.match(modal, /max-height:\s*min\(calc\(100vh - 48px\),\s*82vh\)/);
  assert.match(modal, /overflow:\s*hidden/);
  assert.match(content, /overflow-y:\s*auto/);
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

test("SessionDetailPanel fetches and renders the summary-log checkpoint history", () => {
  const source = readFileSync(new URL("../src/components/SessionDetailPanel.tsx", import.meta.url), "utf8");

  assert.match(source, /import\s*\{\s*getSummaryLog\s*\}\s*from\s*"\.\.\/api"/);
  assert.match(source, /getSummaryLog\(baseUrl, active\.id\)/);
  assert.match(source, /detail-task-history/);
  assert.match(source, /summaryLog\.map/);
});

test("SessionDetailPanel resets task history synchronously on session switch, not just via effect", () => {
  const source = readFileSync(new URL("../src/components/SessionDetailPanel.tsx", import.meta.url), "utf8");

  // Regression: switching sessions must not paint the previous session's
  // task history under the new one's header while the fetch is in flight.
  // The reset has to run during render (not inside a useEffect, which only
  // fires after the stale content has already painted once).
  assert.match(source, /summaryLogSessionId/);
  const resetIndex = source.indexOf("active.id !== summaryLogSessionId");
  const firstEffectIndex = source.indexOf("useEffect(");
  assert.notEqual(resetIndex, -1, "expected a render-time session-id comparison");
  assert.ok(
    resetIndex < firstEffectIndex,
    "the session-id reset must run before any effect, i.e. during render",
  );
});

test("SessionDetailPanel refetches task history on lastActivityAt, not just peonLastInference", () => {
  const source = readFileSync(new URL("../src/components/SessionDetailPanel.tsx", import.meta.url), "utf8");

  // Regression: agent-hook-driven summary checkpoints advance lastActivityAt
  // but not peonLastInference, so the fetch effect must depend on the
  // former (which advances for every checkpoint source) to pick those up.
  const fetchEffectStart = source.indexOf("getSummaryLog(baseUrl, active.id)");
  const depsSlice = source.slice(fetchEffectStart, fetchEffectStart + 700);
  assert.match(depsSlice, /\[active\?\.id, active\?\.lastActivityAt\]/);
});

test("SessionDetailPanel surfaces lifecycle, work phase, and frozen final attention metadata", () => {
  const source = readFileSync(new URL("../src/components/SessionDetailPanel.tsx", import.meta.url), "utf8");

  assert.match(
    source,
    /showDebugMetadata[\s\S]*label="Work phase"[\s\S]*label="Lifecycle"[\s\S]*label="OrkWorks session ID"/,
  );
  assert.match(source, /finalObservedStatus/);
});

test("SessionDetailPanel keeps its existing action zone and adds plan review for every readable plan", () => {
  const source = readFileSync(new URL("../src/components/SessionDetailPanel.tsx", import.meta.url), "utf8");

  assert.match(source, /import\s*\{[^}]*detailActionZone[^}]*\}\s*from\s*"\.\.\/labels"/);
  assert.match(source, /actionZone\.kind === "cue"/);
  assert.match(source, /actionZone\.kind === "buttons"/);
  assert.match(source, /actionZone\.kind === "resume"/);
  assert.match(source, /actionZone\.kind === "plan"/);
  assert.match(source, /active\.hasOpenablePlan/);
  assert.match(source, /Plan ready for review/);
  assert.match(source, /Plan available/);
  assert.match(source, /Review plan/);
  assert.match(source, /Request independent review/);
  assert.match(source, /window\.orkworks\.requestPlanReview\(active\.id\)/);
  assert.doesNotMatch(source, /window\.orkworks\.openPlan/);
  assert.match(source, /<ResumeChooser\b/);
  // "Nothing at all" for a live session with no pending question — no disabled resume button left behind.
  assert.doesNotMatch(source, /session-resume-button/);
});



test("needsAttention lifecycle statuses do not trigger from raw lifecycle", () => {
  assert.equal(needsAttention("running"), false);
  assert.equal(needsAttention("ended"), false);
  assert.equal(needsAttention("creating"), false);
});

test("sessionAttentionStatus defaults an alive session to idle", () => {
  const session: SessionInfo = {
    id: "1", label: "test", status: "running", lifecycle: "alive", cwd: "/tmp", created_at: "now",
    memoryState: "live", resumeStrategy: "none",
  };
  assert.equal(sessionAttentionStatus(session), "idle");
});

test("sessionAttentionStatus is neutral for a dead session, ignoring stale attention", () => {
  const session: SessionInfo = {
    id: "1",
    label: "ended",
    status: "ended",
    lifecycle: "dead",
    attention: "needs_you",
    cwd: "/tmp",
    created_at: "now",
    memoryState: "remembered",
    resumeStrategy: "none",
  };

  assert.equal(sessionAttentionStatus(session), "neutral");
});

test("session rows derive tone from sessionAttentionStatus, no component-local lifecycle override", () => {
  const list = readFileSync(new URL("../src/components/SessionListPanel.tsx", import.meta.url), "utf8");
  const detail = readFileSync(new URL("../src/components/SessionDetailPanel.tsx", import.meta.url), "utf8");

  // Whether creating/stopping reads as working is sessionAttentionStatus's
  // call, so it stays correct the instant a session goes alive (idle,
  // unless the harness has actually reported otherwise). A component-local
  // override here would risk drifting out of sync with it.
  for (const source of [list, detail]) {
    assert.doesNotMatch(source, /transitional/);
  }
});

test("SessionListPanel displays canonical session activity when present", () => {
  // lastActivity/lastActivityTimestamp moved to labels.ts (shared with
  // SessionDetailPanel) so they're unit-testable without a JSX-parsing
  // runtime; see labels.test.ts for behavioral coverage.
  const source = readFileSync(
    new URL("../src/labels.ts", import.meta.url),
    "utf8",
  );

  assert.match(source, /return relativeTime\(lastActivityTimestamp\(s\), now\)/);
  assert.match(source, /const candidates = \[s\.lastOutputAt, s\.lastActivityAt\]/);
});

test("StatusIndicator renders completed unread results as accessible dots", () => {
  const source = readFileSync(
    new URL("../src/components/StatusIndicator.tsx", import.meta.url),
    "utf8",
  );
  const css = readFileSync(new URL("../src/App.css", import.meta.url), "utf8");

  assert.match(source, /variant\?:\s*"status"\s*\|\s*"unread"/);
  assert.match(source, /variant\s*=\s*"status"/);
  assert.match(source, /variant\s*===\s*"unread"\s*&&\s*tone\s*!==\s*"working"/);
  assert.match(source, /className="status-indicator status-indicator-unread"/);
  assert.match(source, /aria-label=\{`Unread:\s*\$\{label\}`\}/);
  assert.match(css, /\.status-indicator\s*\{[\s\S]*width:\s*14px;[\s\S]*height:\s*14px;/);
  assert.match(css, /\.status-indicator-dot::before\s*\{[\s\S]*display:\s*block;[\s\S]*width:\s*8px;[\s\S]*height:\s*8px;/);
  assert.match(css, /\.status-indicator-unread::before\s*\{[\s\S]*display:\s*block;[\s\S]*width:\s*7px;[\s\S]*height:\s*7px;/);
  for (const tone of ["needs-you", "blocked", "failed", "idle"]) {
    assert.match(css, new RegExp(`\\.status-indicator\\[data-attention="${tone}"\\]`));
  }
  assert.match(
    css,
    /\.status-indicator-unread\[data-attention="idle"\][\s\S]*color:\s*var\(--attention-needs-you\)/,
  );
});

test("SessionDetailPanel keeps the normal status variant", () => {
  const source = readFileSync(
    new URL("../src/components/SessionDetailPanel.tsx", import.meta.url),
    "utf8",
  );

  assert.match(source, /<StatusIndicator tone=\{tone\} label=\{attentionLabel\(attn\)\} \/>/);
  assert.doesNotMatch(source, /<StatusIndicator[^>]*variant=/);
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

test("session list marks dead sessions separately from alive sessions", () => {
  const source = readFileSync(
    new URL("../src/components/SessionListPanel.tsx", import.meta.url),
    "utf8",
  );

  assert.match(source, /const\s+remembered\s*=\s*s\.lifecycle\s*===\s*"dead"/);
  assert.match(source, /remembered\s*\?\s*"session-row--remembered"/);
});

test("session list only offers kill for alive sessions", () => {
  const source = readFileSync(
    new URL("../src/components/SessionListPanel.tsx", import.meta.url),
    "utf8",
  );

  assert.match(source, /const\s+canKill\s*=\s*s\.lifecycle\s*===\s*"alive"/);
  assert.match(source, /\{canKill && \(\s*<button[\s\S]*session-row-kill/);
  const deadBlock = source.match(/\{remembered && \([\s\S]*?\)\s*\}/)?.[0] ?? "";
  assert.doesNotMatch(deadBlock, /session-row-kill/);
});

test("session list uses one status slot and keeps tool/time metadata separate from destructive controls", () => {
  const panel = readFileSync(
    new URL("../src/components/SessionListPanel.tsx", import.meta.url),
    "utf8",
  );
  const css = readFileSync(new URL("../src/App.css", import.meta.url), "utf8");

  assert.match(panel, /className="session-row-meta"[\s\S]*HarnessIcon[\s\S]*session-row-time/);
  assert.match(panel, /className="session-row-primary"[\s\S]*<StatusIndicator[^>]*variant=\{unread \? "unread" : "status"\}/);
  assert.doesNotMatch(panel, /session-row-unread-slot/);
  assert.doesNotMatch(panel, /session-row-unread-dot/);
  assert.match(css, /\.session-row-meta\s*\{/);
  assert.match(css, /\.session-row-actions\s*\{/);
});

test("session list keeps the right-side metadata footprint compact", () => {
  const css = readFileSync(new URL("../src/App.css", import.meta.url), "utf8");

  assert.match(css, /\.session-row\s*\{[\s\S]*padding:\s*var\(--space-3\) var\(--space-5\) var\(--space-3\) var\(--space-2\);/);
  assert.match(css, /\.session-row-secondary\s*\{[\s\S]*gap:\s*var\(--space-1\);/);
  assert.match(css, /\.session-row-meta\s*\{[\s\S]*grid-template-columns:\s*12px 6ch;/);
  assert.match(css, /\.session-row-meta\s*\{[\s\S]*column-gap:\s*var\(--space-2\);/);
  assert.match(css, /\.session-row-actions\s*\{[\s\S]*gap:\s*0;/);
  assert.match(css, /\.session-row-kill\s*\{[\s\S]*padding:\s*0 2px;/);
  assert.match(css, /\.session-row-forget\s*\{[\s\S]*padding:\s*0 2px;/);
});

test("unread rows keep their tint without making the label bold", () => {
  const css = readFileSync(new URL("../src/App.css", import.meta.url), "utf8");

  assert.match(css, /\.session-row--unread,\s*\n\.session-row--unread:hover\s*\{\s*background:\s*var\(--surface-2\);\s*\}/);
  assert.doesNotMatch(css, /\.session-row--unread\s+\.session-row-label/);
  assert.match(css, /\.session-row--loud\s+\.session-row-label\s*\{\s*font-weight:\s*var\(--weight-semibold\);\s*\}/);
});

test("session list destructive controls still stop row selection propagation", () => {
  const panel = readFileSync(
    new URL("../src/components/SessionListPanel.tsx", import.meta.url),
    "utf8",
  );

  assert.match(panel, /className="session-row-kill"[\s\S]*e\.stopPropagation\(\)/);
  assert.match(panel, /className="session-row-forget"[\s\S]*e\.stopPropagation\(\)/);
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
  const source = readFileSync(new URL("../src/workspaceSessionController.ts", import.meta.url), "utf8");

  assert.match(source, /info\.lastActiveSessionId/);
  assert.match(source, /options\.onActiveSession\?\.\(match\.id\)/);
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

test("SettingsModal keeps detection status in every Coding tools row", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(source, /HarnessDetectionStatus harnessId=\{h\.id\}/);
  assert.match(source, /refreshGeneration=/);
  assert.match(source, /onDetectionChanged=/);
  assert.doesNotMatch(source, /activeDraft\.includes\(h\.id\).*HarnessDetectionStatus/);
});

test("TerminalPanel no longer renders internal session tabs or duplicate kill controls", () => {
  const source = readFileSync(new URL("../src/components/TerminalPanel.tsx", import.meta.url), "utf8");

  assert.doesNotMatch(source, /liveSessions\.map/);
  assert.doesNotMatch(source, /onKillSession/);
  assert.match(source, /<CenterPanel/);
});

test("TerminalPanel replays dead sessions without retaining their interactive handles", () => {
  const terminalPanel = readFileSync(
    new URL("../src/components/TerminalPanel.tsx", import.meta.url),
    "utf8",
  );
  const controller = readFileSync(new URL("../src/workspaceSessionController.ts", import.meta.url), "utf8");
  assert.match(terminalPanel, /renderTerminalPresentation/);
  assert.match(terminalPanel, /HistoricalTerminal/);
  assert.match(controller, /pruneTerminals\(/);
  assert.match(controller, /session\.lifecycle !== "dead"/);
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

test("TerminalPanel marks CenterPanel as starting while the session is still being created", () => {
  const source = readFileSync(new URL("../src/components/TerminalPanel.tsx", import.meta.url), "utf8");
  assert.match(source, /starting=\{isSessionStarting\(session\)\}/);
  assert.match(source, /import \{ isSessionStarting, renderTerminalPresentation \} from "\.\.\/terminalPresentation"/);
});

test("CenterPanel routes backend attach failures to the shared recovery callback", () => {
  const center = readFileSync(new URL("../src/components/CenterPanel.tsx", import.meta.url), "utf8");
  const terminal = readFileSync(new URL("../src/components/TerminalPanel.tsx", import.meta.url), "utf8");
  const dockview = readFileSync(new URL("../src/components/DockviewApp.tsx", import.meta.url), "utf8");

  assert.match(center, /attachTerminalAfterBackendReady/);
  assert.match(center, /onBackendUnavailable/);
  assert.match(terminal, /onBackendUnavailable=\{onBackendUnavailable\}/);
  assert.match(dockview, /onBackendUnavailable=\{ctx\.onBackendUnavailable\}/);
});

test("CenterPanel disables stdin and shows a loading overlay while starting, instead of an interactable blank terminal", () => {
  const source = readFileSync(new URL("../src/components/CenterPanel.tsx", import.meta.url), "utf8");
  assert.match(source, /computeTerminalInteractivity/);
  assert.match(source, /terminal-starting-overlay/);
  assert.match(source, /starting-dots/);
  assert.match(source, /aria-live="polite"/);
});

test("terminal starting overlay exposes role=status so assistive tech announces it like other live regions", () => {
  const source = readFileSync(new URL("../src/components/CenterPanel.tsx", import.meta.url), "utf8");
  const overlay = source.match(/<div className="terminal-starting-overlay"[^>]*>/)?.[0] ?? "";
  assert.match(overlay, /role="status"/);
});

test("the starting overlay's render-time ended check also treats an unavailable terminal as ended, so a rejected attach on a still-creating session doesn't strand the overlay on screen forever", () => {
  const source = readFileSync(new URL("../src/components/CenterPanel.tsx", import.meta.url), "utf8");
  assert.match(source, /terminalHandle\.ended \|\| terminalHandle\.unavailable/);
});

test("attachTerminal keeps a stable identity across starting transitions, so a session finishing setup in the background does not steal focus from the user", () => {
  const source = readFileSync(new URL("../src/components/CenterPanel.tsx", import.meta.url), "utf8");

  const start = source.indexOf("const attachTerminal = useCallback(");
  const bodyEnd = source.indexOf("\n  }, [", start);
  const depsStart = bodyEnd + "\n  }, [".length;
  const depsEnd = source.indexOf("]);", depsStart);
  const deps = source.slice(depsStart, depsEnd);

  assert.equal(
    deps.trim(),
    "",
    "attachTerminal's useCallback deps must stay empty — depending on `starting` re-triggers the attach " +
      "effect (backendStatus/sessionId/attachTerminal) on every creating→running transition, which " +
      "unconditionally calls terminal.focus() and steals focus from whatever the user is doing elsewhere " +
      "in the app at that moment",
  );
});

test("handleOpenWorkspace refreshes sessions before setting activeSessionId, so no consumer sees a real active id with an empty session list", () => {
  const source = readFileSync(new URL("../src/workspaceSessionController.ts", import.meta.url), "utf8");

  const start = source.indexOf("async function openWorkspace");
  const end = source.indexOf("\n  async function createSession", start);
  const body = source.slice(start, end);

  const refreshIndex = body.indexOf("const refreshed = await refreshSessions()");
  const setActiveIndex = body.indexOf("options.onActiveSession?.(match.id)");
  assert.ok(refreshIndex !== -1, "openWorkspace should await refreshSessions()");
  assert.ok(setActiveIndex !== -1, "openWorkspace should publish the restored active session");
  assert.ok(
    refreshIndex < setActiveIndex,
    "refreshSessions() must resolve before activeSessionId is set, otherwise a consumer reading ctx.sessions " +
      "for the just-set activeSessionId (e.g. ReviewTab) transiently sees no match during workspace switch",
  );
});
