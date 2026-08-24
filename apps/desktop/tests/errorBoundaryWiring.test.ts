import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const mainSource = readFileSync(new URL("../src/main.tsx", import.meta.url), "utf8");
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
