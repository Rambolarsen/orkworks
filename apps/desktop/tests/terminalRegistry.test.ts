import test from "node:test";
import assert from "node:assert/strict";
import { createTerminalRegistry } from "../src/terminalRegistry.ts";

test("set then get returns the handle", () => {
  const reg = createTerminalRegistry<{ x: number }>();
  reg.set("a", { x: 1 });
  assert.equal(reg.size, 1);
  assert.deepEqual(reg.get("a"), { x: 1 });
  assert.equal(reg.isDisposed("a"), false);
});

test("remove returns the handle and marks disposed; second remove returns undefined", () => {
  const reg = createTerminalRegistry<{ x: number }>();
  reg.set("a", { x: 1 });
  const removed = reg.remove("a");
  assert.deepEqual(removed, { x: 1 });
  assert.equal(reg.get("a"), undefined);
  assert.equal(reg.size, 0);
  assert.equal(reg.isDisposed("a"), true);
  assert.equal(reg.remove("a"), undefined);
});

test("prune keeps the keep-set and returns the disposed handles in insertion order", () => {
  const reg = createTerminalRegistry<number>();
  reg.set("a", 1);
  reg.set("b", 2);
  reg.set("c", 3);
  const removed = reg.prune(new Set(["a", "never-seen"]));
  assert.deepEqual(removed, [2, 3]);
  assert.equal(reg.size, 1);
  assert.deepEqual(reg.get("a"), 1);
  assert.equal(reg.get("b"), undefined);
  assert.equal(reg.get("c"), undefined);
  assert.equal(reg.isDisposed("b"), true);
  assert.equal(reg.isDisposed("c"), true);
  assert.equal(reg.isDisposed("never-seen"), false);
});

test("prune with empty keep-set disposes everything", () => {
  const reg = createTerminalRegistry<number>();
  reg.set("a", 1);
  reg.set("b", 2);
  assert.deepEqual(reg.prune(new Set()), [1, 2]);
  assert.equal(reg.size, 0);
});

test("prune is idempotent and ignores ids in keep that were never registered", () => {
  const reg = createTerminalRegistry<number>();
  reg.set("a", 1);
  assert.deepEqual(reg.prune(new Set(["a", "z"])), []);
  assert.deepEqual(reg.prune(new Set()), [1]);
});

test("liveIds returns a snapshot of currently-live ids", () => {
  const reg = createTerminalRegistry<number>();
  reg.set("a", 1);
  reg.set("b", 2);
  const ids = reg.liveIds();
  assert.deepEqual([...ids].sort(), ["a", "b"]);
  reg.remove("a");
  assert.deepEqual([...reg.liveIds()], ["b"]);
});

test("isDisposed is false for ids the registry has never seen", () => {
  const reg = createTerminalRegistry<number>();
  assert.equal(reg.isDisposed("never-seen"), false);
});