import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const settings = await readFile(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
const app = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
const css = await readFile(new URL("../src/App.css", import.meta.url), "utf8");

test("Settings Updates renders every UpdateStatus state explicitly", () => {
  for (const state of ["unavailable", "never-checked", "checking", "up-to-date", "available", "downloading", "downloaded", "installing", "error"]) {
    assert.match(settings, new RegExp(`case "${state}"`));
  }
});

test("Settings Updates renders version, channel, candidate, notes, and download progress", () => {
  for (const field of [
    /Current version[\s\S]*currentVersion/,
    /Channel[\s\S]*channel === "latest"[\s\S]*Stable[\s\S]*channel === "nightly"[\s\S]*Nightly/,
    /Available version[\s\S]*candidate\.identity\.version/,
    /Release tag[\s\S]*candidate\.identity\.tag/,
    /Release notes[\s\S]*candidate\.releaseNotes/,
    /progress\.percent[\s\S]*progress\.transferred[\s\S]*progress\.total/,
  ]) {
    assert.match(settings, field);
  }
});

test("Settings Updates exposes only the action valid for each state and disables busy actions", () => {
  assert.match(settings, /status\.state === "never-checked"[\s\S]*status\.state === "checking"[\s\S]*status\.state === "up-to-date"[\s\S]*status\.state === "unavailable"[\s\S]*Check for updates/);
  assert.match(settings, /disabled=\{!status \|\| status\.state === "checking" \|\| status\.state === "unavailable"\}/);
  assert.match(settings, /status\?\.state === "available" \|\| status\?\.state === "downloading"[\s\S]*onClick=\{onDownload\} disabled=\{status\.state === "downloading"\}[\s\S]*Download update/);
  assert.match(settings, /status\?\.state === "downloaded" \|\| status\?\.state === "installing"[\s\S]*onClick=\{onInstall\} disabled=\{status\.state === "installing"\}[\s\S]*Restart and install/);
  assert.match(settings, /status\?\.state === "error"[\s\S]*status\.message[\s\S]*onClick=\{onCheck\}>Retry/);
});

test("App owns the update subscription and shared update actions", () => {
  assert.match(app, /useEffect\(\(\) => window\.orkworks\.onUpdateStatus/);
  assert.match(app, /window\.orkworks\.checkForUpdates\(\)/);
  assert.match(app, /window\.orkworks\.downloadUpdate\(\)/);
  assert.match(app, /window\.orkworks\.requestUpdateInstall\(\)/);
});

test("an already-open Settings modal follows the menu command to Updates", () => {
  assert.match(app, /action === "check-for-updates"[\s\S]*openSettings\("updates"\)[\s\S]*checkForUpdates/);
  assert.match(app, /initialSection=\{settingsSection\}/);
  assert.match(settings, /useEffect\(\(\) => \{\s*setActiveSection\(initialSection\);\s*\}, \[initialSection\]\);/);
});

test("update errors and restart warnings use distinct application colors", () => {
  assert.match(css, /\.updates-error\s*\{\s*color:\s*var\(--state-error\);\s*\}/);
  assert.match(css, /\.updates-warning\s*\{\s*color:\s*var\(--state-warn\);\s*\}/);
});
