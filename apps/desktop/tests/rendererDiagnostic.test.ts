import assert from "node:assert/strict";
import test from "node:test";

import {
  rendererConsoleDiagnostic,
  rendererOrigin,
  sanitizeRendererDiagnosticMessage,
} from "../electron/rendererDiagnostic.ts";

test("console diagnostics contain only allowlisted metadata", () => {
  const diagnostic = rendererConsoleDiagnostic(
    2,
    "https://example.test/?prompt=secret",
    41,
  );

  assert.deepEqual(diagnostic, {
    type: "console-message",
    level: 2,
    origin: "https://example.test",
    line: 41,
  });
  assert.equal("message" in diagnostic, false);
  assert.doesNotMatch(JSON.stringify(diagnostic), /prompt|secret/);
});

test("redacts URLs, bearer credentials, and structured secret values", () => {
  const message = sanitizeRendererDiagnosticMessage(
    'GET https://example.test/workspace?token=url-secret failed Authorization: Bearer bearer-secret password="password-secret" api_key: api-secret',
  );

  assert.equal(message, "GET [url] failed Authorization: [redacted] password=[redacted] api_key: [redacted]");
  assert.doesNotMatch(message, /url-secret|bearer-secret|password-secret|api-secret/);
});

test("redacts nested JSON secrets when values contain escaped quotes", () => {
  const message = JSON.stringify({
    error: 'Authorization: Bearer leaked-token with a \\"quoted\\" suffix',
    password: 'password-secret with a \\"quote\\"',
    nested: { headers: { authorization: "Bearer nested-secret" } },
    embedded: JSON.stringify({ password: 'embedded-secret \\"quoted-secret-suffix\\"' }),
  });

  const sanitized = sanitizeRendererDiagnosticMessage(message);

  assert.doesNotMatch(sanitized, /leaked-token|password-secret|nested-secret|embedded-secret|quoted-secret-suffix/);
  assert.match(sanitized, /\[redacted\]/);
  assert.doesNotThrow(() => JSON.parse(sanitized));
});

test("redacts file paths and bounds diagnostic messages", () => {
  const message = sanitizeRendererDiagnosticMessage(
    `/Users/froomiebot/workspace/orkworks/apps/desktop/src/App.tsx /workspace/project /opt/secrets /Volumes/private ${"x".repeat(300)}`,
  );

  assert.equal(message.length, 200);
  assert.doesNotMatch(message, /Users|froomiebot|orkworks|App\.tsx/);
  assert.doesNotMatch(message, /workspace\/project|opt\/secrets|Volumes\/private/);
});

test("extracts only an origin from a renderer URL", () => {
  assert.equal(rendererOrigin("http://127.0.0.1:5173/?token=secret"), "http://127.0.0.1:5173");
  assert.equal(rendererOrigin("not a URL"), "unknown");
});
