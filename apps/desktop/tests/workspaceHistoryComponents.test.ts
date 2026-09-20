import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const listSource = readFileSync(new URL("../src/components/WorkspaceHistoryList.tsx", import.meta.url), "utf8");
const dropdownSource = readFileSync(new URL("../src/components/WorkspaceHistoryDropdown.tsx", import.meta.url), "utf8");
const labelsSource = readFileSync(new URL("../src/labels.ts", import.meta.url), "utf8");

test("WorkspaceHistoryList disables opening the current workspace but keeps pin/remove available", () => {
  assert.match(listSource, /disabled=\{isCurrent \|\| isSwitching\}/);
  assert.match(listSource, /onClick=\{\(\) => \(pinned \? handleUnpin\(path\) : handlePin\(path\)\)\}/);
  assert.match(listSource, /onClick=\{\(\) => handleForget\(path\)\}/);
});

test("WorkspaceHistoryList renders pinned and recent as separate sections using VOCAB", () => {
  assert.match(listSource, /VOCAB\.workspaceHistoryPinnedSection/);
  assert.match(listSource, /VOCAB\.workspaceHistoryRecentSection/);
  assert.match(listSource, /VOCAB\.workspaceHistoryEmpty/);
  assert.match(listSource, /VOCAB\.workspaceHistoryOpenOtherFolder/);
  assert.doesNotMatch(listSource, /"Pinned"|"Recent"/);
});

test("WorkspaceHistoryDropdown dismisses on outside click and Escape without a new dependency", () => {
  assert.match(dropdownSource, /document\.addEventListener\("mousedown", handlePointerDown\)/);
  assert.match(dropdownSource, /document\.addEventListener\("keydown", handleKeyDown\)/);
  assert.match(dropdownSource, /event\.key === "Escape"/);
});

test("labels.ts defines the workspace-history VOCAB entries", () => {
  assert.match(labelsSource, /pinWorkspace: "Pin workspace"/);
  assert.match(labelsSource, /unpinWorkspace: "Unpin workspace"/);
  assert.match(labelsSource, /removeWorkspace: "Remove from history"/);
});

test("WorkspaceHistoryList extracts basenames on both POSIX and Windows paths", () => {
  assert.match(listSource, /path\.split\(\/\[.*\\.*\/\]\/\)\.pop\(\)/);
});
