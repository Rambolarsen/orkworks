import test from "node:test";
import assert from "node:assert/strict";
import xterm from "@xterm/xterm";
import { dismissToast, subscribeToasts } from "../src/feedback.ts";
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

test("recognizes quoted absolute paths and normalizes Windows separator padding", () => {
  assert.deepEqual(
    terminalPlanPaths("Wrote `/Users/me/repo/docs/superpowers/plans/review.md` and C:\\repo\\docs\\ superpowers\\ specs\\plan.md"),
    [
      "/Users/me/repo/docs/superpowers/plans/review.md",
      "C:\\repo\\docs\\superpowers\\specs\\plan.md",
    ],
  );
});

test("does not recognize generic Markdown paths", () => {
  assert.deepEqual(terminalPlanPaths("See docs/readme.md and notes/plan.md"), []);
});

test("does not recognize plan-looking paths outside supported roots", () => {
  for (const path of ["docs/plans/plan.md", "docs/specs/spec.md", "/repo/docs/plans/other.md"]) {
    assert.deepEqual(terminalPlanPaths(path), [], path);
  }
});

test("provides a multiline link from xterm's wrapped buffer", async () => {
  const terminal = new Terminal({ cols: 12, rows: 4 });
  await new Promise<void>((resolve) => terminal.write("specs/wrapped-plan.md", resolve));
  const provider = createTerminalPlanLinkProvider(terminal, async () => {});
  const links = await new Promise<any>((resolve) => {
    provider.provideLinks(2, resolve);
  });
  assert.equal(links?.[0]?.text, "specs/wrapped-plan.md");
  assert.equal(links?.[0]?.range.start.y, 2);
  assert.equal(links?.[0]?.range.end.y, 2);
  terminal.dispose();
});

test("exposes each wrapped absolute-path row as a clickable single-row range", async () => {
  const terminal = new Terminal({ cols: 80, rows: 8 });
  const expected = "/Users/froomiebot/workspace/orkworks-provider-model-selection/docs/superpowers/specs/2026-08-25-provider-model-selection-design.md";
  await new Promise<void>((resolve) => terminal.write(
    "• The provider-scoped model design is written and committed at " + expected + ".",
    resolve,
  ));
  const activated: string[] = [];
  const provider = createTerminalPlanLinkProvider(terminal, async (path) => { activated.push(path); });

  for (const row of [1, 2, 3]) {
    const links = await new Promise<any>((resolve) => provider.provideLinks(row, resolve));
    assert.equal(links?.length, 1);
    assert.equal(links[0].text, expected);
    assert.equal(links[0].range.start.y, row);
    assert.equal(links[0].range.end.y, row);
    links[0].activate();
  }

  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(activated, [expected, expected, expected]);
  terminal.dispose();
});

test("keeps an absolute plan link across a hard terminal line break", async () => {
  const terminal = new Terminal({ cols: 120, rows: 4 });
  await new Promise<void>((resolve) => terminal.write(
    "/Users/froomiebot/workspace/orkworks-harness-version-status/docs/\n"
      + "superpowers/specs/2026-08-25-harness-version-status-design.md",
    resolve,
  ));
  const provider = createTerminalPlanLinkProvider(terminal, async () => {});
  const expected = "/Users/froomiebot/workspace/orkworks-harness-version-status/docs/superpowers/specs/2026-08-25-harness-version-status-design.md";
  for (const row of [1, 2]) {
    const links = await new Promise<any>((resolve) => provider.provideLinks(row, resolve));
    assert.equal(links?.[0]?.text, expected);
  }
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

test("caps the wrapped-chain scan instead of walking an entire oversized buffer", async () => {
  // Legacy `.terminal` replay files predate the 1,000-line/1 MiB retention cap
  // and can still be tens of megabytes of unbroken output. A plan path lying
  // beyond the scan cap must not be found (a bounded, cheap miss), rather
  // than the provider re-walking the whole wrapped chain on every call.
  const terminal = new Terminal({ cols: 2, rows: 4 });
  const filler = "xy".repeat(210); // wraps into 210 rows at cols=2
  await new Promise<void>((resolve) => terminal.write(filler + "specs/plan.md", resolve));
  const provider = createTerminalPlanLinkProvider(terminal, async () => {});
  const links = await new Promise<any>((resolve) => provider.provideLinks(1, resolve));
  assert.equal(links, undefined);
  terminal.dispose();
});

test("still finds a wrapped link that stays within the scan cap", async () => {
  const terminal = new Terminal({ cols: 2, rows: 4 });
  const filler = "xy".repeat(50); // wraps into 50 rows at cols=2, well under the cap
  await new Promise<void>((resolve) => terminal.write(filler + "specs/plan.md", resolve));
  const provider = createTerminalPlanLinkProvider(terminal, async () => {});
  const links = await new Promise<any>((resolve) => provider.provideLinks(51, resolve));
  assert.equal(links?.[0]?.text, "specs/plan.md");
  terminal.dispose();
});

test("shows a visible error when selecting a terminal plan fails", async () => {
  const terminal = new Terminal({ cols: 80, rows: 2 });
  await new Promise<void>((resolve) => terminal.write("specs/plan.md", resolve));
  const provider = createTerminalPlanLinkProvider(terminal, async () => { throw new Error("Couldn't open this plan."); });
  const links = await new Promise<any>((resolve) => provider.provideLinks(1, resolve));
  let current: readonly { id: string; message: string }[] = [];
  const unsubscribe = subscribeToasts((toasts) => { current = toasts; });
  links[0].activate();
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(current.at(-1)?.message, "Couldn't open this plan.");
  for (const toast of current) dismissToast(toast.id);
  unsubscribe();
  terminal.dispose();
});
