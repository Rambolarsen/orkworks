import test from "node:test";
import assert from "node:assert/strict";
import xterm from "@xterm/xterm";
import { createTerminalPlanLinkProvider, terminalLinkHandler, terminalPlanPaths } from "../src/terminalLinks.ts";

const { Terminal } = xterm;

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

test("recognizes supported Windows paths and spaces", () => {
  assert.deepEqual(
    terminalPlanPaths("Wrote C:\\repo folder\\specs\\my plan.md and specs/a plan.md"),
    ["C:\\repo folder\\specs\\my plan.md", "specs/a plan.md"],
  );
});

test("does not recognize generic Markdown paths", () => {
  assert.deepEqual(terminalPlanPaths("See docs/readme.md and notes/plan.md"), []);
});

test("provides a multiline link from xterm's wrapped buffer", async () => {
  const terminal = new Terminal({ cols: 12, rows: 4 });
  await new Promise<void>((resolve) => terminal.write("specs/wrapped-plan.md", resolve));
  const provider = createTerminalPlanLinkProvider(terminal, async () => {});
  const links = await new Promise<any>((resolve) => {
    provider.provideLinks(2, resolve);
  });
  assert.equal(links?.[0]?.text, "specs/wrapped-plan.md");
  assert.notEqual(links?.[0]?.range.start.y, links?.[0]?.range.end.y);
  terminal.dispose();
});

test("uses xterm cell widths for a path after a wide character", async () => {
  const terminal = new Terminal({ cols: 80, rows: 2 });
  await new Promise<void>((resolve) => terminal.write("界 specs/plan.md", resolve));
  const provider = createTerminalPlanLinkProvider(terminal, async () => {});
  const links = await new Promise<any>((resolve) => provider.provideLinks(1, resolve));
  const wideCellWidth = terminal.buffer.active.getLine(0)?.getCell(0)?.getWidth() ?? 1;
  assert.equal(links?.[0]?.range.start.x, wideCellWidth + 2);
  terminal.dispose();
});
