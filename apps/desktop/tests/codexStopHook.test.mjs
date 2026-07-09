import test from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const testDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(testDir, "..", "..", "..");
const wrapper = resolve(repoRoot, ".codex", "hooks", "doc-check-stop.sh");
const hooksJson = resolve(repoRoot, ".codex", "hooks.json");

test("codex stop hook points Stop to the JSON wrapper", () => {
  const hooks = readFileSync(hooksJson, "utf8");

  assert.match(hooks, /"Stop"/);
  assert.match(hooks, /doc-check-stop\.sh/);
});

test("codex stop hook wrapper emits {} when doc-check is quiet", () => {
  const stdout = execFileSync("bash", [wrapper], {
    cwd: repoRoot,
    env: { ...process.env, ORKWORKS_DOC_CHECK_OUTPUT: "" },
    encoding: "utf8",
  });

  assert.deepEqual(JSON.parse(stdout), {});
});

test("codex stop hook wrapper emits systemMessage JSON when doc-check reports updates", () => {
  const message = "[doc-check] Consider updating before closing:\n  • README.md";
  const stdout = execFileSync("bash", [wrapper], {
    cwd: repoRoot,
    env: { ...process.env, ORKWORKS_DOC_CHECK_OUTPUT: message },
    encoding: "utf8",
  });

  assert.deepEqual(JSON.parse(stdout), { systemMessage: message });
});

test("codex stop hook wrapper preserves failure details as JSON instead of plain text", () => {
  const stdout = execFileSync("bash", [wrapper], {
    cwd: repoRoot,
    env: {
      ...process.env,
      ORKWORKS_DOC_CHECK_OUTPUT: "boom",
      ORKWORKS_DOC_CHECK_EXIT_CODE: "7",
    },
    encoding: "utf8",
  });

  assert.deepEqual(JSON.parse(stdout), {
    systemMessage: "[doc-check] Hook failed with exit 7.\nboom",
  });
});
