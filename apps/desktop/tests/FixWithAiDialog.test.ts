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

test("FixWithAiDialog traps Tab focus within the modal and focuses on mount", () => {
  // Mirrors NewSessionDialog: without this, opening the dialog leaves focus
  // on the covered "Fix with AI" button, and Tab can escape the aria-modal
  // overlay into controls behind it.
  const source = readFileSync(new URL("../src/components/FixWithAiDialog.tsx", import.meta.url), "utf8");

  assert.match(source, /\.focus\(\)/);
  assert.match(source, /e\.key === "Tab"/);
  assert.match(source, /querySelectorAll/);
});
