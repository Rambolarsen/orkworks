import test from "node:test";
import assert from "node:assert/strict";
import {
  parseTerminalControlMessage,
  formatTerminalOutputGapNotice,
  shouldReplayTerminalOutputOnClose,
  appendPendingInput,
  canSendTerminalInput,
} from "../src/terminalProtocol.ts";

test("parseTerminalControlMessage parses typed ended frames", () => {
  assert.deepEqual(
    parseTerminalControlMessage('{"type":"ended","status":"killed"}'),
    { type: "ended", status: "killed" },
  );
});

test("parseTerminalControlMessage parses typed error frames", () => {
  assert.deepEqual(
    parseTerminalControlMessage('{"type":"error","code":"pty_wait_failed","message":"boom"}'),
    { type: "error", code: "pty_wait_failed", message: "boom" },
  );
});

test("parseTerminalControlMessage ignores malformed payloads", () => {
  assert.equal(parseTerminalControlMessage("not json"), null);
  assert.equal(parseTerminalControlMessage('{"type":"ended"}'), null);
});

test("parseTerminalControlMessage parses typed input-dropped frames", () => {
  assert.deepEqual(
    parseTerminalControlMessage('{"type":"input-dropped"}'),
    { type: "input-dropped" },
  );
});

test("parseTerminalControlMessage parses typed output-gap frames and rejects malformed ones", () => {
  assert.deepEqual(
    parseTerminalControlMessage(
      '{"type":"output-gap","fromCursor":2,"toCursor":5,"missedChunks":3}',
    ),
    { type: "output-gap", fromCursor: 2, toCursor: 5, missedChunks: 3 },
  );
  assert.equal(
    parseTerminalControlMessage(
      '{"type":"output-gap","fromCursor":2,"toCursor":2,"missedChunks":0}',
    ),
    null,
  );
  assert.equal(
    parseTerminalControlMessage(
      '{"type":"output-gap","fromCursor":5,"toCursor":2,"missedChunks":3}',
    ),
    null,
  );
  assert.equal(
    parseTerminalControlMessage(
      '{"type":"output-gap","fromCursor":1,"toCursor":3,"missedChunks":1.5}',
    ),
    null,
  );
  assert.equal(
    parseTerminalControlMessage(
      '{"type":"output-gap","fromCursor":1,"toCursor":5,"missedChunks":3}',
    ),
    null,
  );
  assert.equal(
    parseTerminalControlMessage('{"type":"output-gap","missedChunks":2}'),
    null,
  );
});

test("formatTerminalOutputGapNotice formats singular and plural missed-chunk notices", () => {
  assert.equal(
    formatTerminalOutputGapNotice({
      type: "output-gap",
      fromCursor: 1,
      toCursor: 2,
      missedChunks: 1,
    }),
    "\r\n[terminal output gap: 1 chunk of live output could not be recovered]",
  );
  assert.equal(
    formatTerminalOutputGapNotice({
      type: "output-gap",
      fromCursor: 2,
      toCursor: 7,
      missedChunks: 5,
    }),
    "\r\n[terminal output gap: 5 chunks of live output could not be recovered]",
  );
});

test("shouldReplayTerminalOutputOnClose skips intentional detach", () => {
  assert.equal(
    shouldReplayTerminalOutputOnClose({ disposed: true, receivedData: false }),
    false,
  );
  assert.equal(
    shouldReplayTerminalOutputOnClose({ disposed: false, receivedData: false }),
    true,
  );
  assert.equal(
    shouldReplayTerminalOutputOnClose({ disposed: false, receivedData: true }),
    false,
  );
});

test("appendPendingInput appends while under the cap", () => {
  assert.deepEqual(appendPendingInput("abc", "def", 10), {
    next: "abcdef",
    dropped: false,
  });
});

test("appendPendingInput accepts a chunk that exactly fills the cap", () => {
  assert.deepEqual(appendPendingInput("abcde", "fghij", 10), {
    next: "abcdefghij",
    dropped: false,
  });
});

test("appendPendingInput drops the incoming chunk once it would exceed the cap", () => {
  assert.deepEqual(appendPendingInput("abcdefghij", "k", 10), {
    next: "abcdefghij",
    dropped: true,
  });
});

test("appendPendingInput keeps reporting dropped on repeated overflow without growing", () => {
  const first = appendPendingInput("abcdefghij", "k", 10);
  const second = appendPendingInput(first.next, "lmno", 10);
  assert.deepEqual(second, { next: "abcdefghij", dropped: true });
});

test("canSendTerminalInput keeps the browser WebSocket buffer within the cap", () => {
  assert.equal(canSendTerminalInput(4, 6, 10), true);
  assert.equal(canSendTerminalInput(4, 7, 10), false);
});
