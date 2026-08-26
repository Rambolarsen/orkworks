import assert from "node:assert/strict";
import test from "node:test";

import { recoveryDocumentUrl } from "../electron/rendererRecoveryDocument.ts";

test("recovery document is self-contained and retries the exact original URL", () => {
  const originalUrl = "file:///Applications/OrkWorks.app/Contents/Resources/dist/index.html";
  const recoveryUrl = recoveryDocumentUrl(originalUrl);
  const html = decodeURIComponent(recoveryUrl.replace("data:text/html;charset=utf-8,", ""));

  assert.equal((html.match(/<button/g) ?? []).length, 1);
  assert.match(html, /location\.replace\(originalUrl\)/);
  assert.match(html, /OrkWorks is unavailable/);
  assert.doesNotMatch(html, /<script[^>]+src=|<link[^>]+href=/);
  assert.match(html, /file:\/\/\/Applications\/OrkWorks\.app/);
});
