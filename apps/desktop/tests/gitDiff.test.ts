import test from "node:test";
import assert from "node:assert/strict";
import { parseGitPatch, splitDiffRows, tokenizeDiffLine } from "../src/gitDiff.ts";

test("patch line numbers follow independent hunk offsets and never count metadata", () => {
  const rows = parseGitPatch('diff --git a/a.ts b/a.ts\n--- a/a.ts\n+++ b/a.ts\n@@ -8,2 +12,3 @@ fn\n same\n-old\n+new\n+extra\n\\ No newline at end of file\n@@ -30 +35 @@\n-x\n+y\n');
  assert.deepEqual(rows.slice(4,8).map(r => [r.kind,r.text,r.oldLine,r.newLine]), [
    ["context","same",8,12], ["deletion","old",9,null],
    ["addition","new",null,13], ["addition","extra",null,14],
  ]);
  assert.equal(rows[2].kind, "metadata");
  assert.equal(rows[8].kind, "metadata");
  assert.deepEqual(rows.at(-1), {kind:"addition",text:"y",oldLine:null,newLine:35});
});

test("split diffs align replacements and pad unmatched lines without dropping hunk or newline markers", () => {
  const rows = splitDiffRows(parseGitPatch('@@ -1,3 +1,4 @@\n keep\n-old\n+new\n+extra\n\\ No newline at end of file\n tail'));
  assert.equal(rows[0].full?.kind, "hunk");
  assert.equal(rows[1].left?.text, "keep");
  assert.equal(rows[1].right?.text, "keep");
  assert.equal(rows[2].left?.text, "old");
  assert.equal(rows[2].right?.text, "new");
  assert.equal(rows[3].left, null);
  assert.equal(rows[3].right?.text, "extra");
  assert.equal(rows[3].rightNote, "\\ No newline at end of file");
  assert.equal(rows[4].right?.newLine, 4);
});

for (const both of [false, true]) test(`split replacements preserve ${both ? "both" : "old"}-side missing-newline markers without breaking alignment`, () => {
  const rows = splitDiffRows(parseGitPatch(`@@ -1 +1 @@\n-old\n\\ No newline at end of file\n+new\n${both ? "\\ No newline at end of file\n" : ""}`));
  assert.equal(rows.length, 2);
  assert.equal(rows[1].left?.text, "old");
  assert.equal(rows[1].right?.text, "new");
  assert.equal(rows[1].leftNote, "\\ No newline at end of file");
  assert.equal(rows[1].rightNote, both ? "\\ No newline at end of file" : undefined);
});

test("empty, CRLF, binary and malformed patches remain readable without invented line numbers", () => {
  assert.deepEqual(parseGitPatch(""), []);
  assert.deepEqual(parseGitPatch("Binary files differ\r\n"), [{kind:"metadata",text:"Binary files differ",oldLine:null,newLine:null}]);
  assert.deepEqual(parseGitPatch("@@ unknown\n+<script>alert(1)</script>").at(-1), {kind:"addition",text:"<script>alert(1)</script>",oldLine:null,newLine:null});
});

test("source lines beginning with header-like plus and minus signs remain changes inside a hunk", () => {
  const rows = parseGitPatch('--- a/x\n+++ b/x\n@@ -1 +1 @@\n---old\n+++new\n');
  assert.deepEqual(rows.slice(3), [
    {kind:"deletion",text:"--old",oldLine:1,newLine:null},
    {kind:"addition",text:"++new",oldLine:null,newLine:1},
  ]);
});

test("basic token coloring preserves source exactly and keeps keywords inside strings and comments intact", () => {
  const input = 'const value = "return <script>"; // false 12';
  const tokens = tokenizeDiffLine(input);
  assert.equal(tokens.map(t => t.text).join(""), input);
  assert.deepEqual(tokens.filter(t => t.kind !== "plain"), [
    {kind:"keyword",text:"const"}, {kind:"string",text:'"return <script>"'}, {kind:"comment",text:"// false 12"},
  ]);
});
