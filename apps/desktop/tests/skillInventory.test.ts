import test from "node:test";
import { createHash } from "node:crypto";
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


test("reads realistic APM deployment locks and attributes only recorded workspace sources", async (t) => {
  const f = await fixture(t);
  await f.put(f.workspace, ".agents/skills/generated");
  await f.put(f.workspace, ".claude/skills/generated");
  await f.put(f.workspace, ".agents/skills/unowned");
  const hashes = Object.fromEntries(Array.from({ length: 3000 }, (_, n) =>
    [`.agents/skills/generated/resources/file-${n}.md`, "sha256:" + "a".repeat(64)]));
  const text = JSON.stringify({ lockfile_version: "1", dependencies: [{
    repo_url: "owner/skill-package", version: "1.2.3",
    deployed_files: [".agents/skills/generated/SKILL.md", ".claude/skills/generated/SKILL.md"],
    deployed_file_hashes: hashes,
  }] });
  assert.ok(Buffer.byteLength(text) > 300000);
  await writeFile(join(f.workspace, "apm.lock.yaml"), text);
  const result = await discoverSkills({ workspace: f.workspace, home: f.home, env: {} });
  const owned = result.skills.filter(skill => skill.management === "apm");
  assert.equal(owned.length, 2);
  assert.ok(owned.every(skill => skill.packageSource === "owner/skill-package" && skill.version === "1.2.3"));
  assert.equal(result.skills.find(skill => skill.paths[0].endsWith("/unowned"))?.management, "local");
  assert.ok(!result.diagnostics.some(line => /APM lock/.test(line)));
});

test("matches personal APM ownership through discovered logical aliases without borrowing workspace ownership", async (t) => {
  const f = await fixture(t);
  const original = await f.put(f.root, "package/skills/example");
  await mkdir(join(f.home, ".agents/skills"), { recursive: true });
  await symlink(original, join(f.home, ".agents/skills/example"), "dir");
  await f.put(f.workspace, ".agents/skills/example");
  await mkdir(join(f.home, ".apm"));
  await writeFile(join(f.home, ".apm/apm.lock.yaml"), JSON.stringify({ dependencies: [{
    local_path: "/sources/example", version: "local",
    deployed_files: [".agents/skills/example/SKILL.md"],
  }] }));
  const result = await discoverSkills({ workspace: f.workspace, home: f.home, env: {} });
  const personal = result.skills.find(skill => skill.scope === "personal");
  assert.equal(personal?.management, "apm");
  assert.equal(personal?.packageSource, "/sources/example");
  assert.equal(personal?.resolvedPath, await realpath(original));
  assert.equal(result.skills.find(skill => skill.scope === "workspace")?.management, "local");
  await writeFile(join(f.home, ".apm/apm.lock.yaml"), JSON.stringify({ dependencies: [{
    local_path: "/sources/example", deployed_files: [join(await realpath(original), "SKILL.md")],
  }] }));
  const canonical = await discoverSkills({ workspace: f.workspace, home: f.home, env: {} });
  assert.equal(canonical.skills.find(skill => skill.scope === "personal")?.management, "apm");
  assert.equal(canonical.skills.find(skill => skill.scope === "workspace")?.management, "local");
});

test("keeps APM provenance unknown with diagnostics for oversized or malformed locks", async (t) => {
  const f = await fixture(t);
  await f.put(f.workspace, ".agents/skills/example");
  for (const text of ["x".repeat(2 * 1024 * 1024 + 1), "dependencies: [invalid"]) {
    await writeFile(join(f.workspace, "apm.lock.yaml"), text);
    const result = await discoverSkills({ workspace: f.workspace, home: f.home, env: {} });
    assert.equal(result.skills[0].management, "local");
    assert.equal(result.skills[0].packageSource, undefined);
    assert.ok(result.diagnostics.some(line => /workspace: APM lock provenance could not be read/.test(line)));
  }
});


test("retains coding-tool plugin registration when the same source has an APM ownership record", async (t) => {
  const f = await fixture(t);
  const original = await f.put(f.home, ".agents/skills/example");
  const plugin = join(f.home, "plugins/shared");
  await mkdir(join(plugin, "skills"), { recursive: true });
  await symlink(original, join(plugin, "skills/example"), "dir");
  await mkdir(join(f.home, ".claude/plugins"), { recursive: true });
  await writeFile(join(f.home, ".claude/plugins/installed_plugins.json"), JSON.stringify({
    plugins: { "sample@local": [{ scope: "user", installPath: plugin }] },
  }));
  await mkdir(join(f.home, ".apm"));
  await writeFile(join(f.home, ".apm/apm.lock.yaml"), JSON.stringify({ dependencies: [{
    repo_url: "owner/package", deployed_files: [".agents/skills/example/SKILL.md"],
  }] }));
  const result = await discoverSkills({ workspace: null, home: f.home, env: {} });
  assert.equal(result.skills.length, 1);
  assert.equal(result.skills[0].management, "apm");
  assert.equal(result.skills[0].packageSource, "owner/package");
  assert.deepEqual(result.skills[0].plugins, ["sample@local"]);
});


test("compares complete APM instruction files and distinguishes missing or unsupported hashes", async (t) => {
  const f = await fixture(t);
  const original = "---\nname: recorded\ndescription: Has an APM baseline.\n---\n" + "body\n".repeat(20000);
  const expected = "sha256:" + createHash("sha256").update(original).digest("hex");
  const files = ["same", "changed", "missing-hash", "unsupported-hash"];
  for (const name of files) await f.put(f.workspace, `.agents/skills/${name}`, original + (name === "changed" ? "Local edit.\n" : ""));
  await writeFile(join(f.workspace, "apm.lock.yaml"), JSON.stringify({ dependencies: [{
    repo_url: "owner/package", deployed_files: files.map(name => `.agents/skills/${name}/SKILL.md`),
    deployed_file_hashes: {
      ".agents/skills/same/SKILL.md": expected,
      ".agents/skills/changed/SKILL.md": expected,
      ".agents/skills/unsupported-hash/SKILL.md": "sha512:unsupported",
    },
  }] }));
  const result = await discoverSkills({ workspace: f.workspace, home: f.home, env: {} });
  const status = (name: string) => result.skills.find(skill => skill.paths[0].endsWith("/" + name))?.apmIntegrity?.status;
  assert.equal(status("same"), "matches");
  assert.equal(status("changed"), "differs");
  assert.equal(status("missing-hash"), "unavailable");
  assert.equal(status("unsupported-hash"), "unavailable");
  assert.ok(result.skills.every(skill => skill.management === "apm"));
  await writeFile(join(f.workspace, ".agents/skills/same/SKILL.md"), original + "Later edit.\n");
  const refreshed = await discoverSkills({ workspace: f.workspace, home: f.home, env: {} });
  assert.equal(refreshed.skills.find(skill => skill.paths[0].endsWith("/same"))?.apmIntegrity?.status, "differs");
});

test("does not choose a winning baseline when APM alias records have conflicting hashes", async (t) => {
  const f = await fixture(t);
  const text = "---\nname: alias\ndescription: Aliased instructions.\n---\nOriginal.\n";
  const original = await f.put(f.workspace, ".agents/skills/example", text);
  await mkdir(join(f.workspace, ".claude/skills"), { recursive: true });
  await symlink(original, join(f.workspace, ".claude/skills/example"), "dir");
  await writeFile(join(f.workspace, "apm.lock.yaml"), JSON.stringify({ dependencies: [{
    repo_url: "owner/package", deployed_files: [".agents/skills/example/SKILL.md", ".claude/skills/example/SKILL.md"],
    deployed_file_hashes: {
      ".agents/skills/example/SKILL.md": "sha256:" + createHash("sha256").update(text).digest("hex"),
      ".claude/skills/example/SKILL.md": "sha256:" + "0".repeat(64),
    },
  }] }));
  const result = await discoverSkills({ workspace: f.workspace, home: f.home, env: {} });
  assert.equal(result.skills.length, 1);
  assert.equal(result.skills[0].apmIntegrity?.status, "unavailable");
  assert.match(result.skills[0].apmIntegrity?.reason || "", /conflicting/i);
  for (const incomplete of [undefined, "sha512:unsupported"]) {
    await writeFile(join(f.workspace, "apm.lock.yaml"), JSON.stringify({ dependencies: [{
      repo_url: "owner/package", deployed_files: [".agents/skills/example/SKILL.md", ".claude/skills/example/SKILL.md"],
      deployed_file_hashes: {
        ".agents/skills/example/SKILL.md": "sha256:" + createHash("sha256").update(text).digest("hex"),
        ".claude/skills/example/SKILL.md": incomplete,
      },
    }] }));
    const unknown = await discoverSkills({ workspace: f.workspace, home: f.home, env: {} });
    assert.equal(unknown.skills[0].apmIntegrity?.status, "unavailable");
    assert.match(unknown.skills[0].apmIntegrity?.reason || "", /missing|unsupported/i);
  }
});

test("bounds APM file comparisons per file and per inventory without reporting limits as edits", async (t) => {
  const f = await fixture(t);
  const header = "---\nname: bounded\ndescription: Bounded hash comparison.\n---\n";
  const text = header + "x".repeat(1024 * 1024 - 1 - header.length);
  const hash = "sha256:" + createHash("sha256").update(text).digest("hex");
  const files = Array.from({ length: 17 }, (_, n) => `.agents/skills/bounded-${String(n).padStart(2, "0")}/SKILL.md`);
  for (const file of files) await f.put(f.workspace, file.slice(0, -9), text);
  await f.put(f.workspace, ".agents/skills/too-large", text + "extra");
  files.unshift(".agents/skills/too-large/SKILL.md");
  await writeFile(join(f.workspace, "apm.lock.yaml"), JSON.stringify({ dependencies: [{
    repo_url: "owner/package", deployed_files: files,
    deployed_file_hashes: Object.fromEntries(files.map(file => [file, hash])),
  }] }));
  const result = await discoverSkills({ workspace: f.workspace, home: f.home, env: {} });
  assert.equal(result.skills.filter(skill => skill.apmIntegrity?.status === "matches").length, 16);
  assert.equal(result.skills.filter(skill => skill.apmIntegrity?.status === "unavailable").length, 2);
  assert.equal(result.skills.filter(skill => skill.apmIntegrity?.status === "differs").length, 0);
});
