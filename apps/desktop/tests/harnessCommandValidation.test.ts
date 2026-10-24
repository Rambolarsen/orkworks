import test from "node:test";
import assert from "node:assert/strict";
import { parseHarnessDraft } from "../src/harnessTypes.ts";

const contexts = [
  { name: "sparse launch", wrap: (fields: object) => ({ launch: fields }), path: "$.launch", emptyCode: "invalid_command" },
  { name: "command-template launch", wrap: (fields: object) => ({ launch: { kind: "command-template", ...fields } }), path: "$.launch", emptyCode: "invalid_command" },
  { name: "resume", wrap: (fields: object) => ({ resume: { exact: fields } }), path: "$.resume.exact", emptyCode: "invalid_resume_command" },
  { name: "model discovery", wrap: (fields: object) => ({ models: { kind: "command", ...fields } }), path: "$.models", emptyCode: "invalid_model_command" },
];

for (const { name, wrap, path, emptyCode } of contexts) {
  test(`${name} validates command type, emptiness, placeholders and arguments`, () => {
    const diagnostics = (fields: object) => parseHarnessDraft(JSON.stringify(wrap(fields)), "override")
      .diagnostics.map(({ code, path }) => ({ code, path }));
    assert.deepEqual(diagnostics({ command: "tool {model} {effort} {cwd} {repoRoot} {harnessSessionId}", args: [] }), []);
    assert.deepEqual(diagnostics({ command: 42, args: [false] }), [
      { code: "invalid_schema", path: `${path}.command` },
      { code: "invalid_schema", path: `${path}.args` },
    ]);
    assert.deepEqual(diagnostics({ command: " \t", args: [] }), [{ code: emptyCode, path: `${path}.command` }]);
    for (const command of ["tool {unknown}", "tool {model", "tool }"]) {
      assert.deepEqual(diagnostics({ command, args: [] }), [{ code: "invalid_placeholder", path: `${path}.command` }]);
    }
  });
}

test("sparse launch patches keep omitted command fields optional while complete command capabilities require them", () => {
  assert.deepEqual(parseHarnessDraft('{"launch":{"login":false,"modelPrefix":null}}', "override").diagnostics, []);
  assert.deepEqual(parseHarnessDraft('{"launch":{"kind":"command-template"}}', "override").diagnostics, []);
  const complete = parseHarnessDraft('{"id":"tool","name":"Tool","launch":{"kind":"command-template"},"labelResetCommands":[]}', "create");
  assert.deepEqual(complete.diagnostics.map(({ code, path }) => ({ code, path })), [
    { code: "missing_field", path: "$.launch.command" },
    { code: "missing_field", path: "$.launch.args" },
  ]);
  const nested = parseHarnessDraft('{"resume":{"exact":{}},"models":{"kind":"command"}}', "override");
  assert.deepEqual(nested.diagnostics.map(({ code, path }) => ({ code, path })), [
    { code: "missing_field", path: "$.resume.exact.command" },
    { code: "missing_field", path: "$.resume.exact.args" },
    { code: "missing_field", path: "$.models.command" },
    { code: "missing_field", path: "$.models.args" },
  ]);
});

test("launch validation keeps platform-shell fields separate from command-template fields", () => {
  const shell = parseHarnessDraft('{"launch":{"kind":"platform-shell","login":true,"command":42,"args":false,"modelPrefix":null}}', "override");
  assert.deepEqual(shell.diagnostics.map(({ code, path }) => ({ code, path })), [
    { code: "invalid_capability_combination", path: "$.launch.command" },
    { code: "invalid_capability_combination", path: "$.launch.args" },
    { code: "invalid_capability_combination", path: "$.launch.modelPrefix" },
  ]);
  const command = parseHarnessDraft('{"launch":{"kind":"command-template","command":" ","args":false,"modelPrefix":42,"login":false}}', "override");
  assert.deepEqual(command.diagnostics.map(({ code, path }) => ({ code, path })), [
    { code: "invalid_command", path: "$.launch.command" },
    { code: "invalid_schema", path: "$.launch.args" },
    { code: "invalid_schema", path: "$.launch.modelPrefix" },
    { code: "invalid_capability_combination", path: "$.launch.login" },
  ]);
  const invalidKind = parseHarnessDraft('{"launch":{"kind":"unknown","command":42}}', "override");
  assert.deepEqual(invalidKind.diagnostics.map(({ code, path }) => ({ code, path })), [
    { code: "invalid_schema", path: "$.launch.kind" },
  ]);
});
