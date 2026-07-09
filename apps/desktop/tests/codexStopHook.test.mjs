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
const stopHookCommand = JSON.parse(readFileSync(hooksJson, "utf8")).hooks.Stop[0].hooks[0].command;

test("codex stop hook points Stop to the JSON wrapper", () => {
  const hooks = readFileSync(hooksJson, "utf8");

  assert.match(hooks, /"Stop"/);
  assert.match(hooks, /doc-check-stop\.sh/);
});

test("codex stop hook resolves the wrapper from the git root instead of the session cwd", () => {
  assert.match(stopHookCommand, /git rev-parse --show-toplevel/);
  assert.doesNotMatch(stopHookCommand, /bash '\.codex\/hooks\/doc-check-stop\.sh'/);

  const stdout = execFileSync("bash", ["-lc", stopHookCommand], {
    cwd: resolve(repoRoot, "apps", "desktop"),
    env: { ...process.env, ORKWORKS_DOC_CHECK_OUTPUT: "" },
    encoding: "utf8",
  });

  assert.deepEqual(JSON.parse(stdout), {});
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

test("codex stop hook wrapper tolerates a non-numeric injected exit code", () => {
  const stdout = execFileSync("bash", [wrapper], {
    cwd: repoRoot,
    env: {
      ...process.env,
      ORKWORKS_DOC_CHECK_OUTPUT: "boom",
      ORKWORKS_DOC_CHECK_EXIT_CODE: "wat",
    },
    encoding: "utf8",
  });

  assert.deepEqual(JSON.parse(stdout), {
    systemMessage: "[doc-check] Hook failed with invalid exit code wat.\nboom",
  });
});
