import assert from "node:assert/strict";
import test from "node:test";

import { createRecoveryDocumentGuard, isSupersededNavigation } from "../electron/rendererRecoveryState.ts";

test("treats ERR_ABORTED (-3) as a superseded navigation, not a load failure", () => {
  assert.equal(isSupersededNavigation(-3), true);
});

test("treats every other load error code as a genuine failure", () => {
  assert.equal(isSupersededNavigation(-2), false);
  assert.equal(isSupersededNavigation(-105), false);
  assert.equal(isSupersededNavigation(0), false);
});

test("permits recovery again when a second original-document load fails", () => {
  const originalUrl = "file:///Applications/OrkWorks.app/Contents/Resources/dist/index.html";
  const guard = createRecoveryDocumentGuard(originalUrl);

  assert.equal(guard.beginRecoveryDocumentLoad(), true);
  guard.beginOriginalDocumentNavigation(originalUrl);
  assert.equal(guard.beginRecoveryDocumentLoad(), true);
});

test("clears recovery state after the original document finishes loading", () => {
  const originalUrl = "http://localhost:5173/";
  const guard = createRecoveryDocumentGuard(originalUrl);

  assert.equal(guard.beginRecoveryDocumentLoad(), true);
  guard.finishOriginalDocumentLoad(originalUrl);
  assert.equal(guard.beginRecoveryDocumentLoad(), true);
});
