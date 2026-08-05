import test from "node:test";
import assert from "node:assert/strict";
import { captureRendererHealth } from "../src/rendererHealthProbe.ts";

function makeDeps({
  liveTerminalCount = 0,
  liveTerminalIds = [] as readonly string[],
  panelCount = 0,
} = {}) {
  return {
    panelCountProvider: () => panelCount,
    liveTerminalCountProvider: () => liveTerminalCount,
    liveTerminalIdsProvider: () => liveTerminalIds,
  };
}

test("captureRendererHealth reports live-terminal count and ids from the providers", () => {
  const sample = captureRendererHealth(makeDeps({ liveTerminalCount: 2, liveTerminalIds: ["a", "b"], panelCount: 3 }));
  assert.equal(sample.liveTerminalCount, 2);
  assert.deepEqual([...sample.liveTerminalIds], ["a", "b"]);
});

test("captureRendererHealth reports zero historical-terminal xterm nodes when document has none", () => {
  const sample = captureRendererHealth(makeDeps());
  assert.equal(sample.historicalTerminalNodeCount, 0);
});

test("captureRendererHealth reports performance.memory when available", () => {
  const original = (globalThis as any).performance;
  (globalThis as any).performance = {
    ...original,
    memory: { usedJSHeapSize: 1000, totalJSHeapSize: 2000, jsHeapSizeLimit: 4000 },
  };
  try {
    const sample = captureRendererHealth(makeDeps());
    assert.equal(sample.usedJSHeapSize, 1000);
    assert.equal(sample.totalJSHeapSize, 2000);
    assert.equal(sample.jsHeapSizeLimit, 4000);
  } finally {
    (globalThis as any).performance = original;
  }
});

test("captureRendererHealth omits memory fields when performance.memory is unavailable", () => {
  const original = (globalThis as any).performance;
  (globalThis as any).performance = { ...original, memory: undefined };
  try {
    const sample = captureRendererHealth(makeDeps());
    assert.equal(sample.usedJSHeapSize, undefined);
    assert.equal(sample.totalJSHeapSize, undefined);
    assert.equal(sample.jsHeapSizeLimit, undefined);
  } finally {
    (globalThis as any).performance = original;
  }
});

test("captureRendererHealth counts `.xterm` nodes inside `.terminal-shell` containers", () => {
  const original = (globalThis as any).document;
  const shell = {
    querySelectorAll: (sel: string) =>
      sel === ".xterm" ? [{}, {}] : [],
  };
  (globalThis as any).document = {
    ...original,
    querySelectorAll: (sel: string) => (sel === ".terminal-shell" ? [shell] : []),
  };
  try {
    const sample = captureRendererHealth(makeDeps());
    assert.equal(sample.historicalTerminalNodeCount, 2);
  } finally {
    (globalThis as any).document = original;
  }
});

test("captureRendererHealth surfaces the dockview panel count", () => {
  const sample = captureRendererHealth(makeDeps({ panelCount: 4 }));
  assert.equal(sample.panelCount, 4);
});