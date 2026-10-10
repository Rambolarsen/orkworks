import test from "node:test";
import assert from "node:assert/strict";
import { computeShellLayout } from "../src/fixedShellLayout.ts";

const preferences = { sessionsWidth: 320, inspectorWidth: 420, sessionsVisible: true, density: "low" as const };

test("wide layout clamps regions before violating the central minimum", () => {
  const layout = computeShellLayout(1180, preferences, true);
  assert.equal(layout.mode, "wide");
  assert.equal(layout.inspectorPage, false);
  assert.ok(layout.sessionsWidth >= 200 && layout.sessionsWidth <= 320);
  assert.ok(layout.inspectorWidth >= 280 && layout.inspectorWidth <= 420);
  assert.ok(1180 - layout.sessionsWidth - layout.inspectorWidth - 12 >= 560);
});

test("responsive boundaries use one temporary utility page below 1180", () => {
  assert.equal(computeShellLayout(1180, preferences, true).inspectorPage, false);
  assert.equal(computeShellLayout(1179, preferences, true).inspectorPage, true);
  assert.equal(computeShellLayout(860, preferences, true).mode, "medium");
  assert.equal(computeShellLayout(859, preferences, true).mode, "compact");
  assert.equal(computeShellLayout(640, preferences, true).sessionsVisible, false);
});

test("hidden regions consume no width and preferences survive responsive clamping", () => {
  const layout = computeShellLayout(1280, { ...preferences, sessionsVisible: false }, false);
  assert.equal(layout.sessionsVisible, false);
  assert.equal(layout.inspectorWidth, 0);
  assert.equal(preferences.sessionsWidth, 320);
  assert.equal(preferences.inspectorWidth, 420);
});
