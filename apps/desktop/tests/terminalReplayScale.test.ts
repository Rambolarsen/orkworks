import test from "node:test";
import assert from "node:assert/strict";
import { computeReplayScale, availableContentBox } from "../src/terminalReplayScale.ts";

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

test("availableContentBox subtracts per-side padding from the client size", () => {
  // Matches `.terminal-container { padding: var(--space-2); }` (8px on each side):
  // clientWidth/clientHeight include padding, but xterm lives inside the content box.
  const box = availableContentBox(
    { width: 800, height: 400 },
    { top: 8, right: 8, bottom: 8, left: 8 },
  );
  assert.deepEqual(box, { width: 784, height: 384 });
});

test("availableContentBox handles asymmetric padding", () => {
  const box = availableContentBox(
    { width: 1000, height: 600 },
    { top: 4, right: 12, bottom: 16, left: 8 },
  );
  assert.deepEqual(box, { width: 980, height: 580 });
});

test("availableContentBox with zero padding returns the client size unchanged", () => {
  const box = availableContentBox(
    { width: 600, height: 400 },
    { top: 0, right: 0, bottom: 0, left: 0 },
  );
  assert.deepEqual(box, { width: 600, height: 400 });
});

test("availableContentBox clamps oversized padding to zero instead of going negative", () => {
  const box = availableContentBox(
    { width: 10, height: 10 },
    { top: 100, right: 100, bottom: 100, left: 100 },
  );
  assert.deepEqual(box, { width: 0, height: 0 });
});
