import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

// StatusIndicator falls back to a plain colored dot for any tone missing
// from TONE_ICON — the same shape family "unread" uses (7px vs 8px circle,
// same color). For an icon-less tone that makes read and unread rows
// visually identical, so clearing unread looks like it "didn't work" even
// though the state cleared correctly. Every interruptive tone needs its own
// icon so the read state is never just a smaller unread dot.
test("every non-neutral, non-working attention tone has a distinct status icon", () => {
  const source = readFileSync(
    new URL("../src/components/StatusIndicator.tsx", import.meta.url),
    "utf8",
  );
  const block = source.match(/const TONE_ICON:[\s\S]*?=\s*\{([\s\S]*?)\n\};/)?.[1];
  assert.ok(block, "TONE_ICON object literal not found");

  for (const tone of ["needs-you", "blocked", "failed", "done", "idle"]) {
    assert.match(
      block!,
      new RegExp(`(^|\\s)"?${tone}"?\\s*:\\s*\\w+`),
      `TONE_ICON has no icon for "${tone}" — its read state will render as the same plain dot as unread`,
    );
  }
});
