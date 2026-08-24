import test from "node:test";
import assert from "node:assert/strict";
import { formatBoundaryError } from "../src/errorBoundaryFormatting.ts";

test("formats a real Error with its name, message, and stack", () => {
  const error = new TypeError("cannot read properties of undefined");
  error.stack = "TypeError: cannot read properties of undefined\n    at foo (App.tsx:1:1)";

  const result = formatBoundaryError(error, { componentStack: "\n    in App" });

  assert.equal(result.name, "TypeError");
  assert.equal(result.message, "cannot read properties of undefined");
  assert.equal(result.stack, error.stack);
  assert.equal(result.componentStack, "\n    in App");
});

test("falls back to a generic name and String(error) message for a non-Error throw", () => {
  const result = formatBoundaryError("plain string throw", { componentStack: null });

  assert.equal(result.name, "Error");
  assert.equal(result.message, "plain string throw");
  assert.equal(result.stack, "");
  assert.equal(result.componentStack, "");
});

test("treats a missing componentStack as an empty string", () => {
  const result = formatBoundaryError(new Error("boom"), {});

  assert.equal(result.componentStack, "");
});
