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
  assert.match(electronMainSource, /rendererOrigin/);
  assert.match(electronMainSource, /sanitizeRendererDiagnosticMessage/);
});

test("Electron main preserves the app URL for its local recovery retry", () => {
  assert.match(electronMainSource, /const originalUrl = process\.env\.VITE_DEV_SERVER_URL/);
  assert.match(electronMainSource, /pathToFileURL\(path\.join\(__dirname, "\.\.", "dist", "index\.html"\)\)/);
  assert.match(electronMainSource, /const recoveryUrl = recoveryDocumentUrl\(originalUrl\)/);
  assert.equal((electronMainSource.match(/<button/g) ?? []).length, 1);
  assert.match(electronMainSource, /location\.replace\(originalUrl\)/);
  assert.match(electronMainSource, /JSON\.stringify\(originalUrl\)/);
  assert.doesNotMatch(electronMainSource, /location\.reload\(\)/);
  assert.doesNotMatch(electronMainSource, /<script[^>]+src=|<link[^>]+href=/);
  assert.match(electronMainSource, /loadRecoveryDocument/);
});
