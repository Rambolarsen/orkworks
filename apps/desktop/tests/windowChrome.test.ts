import assert from "node:assert/strict";
import test from "node:test";
import { getWindowChromeOptions } from "../electron/windowChrome.ts";

test("Windows integrates native controls and keeps the menu accessible with Alt", () => {
  const options = getWindowChromeOptions("win32");
  assert.equal(options.titleBarStyle, "hidden");
  assert.equal(options.autoHideMenuBar, true);
  assert.deepEqual(options.titleBarOverlay, { color: "#0c0d10", symbolColor: "#eceef1", height: 38 });
  assert.notEqual(options.frame, false, "retain native resize and window behavior");
});

test("macOS and Linux retain their existing window chrome", () => {
  assert.deepEqual(getWindowChromeOptions("darwin"), { titleBarStyle: "hiddenInset" });
  assert.deepEqual(getWindowChromeOptions("linux"), {});
});
