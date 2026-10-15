import test from "node:test";
import assert from "node:assert/strict";
import { loadTerminalReplay, recordedReplaySize, writeTerminalReplay } from "../src/terminalReplay.ts";
import { computeTerminalInteractivity } from "../src/terminalPresentation.ts";

test("a starting session disables stdin and stops cursor blink so it reads as loading, not frozen", () => {
  assert.deepEqual(computeTerminalInteractivity({ starting: true, ended: false }), {
    disableStdin: true,
    cursorBlink: false,
  });
});

test("a running session (not starting, not ended) is fully interactive", () => {
  assert.deepEqual(computeTerminalInteractivity({ starting: false, ended: false }), {
    disableStdin: false,
    cursorBlink: true,
  });
});

test("an ended session stays non-interactive even if starting is stale-true", () => {
  assert.deepEqual(computeTerminalInteractivity({ starting: true, ended: true }), {
    disableStdin: true,
    cursorBlink: false,
  });
});

test("in-place replay uses the recorded grid when both cols and rows are present", () => {
  assert.deepEqual(recordedReplaySize({ cols: 120, rows: 40 }), { cols: 120, rows: 40 });
});

test("in-place replay falls back to live sizing when either recorded dimension is absent", () => {
  assert.equal(recordedReplaySize({}), null);
  assert.equal(recordedReplaySize({ cols: 120 }), null);
  assert.equal(recordedReplaySize({ rows: 40 }), null);
});

test("writes persisted replay when the request remains current", async () => {
  const written: string[] = [];
  let factories = 0;
  const result = await loadTerminalReplay(
    async () => ({ lines: ["first", "second"] }),
    () => true,
    () => {
      factories += 1;
      return {
        write: (text: string) => written.push(`write:${text}`),
        writeln: (line: string) => written.push(`writeln:${line}`),
      };
    },
  );

  assert.equal(result, "loaded");
  assert.equal(factories, 1);
  assert.deepEqual(written, ["writeln:first", "writeln:second"]);
});

test("writes raw replay records without adding a line ending", async () => {
  const written: string[] = [];
  const result = await loadTerminalReplay(
    async () => ({ lines: [{ text: "one", delimiter: "\r\n" }] }),
    () => true,
    () => ({
      write: (text: string) => written.push(`write:${text}`),
      writeln: (line: string) => written.push(`writeln:${line}`),
    }),
  );

  assert.equal(result, "loaded");
  assert.deepEqual(written, ["write:one\r\n"]);
});

test("replay fallback writes raw records and preserves legacy line behavior", () => {
  const written: string[] = [];

  writeTerminalReplay(
    {
      write: (text: string) => written.push(`write:${text}`),
      writeln: (line: string) => written.push(`writeln:${line}`),
    },
    [{ text: "one", delimiter: "\r\n" }, "one"],
  );

  assert.deepEqual(written, ["write:one\r\n", "writeln:one"]);
});

test("passes the recorded size through to the terminal factory", async () => {
  const sizes: Array<{ cols?: number; rows?: number }> = [];
  const result = await loadTerminalReplay(
    async () => ({ lines: ["one"], cols: 120, rows: 40 }),
    () => true,
    (size) => {
      sizes.push(size);
      return {
        write: () => {},
        writeln: () => {},
      };
    },
  );

  assert.equal(result, "loaded");
  assert.deepEqual(sizes, [{ cols: 120, rows: 40 }]);
});

test("does not write a replay response after selection changes", async () => {
  let resolve!: (payload: { lines: string[] }) => void;
  const pending = new Promise<{ lines: string[] }>((done) => { resolve = done; });
  const written: string[] = [];
  let factories = 0;
  let current = true;
  const result = loadTerminalReplay(() => pending, () => current, () => {
    factories += 1;
    return {
      write: (text: string) => written.push(`write:${text}`),
      writeln: (line: string) => written.push(`writeln:${line}`),
    };
  });

  current = false;
  resolve({ lines: ["stale"] });

  assert.equal(await result, "stale");
  assert.equal(factories, 0);
  assert.deepEqual(written, []);
});

test("reports empty and failed replay without writing", async () => {
  const written: string[] = [];
  let factories = 0;
  const create = () => {
    factories += 1;
    return {
      write: (text: string) => written.push(`write:${text}`),
      writeln: (line: string) => written.push(`writeln:${line}`),
    };
  };
  assert.equal(await loadTerminalReplay(async () => ({ lines: [] }), () => true, create), "empty");
  assert.equal(await loadTerminalReplay(async () => { throw new Error("offline"); }, () => true, create), "error");
  assert.equal(factories, 0);
  assert.deepEqual(written, []);
});
