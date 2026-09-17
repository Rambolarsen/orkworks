import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const menu = await readFile(new URL("../electron/menuTemplate.ts", import.meta.url), "utf8");
const app = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");

test("Help menu exposes the shared update check command", () => {
  assert.match(menu, /check-for-updates/);
  assert.match(app, /check-for-updates[\s\S]*checkForUpdates/);
  assert.match(menu, /isCapturing[\s\S]*check-for-updates|check-for-updates[\s\S]*sendIfNotCapturing/);
});
