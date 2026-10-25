import test from "node:test";
import assert from "node:assert/strict";
import { createSectionSaveScope } from "../src/settingsController.ts";

test("section saves commit in request order even when replies arrive backwards", () => {
  const scope = createSectionSaveScope();
  const first = scope.begin(), second = scope.begin();
  const committed: string[] = [];
  assert.equal(first.isCurrent(), false);
  assert.equal(second.isCurrent(), true);
  second.commit(() => committed.push("second"));
  first.commit(() => committed.push("first"));
  assert.deepEqual(committed, ["second"]);
});

test("an earlier success can commit while its successor is pending or fails", () => {
  const scope = createSectionSaveScope();
  const first = scope.begin(), second = scope.begin();
  const committed: string[] = [];
  first.commit(() => committed.push("first"));
  assert.equal(first.isCurrent(), false);
  assert.equal(second.isCurrent(), true);
  assert.deepEqual(committed, ["first"]);
});

test("close invalidation prevents both status and committed-state publication", () => {
  const scope = createSectionSaveScope();
  const old = scope.begin();
  scope.invalidate();
  let commits = 0;
  old.commit(() => commits++);
  assert.equal(old.isCurrent(), false);
  const reopened = scope.begin();
  reopened.commit(() => commits++);
  assert.equal(reopened.isCurrent(), true);
  assert.equal(commits, 1);
});

test("independent sections cannot supersede each other's saves", () => {
  const retention = createSectionSaveScope(), debug = createSectionSaveScope();
  const retentionSave = retention.begin(), debugSave = debug.begin();
  retention.invalidate();
  assert.equal(retentionSave.isCurrent(), false);
  assert.equal(debugSave.isCurrent(), true);
  let committed = false;
  debugSave.commit(() => { committed = true; });
  assert.equal(committed, true);
});
