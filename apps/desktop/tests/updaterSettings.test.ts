import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const settings = await readFile(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");

test("Settings Updates renders all lifecycle states and actions", () => {
  for (const text of ["Updates", "Check for updates", "Download update", "Restart and install", "up-to-date", "downloaded", "unavailable"]) {
    assert.match(settings, new RegExp(text, "i"));
  }
  assert.match(settings, /onUpdateStatus/);
});
