import assert from "node:assert/strict";
import test from "node:test";
import { parseHarnessDraft } from "../src/harnessTypes.ts";

test("JSON editor accepts independent inference definitions and explicit removal", () => {
  const inference = { kind: "command", command: "custom-infer", args: ["--model", "{model}"], input: "stdin", output: "result-json-v1" };
  for (const mode of ["create", "custom", "override"] as const) {
    const object = mode === "override" ? { inference } : {
      id: "custom-infer", name: "Custom", launch: { kind: "platform-shell", login: false }, labelResetCommands: [], inference,
    };
    const parsed = parseHarnessDraft(JSON.stringify(object), mode);
    assert.deepEqual(parsed.diagnostics, []);
    assert.deepEqual(parsed.value?.inference, inference);
  }
  assert.deepEqual(parseHarnessDraft('{"inference":null}', "override").diagnostics, []);
});

test("inference editor rejects invalid command contracts, never treating flags as trust", () => {
  const valid = { kind: "command", command: "custom-infer", args: ["--model", "{model}"], input: "stdin", output: "result-json-v1" };
  for (const patch of [
    { kind: "builtin" }, { trusted: true }, { command: "" }, { command: "./tool" },
    { command: "tool\n" }, { command: "{model}" }, { command: "é".repeat(2049) },
    { args: [] }, { args: ["{model}", "{model}"] }, { args: ["{model}", "{cwd}"] },
    { args: ["{model}", "{effort}"] }, { args: ["{model}", "{promptFile}"] },
    { args: ["{model}", "unmatched}"] }, { args: ["{model}", "{broken"] },
    { args: ["{model}", "é".repeat(2049)] }, { args: ["{model}", ...Array(64).fill("x")] },
    { input: "file" }, { input: "argument" }, { output: "text" },
    { timeoutSecs: 0 }, { timeoutSecs: 121 }, { timeoutSecs: null },
    { reasoningEffortArgs: [] }, { reasoningEffortArgs: null }, { reasoningEffortArgs: ["{model}"] },
    { reasoningEffortArgs: ["{effort}", "{effort}"] },
  ]) {
    const parsed = parseHarnessDraft(JSON.stringify({ inference: { ...valid, ...patch } }), "override");
    assert.ok(parsed.diagnostics.some(d => d.path?.startsWith("$.inference")), JSON.stringify(patch));
  }
  assert.ok(parseHarnessDraft('{"inference":{"timeoutSecs":30}}', "override").diagnostics.length);
  assert.ok(parseHarnessDraft('{"inference":{"kind":"command","kind":"command"}}', "override").diagnostics.some(d => d.code === "duplicate_key"));
});

test("inference editor preserves complete file and effort templates", () => {
  const inference = {
    kind: "command", command: "custom-infer", args: ["--model={model}", "--input={promptFile}", "literal;$value"],
    input: "file", output: "result-json-v1", timeoutSecs: 120, reasoningEffortArgs: ["--effort={effort}"],
  };
  const parsed = parseHarnessDraft(JSON.stringify({ inference }), "override");
  assert.deepEqual(parsed.diagnostics, []);
  assert.deepEqual(parsed.value?.inference, inference);
});
