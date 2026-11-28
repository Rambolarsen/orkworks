import test from "node:test";
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { build } from "esbuild";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";

const require = createRequire(import.meta.url);
const compiled = await build({
  entryPoints: [new URL("../src/components/SessionListPanel.tsx", import.meta.url).pathname],
  bundle: true, write: false, platform: "node", format: "cjs", packages: "external",
});
const component = { exports: {} };
new Function("require", "module", "exports", compiled.outputFiles[0].text)(require, component, component.exports);

function renderRow(overrides = {}, { unread = true, acknowledged = false } = {}) {
  const session = {
    id: "row", label: "Session", status: "ended", lifecycle: "dead",
    terminalOutcome: "error", memoryState: "remembered", resumeStrategy: "none",
    cwd: "/repo", created_at: "2026-11-06T10:00:00Z", ...overrides,
  };
  const html = renderToStaticMarkup(createElement(component.exports.default, {
    workspace: { path: "/repo" }, sessions: [session], activeSessionId: null,
    unreadIds: new Set(unread ? [session.id] : []),
    acknowledgedIds: new Set(acknowledged ? [session.id] : []), harnesses: [],
    onSelectSession() {}, onFocusTerminal() {}, onOpenWorkspace() {},
  }));
  const row = html.match(/<li class="([^"]*)" data-session-id="row"[^>]*>(.*?)<\/li>/s);
  assert.ok(row, "the real Sessions panel must render the fixture row");
  return { classes: row[1].split(" "), html: row[0] };
}

test("dead error rows stay remembered without loud or unread treatment and retain the failed icon", () => {
  const { classes, html } = renderRow();
  assert.ok(classes.includes("session-row--remembered"));
  assert.ok(!classes.includes("session-row--loud"));
  assert.ok(!classes.includes("session-row--unread"));
  assert.match(html, /class="status-indicator" data-attention="failed" role="img" aria-label="Error"/);
  assert.doesNotMatch(html, /class="session-row-attention"|status-indicator-unread/);
});

test("live failed rows remain loud and unread when unacknowledged", () => {
  const { classes, html } = renderRow({ lifecycle: "alive", status: "running", attention: "failed", memoryState: "live", terminalOutcome: undefined });
  assert.ok(!classes.includes("session-row--remembered"));
  assert.ok(classes.includes("session-row--loud"));
  assert.ok(classes.includes("session-row--unread"));
  assert.match(html, /status-indicator-unread/);
  assert.match(html, /class="session-row-attention" data-attention="failed">Failed/);
});

test("acknowledging a live failed row suppresses loud treatment independently of unread", () => {
  const { classes, html } = renderRow({ lifecycle: "alive", status: "running", attention: "failed", memoryState: "live", terminalOutcome: undefined }, { acknowledged: true, unread: true });
  assert.ok(!classes.includes("session-row--remembered"));
  assert.ok(!classes.includes("session-row--loud"));
  assert.ok(classes.includes("session-row--unread"));
  assert.match(html, /class="session-row-attention" data-attention="failed">Failed/);
  assert.match(html, /status-indicator-unread/);
});

test("a read live failed row keeps loud treatment and its failed status icon", () => {
  const { classes, html } = renderRow({ lifecycle: "alive", status: "running", attention: "failed", memoryState: "live", terminalOutcome: undefined }, { unread: false });
  assert.ok(classes.includes("session-row--loud"));
  assert.ok(!classes.includes("session-row--unread"));
  assert.match(html, /class="status-indicator" data-attention="failed" role="img" aria-label="Failed"/);
});

test("areSessionListRowPropsEqual ignores enrichment-only churn and detects rendered field changes", () => {
  const areEqual = component.exports.areSessionListRowPropsEqual;
  assert.equal(typeof areEqual, "function", "SessionListPanel must export areSessionListRowPropsEqual");
  const baseSession = {
    id: "row-1",
    label: "Build API",
    status: "running",
    lifecycle: "alive",
    attention: "working",
    needsUserInput: false,
    terminalOutcome: undefined,
    usageLimitReached: false,
    usageLimitInherited: false,
    capacityCheckPending: false,
    memoryState: "live",
    resumeStrategy: "none",
    harnessId: "codex",
    harness: "Codex",
    cwd: "/repo",
    branch: "main",
    dirty: false,
    created_at: "2026-11-28T10:00:00Z",
    enrichmentRevision: "rev-1",
    enrichment: { state: "ready", checkedAt: 1000 },
  };
  const harnesses = [];
  const itemRefs = new Map();
  const onSelect = () => {};
  const onOpenMenu = () => {};
  const baseProps = {
    session: baseSession,
    selected: false,
    isUnread: false,
    acknowledged: false,
    pendingAction: undefined,
    resuming: false,
    menuOpen: false,
    harnesses,
    activityTime: "2m",
    itemRefs,
    onSelect,
    onOpenMenu,
  };

  // Enrichment / stale / Git / summary-only updates must NOT trigger a row re-render.
  assert.equal(
    areEqual(baseProps, {
      ...baseProps,
      session: {
        ...baseSession,
        branch: "feature",
        dirty: true,
        changedFiles: 5,
        summary: "Updated summary",
        enrichmentRevision: "rev-2",
        enrichment: { state: "stale", checkedAt: 6000 },
      },
    }),
    true,
    "enrichment-only session changes must not re-render SessionListRow",
  );

  // Every rendered session field change MUST trigger a row re-render.
  for (const [field, value] of [
    ["label", "Renamed"],
    ["lifecycle", "dead"],
    ["sessionState", "archived"],
    ["terminalOutcome", "error"],
    ["attention", "needs_you"],
    ["usageLimitInherited", true],
    ["capacityCheckPending", true],
    ["harnessId", "claude"],
    ["harness", "Claude"],
  ]) {
    assert.equal(
      areEqual(baseProps, { ...baseProps, session: { ...baseSession, [field]: value } }),
      false,
      `changing rendered session field ${field} must re-render SessionListRow`,
    );
  }

  // Non-session row props must trigger a re-render.
  for (const [prop, value] of [
    ["selected", true],
    ["isUnread", true],
    ["acknowledged", true],
    ["pendingAction", "archive"],
    ["resuming", true],
    ["menuOpen", true],
    ["activityTime", "3m"],
    ["harnesses", [{ id: "codex", name: "Codex" }]],
  ]) {
    assert.equal(
      areEqual(baseProps, { ...baseProps, [prop]: value }),
      false,
      `changing prop ${prop} must re-render SessionListRow`,
    );
  }
});
