import test from "node:test";
import assert from "node:assert/strict";
import { parseHarnessDraft, type HarnessEditorMode } from "../src/harnessTypes.ts";

function diagnostics(models: unknown, mode: HarnessEditorMode = "override") {
  const definition = mode === "override" ? { models } : {
    id: "tool", name: "Tool", launch: { kind: "platform-shell", login: true }, labelResetCommands: [], models,
  };
  return parseHarnessDraft(JSON.stringify(definition), mode).diagnostics;
}

for (const mode of ["override", "create", "custom"] as const) {
  test(`${mode} accepts optional models and each complete model kind`, () => {
    for (const models of [undefined, null, { kind: "static", models: ["model-a"] },
      { kind: "command", command: "tool", args: [] }, { kind: "codex-app-server" }, { kind: "http" }]) {
      assert.deepEqual(diagnostics(models, mode), []);
    }
  });

  test(`${mode} requires complete static and command model capabilities`, () => {
    assert.deepEqual(diagnostics({ kind: "static" }, mode), [
      { code: "missing_field", message: "Required field models is missing.", path: "$.models.models" },
    ]);
    assert.deepEqual(diagnostics({ kind: "command" }, mode), [
      { code: "missing_field", message: "Required field command is missing.", path: "$.models.command" },
      { code: "missing_field", message: "Required field args is missing.", path: "$.models.args" },
    ]);
  });
}

test("static model diagnostics reject foreign fields after validating their model list", () => {
  assert.deepEqual(diagnostics({ kind: "static", models: [false], command: "tool", args: [] }), [
    { code: "invalid_schema", message: "Expected an array of strings.", path: "$.models.models" },
    { code: "invalid_capability_combination", message: "Field command is not valid for static models.", path: "$.models.command" },
    { code: "invalid_capability_combination", message: "Field args is not valid for static models.", path: "$.models.args" },
  ]);
});

test("command model diagnostics preserve unknown, required, command and foreign-field order", () => {
  assert.deepEqual(diagnostics({ kind: "command", surprise: true, command: " ", models: [] }), [
    { code: "unknown_field", message: "Unknown custom definition field surprise.", path: "$.models.surprise" },
    { code: "missing_field", message: "Required field args is missing.", path: "$.models.args" },
    { code: "invalid_model_command", message: "Model command is required.", path: "$.models.command" },
    { code: "invalid_capability_combination", message: "Field models is not valid for command models.", path: "$.models.models" },
  ]);
});

test("remote model kinds reject all local model fields with kind-specific diagnostics", () => {
  assert.deepEqual(diagnostics({ kind: "codex-app-server", models: [], command: "tool", args: [] }), [
    { code: "invalid_capability_combination", message: "Field models is not valid for codex-app-server models.", path: "$.models.models" },
    { code: "invalid_capability_combination", message: "Field command is not valid for codex-app-server models.", path: "$.models.command" },
    { code: "invalid_capability_combination", message: "Field args is not valid for codex-app-server models.", path: "$.models.args" },
  ]);
  assert.deepEqual(diagnostics({ kind: "http", models: [], command: "tool", args: [] }), [
    { code: "invalid_capability_combination", message: "Field models is not valid for http models.", path: "$.models.models" },
    { code: "invalid_capability_combination", message: "Field command is not valid for http models.", path: "$.models.command" },
    { code: "invalid_capability_combination", message: "Field args is not valid for http models.", path: "$.models.args" },
  ]);
});

test("invalid model kinds stop before kind-specific field checks", () => {
  assert.deepEqual(diagnostics({ kind: 42, command: false }), [
    { code: "invalid_schema", message: "Expected a string.", path: "$.models.kind" },
  ]);
  assert.deepEqual(diagnostics({ kind: "unknown", command: false }), [
    { code: "invalid_schema", message: "Model kind must be static, command, codex-app-server, or http.", path: "$.models.kind" },
  ]);
  assert.deepEqual(diagnostics([]), [
    { code: "invalid_schema", message: "Expected a JSON object.", path: "$.models" },
  ]);
});
