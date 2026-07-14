import test from "node:test";
import assert from "node:assert/strict";
import {
  parseTerminalControlMessage,
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
