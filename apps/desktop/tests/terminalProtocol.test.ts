import test from "node:test";
import assert from "node:assert/strict";
import {
  parseTerminalControlMessage,
  shouldReplayTerminalOutputOnClose,
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
