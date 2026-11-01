import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, writeFile, chmod, rm, realpath } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import * as apm from "../electron/skillInstall.ts";
import { normalizeSettings, readSettings, writeSettings } from "../electron/settingsMemory.ts";

test("explicit APM selection wins over PATH and an invalid selection never falls back", async t => {
  const root = await realpath(await mkdtemp(join(tmpdir(), "apm-selection-")));
  t.after(() => rm(root, { recursive: true, force: true }));
  for (const name of ["apm", "custom apm"]) {
    await writeFile(join(root, name), "#!/bin/sh\nprintf 'Agent Package Manager (APM) CLI version 0.33.0 (fixture)\\n'\n");
    await chmod(join(root, name), 0o755);
  }
  const env = { PATH: root };
  assert.equal(await apm.findApm(env, join(root, "custom apm")), join(root, "custom apm"));
  await assert.rejects(apm.findApm(env, join(root, "missing")));
  await assert.rejects(apm.findApm(env, root));
  await assert.rejects(apm.findApm(env, "relative/apm"));
  assert.equal(await apm.findApm(env, null), join(root, "apm"));
  const status = await apm.inspectApm(env, join(root, "custom apm"));
  assert.equal(status.path, join(root, "custom apm"));
  assert.equal(status.version, "0.33.0");
  assert.equal(status.compatible, true);
  await writeFile(join(root, "custom apm"), "#!/bin/sh\nprintf 'CLI version 0.34.0\\n'\n");
  assert.equal((await apm.inspectApm(env, join(root, "custom apm"))).compatible, false);
  const missing = await apm.inspectApm(env, join(root, "missing"));
  assert.equal(missing.available, false);
  assert.equal(missing.executableOverride, join(root, "missing"));
});

test("APM settings default to automatic detection and persist without replacing other settings", async t => {
  const root = await mkdtemp(join(tmpdir(), "apm-settings-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  assert.equal(normalizeSettings({}).apmExecutablePath, null);
  assert.equal(normalizeSettings({ apmExecutablePath: 123 }).apmExecutablePath, null);
  assert.equal(normalizeSettings({ apmExecutablePath: "relative" }).apmExecutablePath, null);
  const settings = normalizeSettings({ apmExecutablePath: "/custom/apm", untouched: "retained" });
  writeSettings(root, settings);
  assert.equal(readSettings(root).apmExecutablePath, "/custom/apm");
  assert.equal(readSettings(root).untouched, "retained");
});
