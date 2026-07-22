import test from "node:test";
import assert from "node:assert/strict";
import { loadTerminalReplay } from "../src/terminalReplay.ts";
import { renderTerminalPresentation } from "../src/terminalPresentation.ts";

test("dead session routing invokes replay instead of interactive terminal creation", () => {
  let interactive = 0;
  let historical = 0;
  const result = renderTerminalPresentation(
    "dead",
    () => { interactive += 1; return "interactive"; },
    () => { historical += 1; return "historical"; },
  );

  assert.equal(result, "historical");
  assert.equal(interactive, 0);
  assert.equal(historical, 1);
});

for (const lifecycle of ["creating", "alive", "stopping"] as const) {
  test(`${lifecycle} session routing retains interactive terminal creation`, () => {
    let interactive = 0;
    let historical = 0;
    const result = renderTerminalPresentation(
      lifecycle,
      () => { interactive += 1; return "interactive"; },
      () => { historical += 1; return "historical"; },
    );

    assert.equal(result, "interactive");
    assert.equal(interactive, 1);
    assert.equal(historical, 0);
  });
}

test("writes persisted replay when the request remains current", async () => {
  const written: string[] = [];
  let factories = 0;
  const result = await loadTerminalReplay(
    async () => ["first", "second"],
    () => true,
    () => {
      factories += 1;
      return { writeln: (line: string) => written.push(line) };
    },
  );

  assert.equal(result, "loaded");
  assert.equal(factories, 1);
  assert.deepEqual(written, ["first", "second"]);
});

test("does not write a replay response after selection changes", async () => {
  let resolve!: (lines: string[]) => void;
  const pending = new Promise<string[]>((done) => { resolve = done; });
  const written: string[] = [];
  let factories = 0;
  let current = true;
  const result = loadTerminalReplay(() => pending, () => current, () => {
    factories += 1;
    return { writeln: (line: string) => written.push(line) };
  });

  current = false;
  resolve(["stale"]);

  assert.equal(await result, "stale");
  assert.equal(factories, 0);
  assert.deepEqual(written, []);
});

test("reports empty and failed replay without writing", async () => {
  const written: string[] = [];
  let factories = 0;
  const create = () => {
    factories += 1;
    return { writeln: (line: string) => written.push(line) };
  };
  assert.equal(await loadTerminalReplay(async () => [], () => true, create), "empty");
  assert.equal(await loadTerminalReplay(async () => { throw new Error("offline"); }, () => true, create), "error");
  assert.equal(factories, 0);
  assert.deepEqual(written, []);
});
