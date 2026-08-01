import test from "node:test";
import assert from "node:assert/strict";
import { computeReplayScale } from "../src/terminalReplayScale.ts";

test("scales down when the panel is narrower than the recorded width", () => {
  const scale = computeReplayScale({ width: 1200, height: 400 }, { width: 600, height: 400 });
  assert.equal(scale, 0.5);
});

test("scales down when the panel is shorter than the recorded height", () => {
  const scale = computeReplayScale({ width: 1200, height: 400 }, { width: 1200, height: 200 });
  assert.equal(scale, 0.5);
});

test("uses the more constraining dimension when both are smaller", () => {
  const scale = computeReplayScale({ width: 1000, height: 500 }, { width: 400, height: 400 });
  assert.equal(scale, 0.4);
});

test("never scales up past 1 when the panel is larger than the recording", () => {
  const scale = computeReplayScale({ width: 800, height: 400 }, { width: 2000, height: 2000 });
  assert.equal(scale, 1);
});

test("returns 1 for a zero or negative natural size instead of dividing by zero", () => {
  assert.equal(computeReplayScale({ width: 0, height: 400 }, { width: 600, height: 400 }), 1);
  assert.equal(computeReplayScale({ width: 800, height: 0 }, { width: 600, height: 400 }), 1);
});
