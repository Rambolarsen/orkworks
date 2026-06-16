import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";

const launch = JSON.parse(fs.readFileSync(new URL("../../../.vscode/launch.json", import.meta.url), "utf8"));

test("VS Code F5 launches the same terminal command as the CLI", () => {
  const config = launch.configurations.find((entry) => entry.name === "Launch OrkWorks");

  assert.equal(config.type, "node-terminal");
  assert.equal(config.cwd, "${workspaceFolder}");
  assert.equal(config.command, "pnpm --dir apps/desktop dev");
  assert.equal(config.runtimeExecutable, undefined);
  assert.equal(config.runtimeArgs, undefined);
});

test("VS Code launch configs do not point Electron at a possibly stale Vite server", () => {
  const staleViteConfigs = launch.configurations.filter(
    (entry) => entry.env?.VITE_DEV_SERVER_URL === "http://localhost:5173",
  );

  assert.deepEqual(staleViteConfigs, []);
});
