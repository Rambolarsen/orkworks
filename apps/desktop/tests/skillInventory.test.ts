import test from "node:test";
import assert from "node:assert/strict";
import {
  mkdtemp,
  mkdir,
  writeFile,
  symlink,
  rm,
  realpath,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { discoverSkills, readSkillDetail } from "../electron/skillInventory.ts";

async function fixture(t: any) {
  const root = await mkdtemp(join(tmpdir(), "orkworks-skills-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const workspace = join(root, "repo");
  const home = join(root, "user");
  await mkdir(workspace);
  await mkdir(home);
  const put = async (
    base: string,
    path: string,
    text = "---\nname: duplicate\ndescription: A useful skill.\n---\nInstructions.",
  ) => {
    const directory = join(base, path);
    await mkdir(directory, { recursive: true });
    await writeFile(join(directory, "SKILL.md"), text);
    return directory;
  };
  return { root, workspace, home, put };
}

test("keeps same-named workspace and personal skills distinct with known source roots", async (t) => {
  const f = await fixture(t);
  await f.put(f.workspace, ".agents/skills/example");
  await f.put(f.home, ".agents/skills/example");
  const result = await discoverSkills({
    workspace: f.workspace,
    home: f.home,
    env: {},
  });
  assert.equal(result.skills.filter((s) => s.name === "duplicate").length, 2);
  assert.deepEqual(
    result.skills
      .filter((s) => s.name === "duplicate")
      .map((s) => s.scope)
      .sort(),
    ["personal", "workspace"],
  );
  assert.ok(result.skills.every((s) => s.tools.includes("codex")));
  assert.equal(result.workspace, f.workspace);
});

test("personal inventory works with no open workspace and details require an inventory identity", async (t) => {
  const f = await fixture(t);
  await f.put(f.home, ".claude/skills/example");
  const result = await discoverSkills({
    workspace: null,
    home: f.home,
    env: {},
  });
  assert.equal(result.skills.length, 1);
  assert.equal(result.skills[0].scope, "personal");
  assert.match(
    await readSkillDetail(result, result.skills[0].id),
    /Instructions/,
  );
  await assert.rejects(
    readSkillDetail(result, "/etc/passwd"),
    /no longer|Unknown/i,
  );
});

test("coalesces same-scope symlink aliases without merging scopes", async (t) => {
  const f = await fixture(t);
  const original = await f.put(f.home, ".agents/skills/example");
  await mkdir(join(f.home, ".claude/skills"), { recursive: true });
  await symlink(original, join(f.home, ".claude/skills/example"), "dir");
  const result = await discoverSkills({
    workspace: f.workspace,
    home: f.home,
    env: {},
  });
  assert.equal(result.skills.length, 1);
  assert.equal(result.skills[0].paths.length, 2);
  assert.ok(result.skills[0].tools.includes("claude"));
});

test("reports malformed, oversized and cyclic skill roots without empty-success claims", async (t) => {
  const f = await fixture(t);
  await f.put(
    f.workspace,
    ".agents/skills/broken",
    "---\nname: [bad\n---\nbody",
  );
  await f.put(f.workspace, ".agents/skills/large", "x".repeat(65537));
  await symlink(
    join(f.workspace, ".agents/skills"),
    join(f.workspace, ".agents/skills/loop"),
    "dir",
  );
  const result = await discoverSkills({
    workspace: f.workspace,
    home: f.home,
    env: {},
  });
  assert.ok(result.diagnostics.some((s) => /broken/.test(s)));
  assert.ok(result.diagnostics.some((s) => /large/.test(s)));
  assert.ok(result.diagnostics.some((s) => /cycle|alias/i.test(s)));
});

test("includes configured OpenCode paths and Claude plugin registrations with honest provenance", async (t) => {
  const f = await fixture(t);
  const configured = await realpath(await f.put(f.workspace, "skills/extra"));
  await writeFile(
    join(f.workspace, "opencode.json"),
    JSON.stringify({ skills: { paths: ["skills"] } }),
  );
  const plugin = join(f.home, "plugins/sample");
  await f.put(plugin, "skills/review");
  await mkdir(join(f.home, ".claude/plugins"), { recursive: true });
  await writeFile(
    join(f.home, ".claude/plugins/installed_plugins.json"),
    JSON.stringify({
      version: 2,
      plugins: { "sample@local": [{ scope: "user", installPath: plugin }] },
    }),
  );
  const result = await discoverSkills({
    workspace: f.workspace,
    home: f.home,
    env: {},
  });
  assert.ok(
    result.skills.some(
      (s) => s.resolvedPath === configured && s.tools.includes("opencode"),
    ),
  );
  assert.deepEqual(result.skills.find((s) => s.management === "plugin")?.plugins, ["sample@local"]);
});

test("refuses detail after a discovered file is replaced by an external symlink", async (t) => {
  const f = await fixture(t);
  const directory = await f.put(f.home, ".agents/skills/example");
  const inventory = await discoverSkills({
    workspace: null,
    home: f.home,
    env: {},
  });
  await writeFile(join(f.root, "secret"), "secret");
  await rm(join(directory, "SKILL.md"));
  await symlink(join(f.root, "secret"), join(directory, "SKILL.md"));
  await assert.rejects(
    readSkillDetail(inventory, inventory.skills[0].id),
    /changed|regular|link/i,
  );
});

// A local alias must not erase the registered plugin identity.
test("retains all plugin registrations when roots and local aliases share a skill", async (t) => {
  const f = await fixture(t);
  const original = await f.put(f.home, ".agents/skills/example");
  const plugin = join(f.home, "plugins/shared");
  await mkdir(join(plugin, "skills"), { recursive: true });
  await symlink(original, join(plugin, "skills/example"), "dir");
  await mkdir(join(f.home, ".claude/plugins"), { recursive: true });
  await writeFile(join(f.home, ".claude/plugins/installed_plugins.json"), JSON.stringify({
    plugins: {
      "alpha@local": [{ scope: "user", installPath: plugin }],
      "beta@local": [{ scope: "user", installPath: plugin }],
    },
  }));
  const result = await discoverSkills({ workspace: null, home: f.home, env: {} });
  assert.equal(result.skills.length, 1);
  assert.equal(result.skills[0].management, "plugin");
  assert.deepEqual(result.skills[0].plugins, ["alpha@local", "beta@local"]);
});

async function codexPlugin(f: Awaited<ReturnType<typeof fixture>>, plugin: string, version: string, manifest: object, portable = false) {
  const directory = join(f.home, ".codex/plugins/cache/test-market", plugin, version);
  const file = join(directory, portable ? "plugin.json" : ".codex-plugin/plugin.json");
  await mkdir(join(file, ".."), { recursive: true });
  await writeFile(file, JSON.stringify(manifest));
  return directory;
}

test("includes Codex system and manifest-declared cached plugin skills without claiming an active version", async (t) => {
  const f = await fixture(t);
  await f.put(f.home, ".codex/skills/.system/system");
  await f.put(f.home, ".codex/skills/.other/hidden");
  const legacy = await codexPlugin(f, "legacy", "1.0", { name: "legacy", skills: "./workflows", version: "1.0" });
  await f.put(legacy, "workflows/first");
  await f.put(legacy, "skills/not-declared");
  const portable = await codexPlugin(f, "portable", "2.0", { name: "portable", version: "2.0" }, true);
  await f.put(portable, "skills/second");
  const older = await codexPlugin(f, "portable", "1.0", { name: "portable", version: "1.0" }, true);
  await f.put(older, "skills/second");
  const result = await discoverSkills({ workspace: null, home: f.home, env: {} });
  assert.equal(result.skills.length, 4);
  assert.ok(result.skills.every(s => s.tools.includes("codex") && s.scope === "personal"));
  assert.deepEqual(result.skills.filter(s => s.plugins?.includes("portable@test-market")).map(s => s.version).sort(), ["1.0", "2.0"]);
  assert.equal(result.skills.find(s => s.plugins?.includes("legacy@test-market"))?.version, "1.0");
  assert.ok(result.diagnostics.some(s => /Codex.*active version.*not verified/.test(s)));
});

test("uses CODEX_HOME for system and plugin discovery and does not walk escaped manifest paths", async (t) => {
  const f = await fixture(t);
  const customHome = join(f.root, "custom-codex");
  await f.put(customHome, "skills/.system/system");
  const outside = await f.put(f.root, "outside");
  const bad = await codexPlugin(f, "bad", "1", { name: "bad", skills: outside });
  const linked = await codexPlugin(f, "linked", "1", { name: "linked", skills: "./skills" });
  await symlink(outside, join(linked, "skills"), "dir");
  await codexPlugin(f, "traversal", "1", { name: "traversal", skills: "./../../../../../../outside" });
  const malformed = await codexPlugin(f, "malformed", "1", {});
  await writeFile(join(malformed, ".codex-plugin/plugin.json"), "broken {");
  const unsupported = await codexPlugin(f, "unsupported", "1", { name: "unsupported", skills: ["./skills"] });
  await f.put(unsupported, "skills/unsupported");
  const result = await discoverSkills({ workspace: null, home: f.home, env: { CODEX_HOME: customHome } });
  assert.equal(result.skills.length, 1);
  assert.ok(result.skills[0].tools.includes("codex"));
  // Default-home cache must not leak into the overridden inventory.
  assert.ok(!result.roots.some(r => r.path.startsWith(join(f.home, ".codex"))));
  const rejected = await discoverSkills({ workspace: null, home: f.home, env: {} });
  assert.equal(rejected.skills.length, 0);
  for (const name of ["bad", "linked", "traversal", "malformed", "unsupported"])
    assert.ok(rejected.diagnostics.some(s => s.includes(name)), name);
});

test("bounds Codex cache traversal and reports incomplete discovery", async (t) => {
  const f = await fixture(t);
  for (let n = 0; n < 140; n++) {
    const directory = await codexPlugin(f, `plugin-${String(n).padStart(3, "0")}`, "1", { name: `plugin-${String(n).padStart(3, "0")}`, skills: "./skills" });
    await f.put(directory, "skills/example");
  }
  const result = await discoverSkills({ workspace: null, home: f.home, env: {} });
  assert.ok(result.skills.length > 0 && result.skills.length < 140);
  assert.ok(result.diagnostics.some(s => /limit|budget/.test(s)));
});


test("counts valid bounded skill metadata even when its instruction body exceeds the detail limit", async (t) => {
  const f = await fixture(t);
  const directory = await f.put(f.home, ".codex/skills/.system/long", "---\nname: long\ndescription: Small metadata, long body.\n---\n" + "x".repeat(100000));
  await f.put(f.home, ".codex/skills/.system/huge-header", "---\nname: huge\ndescription: Invalidly large metadata.\npadding: " + "x".repeat(65536) + "\n---\n");
  const inventory = await discoverSkills({ workspace: null, home: f.home, env: {} });
  assert.equal(inventory.skills.length, 1);
  assert.equal(inventory.skills[0].name, "long");
  assert.ok(inventory.skills[0].tools.includes("codex"));
  assert.ok(!inventory.diagnostics.some(s => s.includes(directory)));
  // Inventory metadata does not grant an unbounded detail read.
  await assert.rejects(readSkillDetail(inventory, inventory.skills[0].id), /bounded|limit/);
});
