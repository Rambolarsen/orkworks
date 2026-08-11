import test from "node:test";
import assert from "node:assert/strict";
import { terminalLinkHandler, terminalPlanPaths } from "../src/terminalLinks.ts";

test("forwards an activated terminal link to Electron", async () => {
  const opened: string[] = [];
  terminalLinkHandler(async (url) => { opened.push(url); }).activate(
    {} as MouseEvent,
    "https://example.test/docs",
    {} as never,
  );

  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(opened, ["https://example.test/docs"]);
});

test("recognizes relative and absolute supported plan paths", () => {
  assert.deepEqual(
    terminalPlanPaths("Wrote specs/a-plan.md and /Users/me/repo/docs/superpowers/plans/review.md"),
    ["specs/a-plan.md", "/Users/me/repo/docs/superpowers/plans/review.md"],
  );
});

test("does not recognize generic Markdown paths", () => {
  assert.deepEqual(terminalPlanPaths("See docs/readme.md and notes/plan.md"), []);
});
