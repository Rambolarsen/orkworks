import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const xtermSource = readFileSync(
  new URL("../node_modules/@xterm/xterm/src/browser/CoreBrowserTerminal.ts", import.meta.url),
  "utf8",
);
const xtermEsm = readFileSync(
  new URL("../node_modules/@xterm/xterm/lib/xterm.mjs", import.meta.url),
  "utf8",
);
const xtermCommonJs = readFileSync(
  new URL("../node_modules/@xterm/xterm/lib/xterm.js", import.meta.url),
  "utf8",
);

test("xterm forwards nonzero wheel events through mouse tracking in the alternate buffer", () => {
  const mouseReportPath = xtermSource.match(/case 'wheel':([\s\S]*?)\n\s*default:/)?.[1] ?? "";

  assert.match(mouseReportPath, /consumeWheelEvent\(/);
  assert.doesNotMatch(mouseReportPath, /if\s*\(\s*lines\s*===\s*0\s*\)/);
  assert.match(mouseReportPath, /if\s*\(\s*deltaY\s*===\s*0\s*\)/);
  assert.match(mouseReportPath, /action = deltaY < 0 \? CoreMouseAction\.UP : CoreMouseAction\.DOWN;/);
  assert.match(mouseReportPath, /but = CoreMouseButton\.WHEEL;/);
});

test("xterm forwards nonzero passive wheel events in the alternate buffer", () => {
  const passiveWheelPath = xtermSource.match(
    /addDisposableListener\(el, 'wheel', \(ev: WheelEvent\) => \{([\s\S]*?)\n\s*\}, \{ passive: false \}\)\);/,
  )?.[1] ?? "";

  assert.match(passiveWheelPath, /if \(!this\.buffer\.hasScrollback\)/);
  assert.match(passiveWheelPath, /consumeWheelEvent\(/);
  assert.doesNotMatch(passiveWheelPath, /if\s*\(\s*lines\s*===\s*0\s*\)/);
  assert.match(passiveWheelPath, /if\s*\(\s*deltaY\s*===\s*0\s*\)/);
  assert.match(passiveWheelPath, /triggerDataEvent\(sequence, true\)/);
});

for (const [format, bundle] of [["ESM", xtermEsm], ["CommonJS", xtermCommonJs]] as const) {
  test(`xterm's shipped ${format} bundle keeps both alternate-buffer wheel paths`, () => {
    const mouseReportPath = bundle.match(/case"wheel":([\s\S]*?)default:/)?.[1] ?? "";
    const passiveStart = bundle.indexOf("if(!this.buffer.hasScrollback)");
    const passiveWheelPath = passiveStart < 0 ? "" : bundle.slice(passiveStart, passiveStart + 500);

    assert.match(mouseReportPath, /consumeWheelEvent\(/);
    assert.doesNotMatch(mouseReportPath, /consumeWheelEvent\([^;]*\)===0/);
    assert.match(mouseReportPath, /deltaY/);
    assert.match(mouseReportPath, /(?:deltaY|[a-z])===0|0===(?:[a-z])/);
    assert.match(passiveWheelPath, /consumeWheelEvent\(/);
    assert.doesNotMatch(passiveWheelPath, /consumeWheelEvent\([^;]*\)===0/);
    assert.match(passiveWheelPath, /(?:deltaY|[a-z])===0|0===(?:[a-z])/);
    assert.match(passiveWheelPath, /triggerDataEvent\(/);
  });
}
