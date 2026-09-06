import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

test("FixWithAiDialog is a confirm-modal prefilled from the caller's draft", () => {
  const source = readFileSync(new URL("../src/components/FixWithAiDialog.tsx", import.meta.url), "utf8");

  assert.match(source, /role="dialog"/);
  assert.match(source, /aria-modal="true"/);
  assert.match(source, /initialPrompt/);
  assert.match(source, /<textarea/);
  assert.match(source, /onConfirm\(prompt/);
  assert.match(source, />Cancel</);
  assert.match(source, /Send to session/);
  assert.match(source, /e\.key === "Escape"/);
});
