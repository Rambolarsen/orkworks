import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const mainSource = readFileSync(new URL("../src/main.tsx", import.meta.url), "utf8");
const electronMainSource = readFileSync(new URL("../electron/main.ts", import.meta.url), "utf8");
const boundarySource = readFileSync(
  new URL("../src/components/ErrorBoundary.tsx", import.meta.url),
  "utf8",
);

test("main.tsx wraps App in ErrorBoundary so a render exception doesn't blank the whole window", () => {
  assert.match(mainSource, /<ErrorBoundary>\s*<App\s*\/>\s*<\/ErrorBoundary>/);
});

test("ErrorBoundary implements getDerivedStateFromError and componentDidCatch", () => {
  assert.match(boundarySource, /static getDerivedStateFromError/);
  assert.match(boundarySource, /componentDidCatch/);
});

test("ErrorBoundary logs the caught error to the console for DevTools visibility", () => {
  assert.match(boundarySource, /console\.error/);
});

test("ErrorBoundary formats the caught error via the shared pure formatter", () => {
  assert.match(boundarySource, /formatBoundaryError/);
});

test("Electron main registers renderer load, process, and console diagnostics", () => {
  assert.match(electronMainSource, /webContents\.on\("did-fail-load"/);
  assert.match(electronMainSource, /webContents\.on\("render-process-gone"/);
  assert.match(electronMainSource, /webContents\.on\("console-message"/);
  assert.match(electronMainSource, /slice\(0, 200\)/);
  assert.match(electronMainSource, /new URL\(.*\)\.origin/);
});

test("Electron main has a local recovery document with one reload action", () => {
  const recoveryDocument = electronMainSource.match(/const RECOVERY_HTML = `([\s\S]*?)`;/)?.[1] ?? "";
  assert.notEqual(recoveryDocument, "");
  assert.equal((recoveryDocument.match(/<button/g) ?? []).length, 1);
  assert.match(recoveryDocument, /location\.reload\(\)/);
  assert.doesNotMatch(recoveryDocument, /<script[^>]+src=|<link[^>]+href=/);
  assert.match(electronMainSource, /loadRecoveryDocument/);
});
