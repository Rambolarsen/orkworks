import test from "node:test";
import assert from "node:assert/strict";
import { outline, initialPresentation, transition } from "../src/components/workflow/workflowPresentation.ts";
import type { Workflow } from "../src/components/workflow/workflowPresentation.ts";

const workflow: Workflow = { id: "run", goal: "Ship the change", stages: [
  { id: "plan", revision: 1, title: "Research", approval: "Approved", tasks: [
    { id: "one", role: "Research", label: "Inspect parser", phase: "session", status: "Working", result: "Unassessed", skills: [] },
    { id: "two", role: "Review", label: "Review parser", phase: "planned", status: "Blocked", blockedReason: "Awaiting research", result: "Unassessed", skills: [] },
  ] },
  { id: "plan", revision: 2, title: "Execution", approval: "Awaiting exact plan approval", tasks: [
    { id: "one", role: "Implementation", label: "Inspect parser", phase: "planned", status: "Planned", result: "Unassessed", skills: [] },
  ] },
] };
const nodes = () => outline(workflow, []);

test("task identity includes run, stage revision and task even when labels repeat", () => {
  const rows = nodes();
  assert.deepEqual(rows.map(n => n.level), [1, 2, 3, 3, 2, 3]);
  assert.equal(new Set(rows.map(n => n.key)).size, 6);
  assert.notEqual(rows[2].key, rows[5].key);
  assert.deepEqual(rows.map(n => n.label), ["Ship the change", "Research", "Inspect parser", "Review parser", "Execution", "Inspect parser"]);
  const changed = structuredClone(workflow); changed.stages[0].tasks[0].status = "Done";
  assert.deepEqual(outline(changed, []).map(n => n.key), rows.map(n => n.key));
});

test("moving focus does not inspect a task; inspection does not change focus", () => {
  const task = nodes()[2];
  const focused = transition(workflow, initialPresentation(workflow), { type: "focus", key: task.key });
  assert.equal(focused.focusedKey, task.key);
  assert.equal(focused.inspectedKey, nodes()[0].key);
  const inspected = transition(workflow, focused, { type: "inspect", key: nodes()[3].key });
  assert.equal(inspected.focusedKey, task.key);
  assert.equal(inspected.inspectedKey, nodes()[3].key);
});

test("collapse reports both inspected and focused descendant protections and is never queued", () => {
  const rows = nodes(); let state = initialPresentation(workflow);
  state = transition(workflow, state, { type: "focus", key: rows[2].key });
  state = transition(workflow, state, { type: "inspect", key: rows[3].key });
  state = transition(workflow, state, { type: "toggle", key: rows[1].key });
  assert.equal(state.collapsed.length, 0);
  assert.equal(state.collapseReasons.length, 2);
  state = transition(workflow, state, { type: "focus", key: rows[1].key });
  state = transition(workflow, state, { type: "inspect", key: rows[0].key });
  assert.equal(state.collapsed.length, 0);
  state = transition(workflow, state, { type: "toggle", key: rows[1].key });
  assert.equal(state.collapsed.length, 1);
  assert.deepEqual(outline(workflow, state.collapsed).map(n => n.label), ["Ship the change", "Research", "Execution", "Inspect parser"]);
  assert.equal(outline(workflow, state.collapsed)[1].blockerCount, 1);
  state = transition(workflow, state, { type: "toggle", key: rows[1].key });
  assert.equal(outline(workflow, state.collapsed).length, 6);
});

test("append preserves focus; removed task falls back to its surviving stage then root", () => {
  const rows = nodes(); let state = initialPresentation(workflow);
  state = transition(workflow, state, { type: "focus", key: rows[2].key });
  state = transition(workflow, state, { type: "inspect", key: rows[2].key });
  const updated = structuredClone(workflow); updated.stages.push({ id: "later", revision: 1, title: "Verify", approval: "Planned", tasks: [] });
  assert.equal(transition(updated, state, { type: "reconcile" }).focusedKey, rows[2].key);
  updated.stages[0].tasks = [];
  state = transition(updated, state, { type: "reconcile" });
  assert.equal(state.focusedKey, rows[1].key); assert.equal(state.inspectedKey, rows[1].key);
  updated.stages.shift();
  state = transition(updated, state, { type: "reconcile" });
  assert.equal(state.focusedKey, rows[0].key); assert.equal(state.inspectedKey, rows[0].key);
});

test("new scenario resets inspection rather than borrowing matching task labels", () => {
  const state = initialPresentation({ ...workflow, id: "other-run" });
  assert.notEqual(state.inspectedKey, initialPresentation(workflow).inspectedKey);
  assert.deepEqual(state.collapsed, []);
});
