import test from "node:test";
import assert from "node:assert/strict";
import { parseHarnessDraft } from "../src/harnessTypes.ts";

function duplicates(text: string) {
  return parseHarnessDraft(text, "override").diagnostics.filter((item) => item.code === "duplicate_key");
}

test("escaped key spellings report the decoded duplicate at its second source location", () => {
  const text = String.raw`{
  "name": "one",
  "\u006eame": "two"
}`;
  const result = parseHarnessDraft(text, "override");
  assert.deepEqual(result.value, { name: "two" });
  assert.deepEqual(result.diagnostics, [{
    code: "duplicate_key", message: "Duplicate object key name.", path: "$", line: 3, column: 3,
  }]);
});

test("escaped quotes, backslashes and prototype-named keys use decoded key equality", () => {
  for (const { text, key } of [
    { text: String.raw`{"a\"b":1,"a\u0022b":2}`, key: 'a"b' },
    { text: String.raw`{"a\\b":1,"a\u005cb":2}`, key: "a\\b" },
    { text: String.raw`{"__proto__":1,"\u005f_proto__":2}`, key: "__proto__" },
  ]) {
    assert.deepEqual(duplicates(text).map((item) => item.message), [`Duplicate object key ${key}.`]);
  }
});

test("the first nested duplicate wins before a later parent duplicate", () => {
  assert.deepEqual(duplicates('{ "a": [{ "b": 0, "b": 1 }], "a": 2 }'), [{
    code: "duplicate_key", message: "Duplicate object key b.", path: "$", line: 1, column: 19,
  }]);
});

test("sibling objects have independent keys and escaped value strings are skipped", () => {
  const text = JSON.stringify({
    payload: [{ name: "a" }, [null, false, -1.5e20, { name: "b" }], {}, []],
    name: 'quoted "name": { [ \\ \n content',
    constructor: 1,
    toString: 2,
  });
  assert.deepEqual(duplicates(text), []);
  assert.deepEqual(duplicates('{"__proto__":1,"constructor":2,"toString":3}'), []);
});

test("invalid JSON is rejected before duplicate scanning even with repeated keys", () => {
  for (const text of [
    '{"name":"a","name":"b",}',
    '{"name":"a","name":"b"} trailing',
    '{"name":"a","name":"b","payload":[1 2]}',
    '{"name":"a","name":"b","payload":"unterminated}',
  ]) {
    const result = parseHarnessDraft(text, "override");
    assert.equal(result.value, null);
    assert.deepEqual(result.diagnostics.map((item) => item.code), ["invalid_json"]);
  }
});
