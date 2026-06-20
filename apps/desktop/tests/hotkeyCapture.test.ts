import test from "node:test";
import assert from "node:assert/strict";

import { acceleratorFromKeyboardEvent } from "../src/hotkeyCapture.ts";

function event(
  key: string,
  modifiers: Partial<Pick<KeyboardEvent, "metaKey" | "ctrlKey" | "altKey" | "shiftKey">> = {},
) {
  return {
    key,
    metaKey: false,
    ctrlKey: false,
    altKey: false,
    shiftKey: false,
    preventDefault() {},
    stopPropagation() {},
    ...modifiers,
  } as KeyboardEvent;
}

test("acceleratorFromKeyboardEvent normalizes command/control chords", () => {
  assert.equal(acceleratorFromKeyboardEvent(event("n", { metaKey: true })), "CmdOrCtrl+N");
  assert.equal(acceleratorFromKeyboardEvent(event("T", { ctrlKey: true, shiftKey: true })), "CmdOrCtrl+Shift+T");
});

test("acceleratorFromKeyboardEvent ignores modifier-only keydown events", () => {
  assert.equal(acceleratorFromKeyboardEvent(event("Shift", { shiftKey: true })), null);
  assert.equal(acceleratorFromKeyboardEvent(event("Control", { ctrlKey: true })), null);
});

test("acceleratorFromKeyboardEvent supports named keys", () => {
  assert.equal(acceleratorFromKeyboardEvent(event("Backspace", { metaKey: true, altKey: true })), "CmdOrCtrl+Alt+Backspace");
  assert.equal(acceleratorFromKeyboardEvent(event(" ", { ctrlKey: true })), "CmdOrCtrl+Space");
  assert.equal(acceleratorFromKeyboardEvent(event("ArrowLeft", { ctrlKey: true })), "CmdOrCtrl+Left");
});

test("acceleratorFromKeyboardEvent ignores unmodified ordinary keys", () => {
  assert.equal(acceleratorFromKeyboardEvent(event("n")), null);
  assert.equal(acceleratorFromKeyboardEvent(event("a")), null);
  assert.equal(acceleratorFromKeyboardEvent(event("F2")), "F2");
});
