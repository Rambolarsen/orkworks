import test from "node:test";
import assert from "node:assert/strict";
import { createTrackpadWheelFilter } from "../src/terminalWheel.ts";

const event = (deltaY: number, timeStamp: number, extra: Partial<WheelEvent> = {}) => ({
  deltaY, timeStamp, deltaMode: 0, altKey: false, ctrlKey: false,
  shiftKey: false, metaKey: false, ...extra,
});

test("a 50px trackpad burst produces three scroll commands instead of fifty", () => {
  const wheel = createTrackpadWheelFilter();
  let forwarded = 0;
  for (let i = 0; i < 50; i++) {
    if (wheel.filter(event(1, i * 10), true, 20)) forwarded++;
  }
  assert.equal(forwarded, 3);
});

test("tiny gestures, reversal and a 150ms pause respond immediately", () => {
  const wheel = createTrackpadWheelFilter();
  assert.equal(wheel.filter(event(0.25, 0), true, 20), true);
  assert.equal(wheel.filter(event(0.25, 10), true, 20), false);
  assert.equal(wheel.filter(event(-0.25, 20), true, 20), true);
  assert.equal(wheel.filter(event(-0.25, 30), true, 20), false);
  assert.equal(wheel.filter(event(-0.25, 180), true, 20), true);
});

test("normal-buffer and line/page-mode input pass through and clear residual pixels", () => {
  const wheel = createTrackpadWheelFilter();
  assert.equal(wheel.filter(event(1, 0), true, 20), true);
  assert.equal(wheel.filter(event(1, 10), true, 20), false);
  assert.equal(wheel.filter(event(1, 20), false, 20), true);
  assert.equal(wheel.filter(event(1, 30), true, 20), true);
  for (const deltaMode of [1, 2]) {
    assert.equal(wheel.filter(event(1, 40, { deltaMode }), true, 20), true);
    assert.equal(wheel.filter(event(1, 50, { deltaMode }), true, 20), true);
  }
  assert.equal(wheel.filter(event(1, 60), true, 20), true);
});

test("zero vertical input does not interrupt a trackpad gesture", () => {
  const wheel = createTrackpadWheelFilter();
  assert.equal(wheel.filter(event(1, 0), true, 20), true);
  assert.equal(wheel.filter(event(0, 10), true, 20), true);
  assert.equal(wheel.filter(event(1, 20), true, 20), false);
});

test("modifier changes start a responsive gesture and retain burst filtering", () => {
  for (const modifier of ["altKey", "ctrlKey", "shiftKey", "metaKey"]) {
    const wheel = createTrackpadWheelFilter();
    assert.equal(wheel.filter(event(1, 0), true, 20), true);
    const extra = { [modifier]: true };
    assert.equal(wheel.filter(event(1, 10, extra), true, 20), true);
    assert.equal(wheel.filter(event(1, 20, extra), true, 20), false);
  }
});

test("large deltas retain only a fractional remainder and never flood later input", () => {
  const wheel = createTrackpadWheelFilter();
  assert.equal(wheel.filter(event(120, 0), true, 20), true);
  assert.equal(wheel.filter(event(121, 10), true, 20), true);
  assert.equal(wheel.filter(event(1, 20), true, 20), false);
});

test("terminal accumulators and buffer-transition resets are independent", () => {
  const first = createTrackpadWheelFilter();
  const second = createTrackpadWheelFilter();
  assert.equal(first.filter(event(1, 0), true, 20), true);
  assert.equal(first.filter(event(1, 10), true, 20), false);
  assert.equal(second.filter(event(1, 10), true, 20), true);
  first.reset();
  assert.equal(first.filter(event(1, 20), true, 20), true);
});
