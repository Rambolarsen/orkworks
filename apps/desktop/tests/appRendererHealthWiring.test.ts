import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const appSource = readFileSync(
  new URL("../src/App.tsx", import.meta.url),
  "utf8",
);

test("App reads settings.debug.rendererHealthLogMs to start the health probe", () => {
  assert.match(appSource, /rendererHealthLogMs/);
});

test("App uses setInterval for the health probe and clears it in cleanup", () => {
  assert.match(appSource, /setInterval\([\s\S]*?clearInterval/);
});

test("App exposes window.__orkworksCaptureRendererHealth for ad-hoc DevTools capture", () => {
  assert.match(appSource, /__orkworksCaptureRendererHealth/);
});