import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

function source(path: string): string {
  return readFileSync(new URL(path, import.meta.url), "utf8");
}

test("Toggle exposes semantic visual state, tooltip, and accessible status description props", () => {
  const text = source("../src/components/Toggle.tsx");

  assert.match(text, /visualState\??:/);
  assert.match(text, /statusDescription\??:/);
  assert.match(text, /tooltip\??:/);
  assert.match(text, /statusGlyph\??:/);
  assert.match(text, /aria-describedby=/);
  assert.match(text, /title=\{tooltip/);
});

test("Toggle renders visible non-color status content without changing the stable switch name", () => {
  const text = source("../src/components/Toggle.tsx");

  assert.match(text, /aria-label=\{ariaLabel \?\? label \?\? undefined\}/);
  assert.match(text, /ui-toggle-status/);
  assert.match(text, /ui-toggle-status-glyph/);
  assert.match(text, /ui-toggle-status-text/);
});

test("Toggle CSS maps semantic states to shared tokens and keeps in-progress neutral", () => {
  const css = source("../src/App.css");
  const needsYouBlock = css.match(/\.ui-toggle--needs-you\s*\{[^}]+\}/);
  const healthyBlock = css.match(/\.ui-toggle--healthy\s*\{[^}]+\}/);
  const errorBlock = css.match(/\.ui-toggle--error\s*\{[^}]+\}/);
  const inProgressBlock = css.match(/\.ui-toggle--in-progress\s*\{[^}]+\}/);

  assert.ok(needsYouBlock, "expected needs-you toggle CSS block");
  assert.ok(healthyBlock, "expected healthy toggle CSS block");
  assert.ok(errorBlock, "expected error toggle CSS block");
  assert.ok(inProgressBlock, "expected in-progress toggle CSS block");

  assert.match(needsYouBlock[0], /var\(--attention-needs-you\)/);
  assert.match(healthyBlock[0], /var\(--state-ok\)/);
  assert.match(errorBlock[0], /var\(--state-error\)/);
  assert.match(inProgressBlock[0], /(var\(--surface-3\)|var\(--surface-2\))/);
  assert.doesNotMatch(inProgressBlock[0], /var\(--attention-needs-you\)/);
});

test("Toggle CSS uses the needs-you token for actionable warning status text", () => {
  const css = source("../src/App.css");
  const warningBlock = css.match(/\.ui-toggle-status--warning\s*\{[^}]+\}/);
  const trustBlock = css.match(/\.ui-toggle-status--trust\s*\{[^}]+\}/);

  assert.ok(warningBlock, "expected warning status CSS block");
  assert.ok(trustBlock, "expected trust status CSS block");

  assert.match(warningBlock[0], /var\(--attention-needs-you\)/);
  assert.doesNotMatch(warningBlock[0], /var\(--state-warn\)/);
  assert.match(trustBlock[0], /var\(--attention-needs-you\)/);
});
