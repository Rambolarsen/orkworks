import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, writeFile, chmod, rm, realpath } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import * as apm from "../electron/skillInstall.ts";
import { normalizeSettings, readSettings, writeSettings } from "../electron/settingsMemory.ts";

const installHelp = "  --target TEXT\n  --only [apm|mcp]\n  --dry-run\n  --global\n  --trust-bin / --no-trust-bin\n";
function executableScript(version: string, help = installHelp, failHelp = false) {
  return `#!/bin/sh
case "$*" in
  --version) printf '%s\\n' 'Agent Package Manager (APM) CLI version ${version} (fixture)' ;;
  'install --help') ${failHelp ? 'exit 2' : `printf '%s' '${help}'`} ;;
  *) exit 2 ;;
esac
`;
}

test("explicit APM selection wins over PATH and an invalid selection never falls back", async t => {
  const root = await realpath(await mkdtemp(join(tmpdir(), "apm-selection-")));
  t.after(() => rm(root, { recursive: true, force: true }));
  for (const name of ["apm", "custom apm"]) {
    await writeFile(join(root, name), executableScript("0.35.0"));
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
  assert.equal(status.version, "0.35.0");
  assert.equal(status.compatible, true);
  await writeFile(join(root, "custom apm"), executableScript("0.35.0", installHelp.replace("  --global\n", "")));
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

test("APM admission follows capabilities across versions and reports missing options", async t => {
  const root = await mkdtemp(join(tmpdir(), "apm-capabilities-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const file = join(root, "apm");
  await writeFile(file, executableScript("99.0.0"));
  await chmod(file, 0o755);
  assert.equal((await apm.inspectApm({ PATH: root }, file)).compatible, true);
  for (const flag of ["--target", "--only", "--dry-run", "--global", "--no-trust-bin"]) {
    await writeFile(file, executableScript("0.33.0", installHelp.replace(flag, flag + "-other")));
    const status = await apm.inspectApm({ PATH: root }, file);
    assert.equal(status.compatible, false, flag);
    assert.ok(status.message.includes(flag), status.message);
  }
  await writeFile(file, executableScript("0.35.0", installHelp, true));
  const failed = await apm.inspectApm({ PATH: root }, file);
  assert.equal(failed.compatible, false);
  assert.match(failed.message, /capabilit.*could not be checked/i);
});
