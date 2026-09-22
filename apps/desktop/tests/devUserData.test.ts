import assert from "node:assert/strict";
import test from "node:test";

import { getDevUserDataPath } from "../electron/paths.ts";

test("dev user data path is stable per repo root and dev port", () => {
  const base = "/tmp/base-user-data";
  const electronDir = "/checkout/apps/desktop/electron";
  const first = getDevUserDataPath(electronDir, base, "5273");
  const second = getDevUserDataPath(electronDir, base, "5273");

  assert.equal(first, second);
  assert.ok(first.startsWith(base + "/"));
  assert.notEqual(first, base);
  assert.match(first, /^\/tmp\/base-user-data\/dev-[0-9a-f]+$/);
});

test("dev user data path differs per dev port", () => {
  const base = "/tmp/base-user-data";
  const electronDir = "/checkout/apps/desktop/electron";
  const one = getDevUserDataPath(electronDir, base, "5273");
  const other = getDevUserDataPath(electronDir, base, "5373");

  assert.notEqual(one, other);
});

test("dev user data path differs per repo root", () => {
  const base = "/tmp/base-user-data";
  const port = "5273";
  const one = getDevUserDataPath("/checkout-a/apps/desktop/electron", base, port);
  const other = getDevUserDataPath("/checkout-b/apps/desktop/electron", base, port);

  assert.notEqual(one, other);
});
