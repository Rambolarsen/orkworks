import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { transpileModule, ModuleKind, JsxEmit } from "typescript";

const require = createRequire(import.meta.url);
const children = {
  "./CenterPanel": function CenterPanel() {},
  "./HistoricalTerminal": function HistoricalTerminal() {},
  "./EmptyState": function EmptyState() {},
};
const source = readFileSync(new URL("../src/components/TerminalPanel.tsx", import.meta.url), "utf8");
const { outputText } = transpileModule(source, {
  compilerOptions: { module: ModuleKind.CommonJS, jsx: JsxEmit.ReactJSX },
});
const component = { exports: {} };
new Function("require", "module", "exports", outputText)((id) => {
  if (id in children) return { default: children[id] };
  if (id === "../sessionRuntimeState") return require("../src/sessionRuntimeState.ts");
  if (id === "../terminalPresentation") return require("../src/terminalPresentation.ts");
  if (id === "../startupDiagnostics") return require("../src/startupDiagnostics.ts");
  return require(id);
}, component, component.exports);
const TerminalPanel = component.exports.default;
const props = { backendStatus: "ready", onBackendUnavailable() {}, onRetryBackend() {}, async onSelectPlan() {} };

test("terminal without a selection shows the empty state", () => {
  const view = TerminalPanel({ ...props, session: null });
  assert.equal(view.type, children["./EmptyState"]);
  assert.equal(view.props.message, "Select a session to open its terminal.");
});

test("dead session selects read-only history with its session id", () => {
  const view = TerminalPanel({ ...props, session: { id: "ended", lifecycle: "dead", status: "done" } });
  assert.equal(view.type, children["./HistoricalTerminal"]);
  assert.deepEqual(view.props, { sessionId: "ended", onSelectPlan: props.onSelectPlan });
});

test("remembered startup failures show fixed diagnostics alongside terminal replay", () => {
  const view = TerminalPanel({ ...props, session: {
    id: "ended", lifecycle: "dead", status: "error",
    startupFailure: { code: "codex_help_timeout", operation: "launch", occurredAt: "2026-11-07T12:00:00.000Z" },
  } });
  assert.equal(view.type, "div");
  assert.equal(view.props.className, "terminal-session-view");
  assert.match(view.props.children[0].props.children[0].props.children, /Session could not start/);
  assert.match(view.props.children[0].props.children[1].props.children, /Codex session isolation check timed out/);
  assert.equal(view.props.children[1].type, children["./HistoricalTerminal"]);
  assert.equal(view.props.children[1].props.sessionId, "ended");
});

for (const lifecycle of [undefined, "creating", "alive", "stopping"]) {
  for (const status of ["creating", "running"]) {
    test(`${lifecycle} session with ${status} status selects the interactive terminal`, () => {
      const view = TerminalPanel({ ...props, session: { id: "live", lifecycle, status } });
      assert.equal(view.type, children["./CenterPanel"]);
      assert.deepEqual(view.props, {
        ...props, sessionId: "live", starting: status === "creating",
      });
    });
  }
}

test("archived session with stale alive metadata selects read-only history", () => {
  const view = TerminalPanel({...props, session:{id:"saved",sessionState:"archived",lifecycle:"alive",status:"running"}});
  assert.equal(view.type, children["./HistoricalTerminal"]);
  assert.deepEqual(view.props, {sessionId:"saved", onSelectPlan: props.onSelectPlan});
});
