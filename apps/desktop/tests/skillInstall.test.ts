import test from "node:test";
import assert from "node:assert/strict";
import {
  mkdtemp,
  mkdir,
  writeFile,
  readFile,
  rm,
  realpath,
  symlink,
} from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import {
  SkillInstaller,
  skillProcessEnvironment,
} from "../electron/skillInstall.ts";
async function fixture(t: any) {
  const root = await realpath(await mkdtemp(join(tmpdir(), "orkworks-add-")));
  t.after(() => rm(root, { recursive: true, force: true }));
  const home = join(root, "home"),
    workspace = join(root, "repo"),
    source = join(root, "example"),
    executable = join(root, "apm");
  for (const path of [home, workspace, source]) await mkdir(path);
  await writeFile(executable, "fixture executable");
  await writeFile(
    join(source, "SKILL.md"),
    "---\nname: example\ndescription: Example.\n---\nUse it.",
  );
  let current: string | null = workspace;
  let confirmed = true;
  let confirmations = 0;
  const operations: Array<{ args: string[]; cwd: string }> = [];
  const installer = new SkillInstaller({
    home,
    env: { PATH: "/usr/bin" },
    workspace: () => current,
    executable: async () => executable,
    confirm: async () => {
      confirmations++;
      return confirmed;
    },
    run: async (_file, args, cwd) => {
      operations.push({ args, cwd });
      if (args[0] === "--version")
        return "Agent Package Manager (APM) CLI version 0.33.0 (18c4c43)";
      if (args.includes("--dry-run")) {
        assert.notEqual(cwd, workspace);
        assert.notEqual(cwd, home);
        await writeFile(join(cwd, "apm.yml"), "preview writes stay temporary");
        return "Dry run completed: would install 1 APM dependency";
      }
      const destination = args.includes("--global") ? home : cwd;
      await mkdir(join(destination, ".agents/skills/example"), {
        recursive: true,
      });
      await writeFile(
        join(destination, ".agents/skills/example/SKILL.md"),
        "installed",
      );
      return "Installed 1 APM dependency";
    },
  });
  return {
    root,
    home,
    workspace,
    source,
    installer,
    operations,
    setWorkspace: (v: string | null) => (current = v),
    decline: () => (confirmed = false),
    count: () => confirmations,
  };
}
test("previews in private storage and only installs the confirmed workspace destination", async (t) => {
  const f = await fixture(t);
  const preview = await f.installer.preview({
    scope: "workspace",
    source: f.source,
    target: "codex",
  });
  assert.equal(preview.available, true);
  await assert.rejects(readFile(join(f.workspace, "apm.yml")));
  assert.equal((await f.installer.install(preview.id)).ok, true);
  assert.equal(
    await readFile(
      join(f.workspace, ".agents/skills/example/SKILL.md"),
      "utf8",
    ),
    "installed",
  );
  assert.equal(f.count(), 1);
  assert.equal((await f.installer.install(preview.id)).ok, false);
});
test("personal install uses explicit global scope even after workspace navigation", async (t) => {
  const f = await fixture(t);
  const preview = await f.installer.preview({
    scope: "personal",
    source: f.source,
    target: "codex",
  });
  f.setWorkspace(null);
  assert.equal((await f.installer.install(preview.id)).ok, true);
  assert.equal(
    await readFile(join(f.home, ".agents/skills/example/SKILL.md"), "utf8"),
    "installed",
  );
  assert.ok(f.operations.at(-1)?.args.includes("--global"));
});
test("source drift and workspace switches invalidate reviewed additions", async (t) => {
  const f = await fixture(t);
  const preview = await f.installer.preview({
    scope: "workspace",
    source: f.source,
    target: "codex",
  });
  await writeFile(join(f.source, "SKILL.md"), "changed");
  assert.equal((await f.installer.install(preview.id)).ok, false);
  const fresh = await fixture(t);
  const review = await fresh.installer.preview({
    scope: "workspace",
    source: fresh.source,
    target: "codex",
  });
  fresh.setWorkspace(null);
  assert.equal((await fresh.installer.install(review.id)).ok, false);
  assert.equal(fresh.count(), 0);
});
test("declining confirmation cannot deploy files", async (t) => {
  const f = await fixture(t);
  const preview = await f.installer.preview({
    scope: "workspace",
    source: f.source,
    target: "codex",
  });
  f.decline();
  assert.equal((await f.installer.install(preview.id)).ok, false);
  await assert.rejects(
    readFile(join(f.workspace, ".agents/skills/example/SKILL.md")),
  );
});
test("mixed packages, unrelated existing manifests and unresolved remotes stay unavailable", async (t) => {
  const f = await fixture(t);
  await writeFile(
    join(f.source, "apm.yml"),
    "lifecycle: {post-install: [bad]}",
  );
  assert.equal(
    (
      await f.installer.preview({
        scope: "workspace",
        source: f.source,
        target: "codex",
      })
    ).available,
    false,
  );
  await rm(join(f.source, "apm.yml"));
  await writeFile(
    join(f.workspace, "apm.yml"),
    "name: repo\ntargets: [claude]\ndependencies: {apm: [owner/unrelated]}\n",
  );
  assert.equal(
    (
      await f.installer.preview({
        scope: "workspace",
        source: f.source,
        target: "codex",
      })
    ).available,
    false,
  );
  const remote = await f.installer.preview({
    scope: "personal",
    source: "owner/repo#main",
    target: "codex",
  });
  assert.equal(remote.available, false);
  assert.match(remote.reason || "", /remote|local|resolution/i);
});
test("rejects options, credential-bearing sources and inherited policy escapes", async (t) => {
  const f = await fixture(t);
  for (const source of [
    "--force",
    "https://secret@github.com/org/repo",
    "x; touch /tmp/bad",
  ])
    await assert.rejects(
      f.installer.preview({ scope: "workspace", source, target: "codex" }),
    );
  const env = skillProcessEnvironment({
    ORKWORKS_REPORT_TOKEN: "secret",
    ORKWORKS_API_TOKEN: "secret",
    APM_POLICY_DISABLE: "1",
    APM_NO_SCRIPTS: "1",
    GH_TOKEN: "git auth",
    PATH: "/usr/bin",
  });
  assert.equal(env.ORKWORKS_REPORT_TOKEN, undefined);
  assert.equal(env.APM_POLICY_DISABLE, undefined);
  assert.equal(env.APM_NO_SCRIPTS, undefined);
  assert.equal(env.GH_TOKEN, "git auth");
});

test("a symlinked personal metadata root cannot redirect APM writes", async (t) => {
  const f = await fixture(t);
  await mkdir(join(f.root, "external"));
  await symlink(join(f.root, "external"), join(f.home, ".apm"), "dir");
  const preview = await f.installer.preview({
    scope: "personal",
    source: f.source,
    target: "codex",
  });
  assert.equal(preview.available, false);
  assert.match(preview.reason || "", /symbolic/i);
});
test("manifest drift after preview cannot reach deployment", async (t) => {
  const f = await fixture(t);
  const review = await f.installer.preview({
    scope: "workspace",
    source: f.source,
    target: "codex",
  });
  await writeFile(
    join(f.workspace, "apm.yml"),
    "name: repo\nincludes: ../outside\n",
  );
  assert.equal((await f.installer.install(review.id)).ok, false);
  await assert.rejects(
    readFile(join(f.workspace, ".agents/skills/example/SKILL.md")),
  );
});

test("binary asset changes invalidate a reviewed source byte-for-byte", async (t) => {
  const f = await fixture(t);
  await mkdir(join(f.source, "assets"));
  await writeFile(join(f.source, "assets/blob"), Buffer.from([0xff]));
  const preview = await f.installer.preview({
    scope: "workspace",
    source: f.source,
    target: "codex",
  });
  assert.equal(preview.available, true);
  await writeFile(join(f.source, "assets/blob"), Buffer.from([0xfe]));
  assert.equal((await f.installer.install(preview.id)).ok, false);
});
test("custom includes cannot expand the reviewed install scope", async (t) => {
  const f = await fixture(t);
  await writeFile(
    join(f.workspace, "apm.yml"),
    "name: repo\nincludes: ../outside\n",
  );
  assert.equal(
    (
      await f.installer.preview({
        scope: "workspace",
        source: f.source,
        target: "codex",
      })
    ).available,
    false,
  );
});
