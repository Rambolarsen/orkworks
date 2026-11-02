import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, mkdir, writeFile, readFile, rm, realpath } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { inspectApm, SkillInstaller } from "../electron/skillInstall.ts";

// APM also writes user-level receipts for workspace operations. Run an unmodified
// executable only on a disposable CI runner, never in a developer's home.
const executable = process.env.CI === "true" ? process.env.ORKWORKS_APM_QUALIFICATION : undefined;
test("real APM supports reviewed workspace installs and existing manifests", { skip: !executable }, async t => {
  const root = await realpath(await mkdtemp(join(tmpdir(), "orkworks-apm-real-")));
  t.after(() => rm(root, { recursive: true, force: true }));
  const home = join(root, "user"), workspace = join(root, "workspace");
  await mkdir(home);
  await mkdir(workspace);
  const status = await inspectApm(process.env, executable!);
  assert.equal(status.compatible, true, status.message);
  let confirmations = 0;
  const installer = new SkillInstaller({
    home, env: process.env, workspace: () => workspace,
    executable: async () => status.path!,
    confirm: async () => { confirmations++; return true; },
  });
  await writeFile(join(workspace, "unmanaged.txt"), "preserve");
  for (const name of ["first", "second"]) {
    const source = join(root, name);
    await mkdir(source);
    const content = `---\nname: ${name}\ndescription: Disposable compatibility fixture.\n---\nDo nothing.\n`;
    await writeFile(join(source, "SKILL.md"), content);
    const manifestBefore = await readFile(join(workspace, "apm.yml"), "utf8").catch(() => null);
    const preview = await installer.preview({ scope: "workspace", source, target: "codex" });
    assert.equal(preview.available, true, preview.reason);
    assert.equal(await readFile(join(workspace, "apm.yml"), "utf8").catch(() => null), manifestBefore);
    const result = await installer.install(preview.id);
    assert.equal(result.ok, true, result.message);
    assert.equal(await readFile(join(workspace, ".agents/skills", name, "SKILL.md"), "utf8"), content);
  }
  assert.match(await readFile(join(workspace, ".agents/skills/first/SKILL.md"), "utf8"), /name: first/);
  assert.equal(await readFile(join(workspace, "unmanaged.txt"), "utf8"), "preserve");
  assert.equal(confirmations, 2);
});
