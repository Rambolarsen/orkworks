import test from "node:test";
import assert from "node:assert/strict";

import { activeNewSessionHarnesses, canStartNewSession, syncDraftWithHarnesses } from "../src/newSessionDialogState.ts";
import type { HarnessConfig } from "../src/harnessTypes.ts";

function harness(id: string, name = id, defaultModel = ""): HarnessConfig {
  return {
    id,
    name,
    retired: false,
    harness: "generic-shell",
    command: id,
    args: [],
    defaultModel,
    capabilities: {
      nativeVoice: false,
      requiresMicrophonePermission: false,
      orkworksDictation: false,
      orkworksVoiceCommands: false,
    },
    isBuiltin: true,
  };
}

test("syncDraftWithHarnesses replaces a saved retired Gemini selection", () => {
  const draft = syncDraftWithHarnesses(
    { harnessId: "", model: "" },
    [
      { ...harness("gemini", "Gemini CLI"), retired: true },
      harness("antigravity", "Antigravity CLI"),
    ],
    { harnessId: "gemini", model: "gemini-2.5-pro" },
  );

  assert.deepEqual(draft, { harnessId: "antigravity", model: "" });
  assert.equal(
    canStartNewSession(
      [{ ...harness("gemini"), retired: true }, harness("antigravity")],
      "gemini",
    ),
    false,
  );
});

test("retired Gemini current drafts clear their model and add Antigravity to active tools", () => {
  const harnesses = [
    { ...harness("gemini", "Gemini CLI"), retired: true },
    harness("antigravity", "Antigravity CLI"),
    harness("generic-shell", "Shell"),
  ];

  assert.deepEqual(
    syncDraftWithHarnesses({ harnessId: "gemini", model: "gemini-2.5-pro" }, harnesses),
    { harnessId: "antigravity", model: "" },
  );
  assert.deepEqual(
    activeNewSessionHarnesses(harnesses, ["gemini"]).map((entry) => entry.id),
    ["antigravity", "generic-shell"],
  );
});

test("syncDraftWithHarnesses hydrates an empty draft when harnesses arrive later", () => {
  const draft = syncDraftWithHarnesses(
    { harnessId: "", model: "" },
    [harness("codex", "Codex", "gpt-5-codex")],
  );

  assert.equal(draft.harnessId, "codex");
  assert.equal(draft.model, "gpt-5-codex");
});

test("syncDraftWithHarnesses prefers saved draft when harnesses arrive later", () => {
  const draft = syncDraftWithHarnesses(
    { harnessId: "", model: "" },
    [harness("codex", "Codex", "gpt-5-codex"), harness("gemini", "Gemini CLI", "gemini-2.5-pro")],
    { harnessId: "gemini", model: "gemini-2.5-flash" },
  );

  assert.deepEqual(draft, { harnessId: "gemini", model: "gemini-2.5-flash" });
});

test("syncDraftWithHarnesses keeps an existing valid selection", () => {
  const draft = syncDraftWithHarnesses(
    { harnessId: "gemini", model: "gemini-2.5-pro" },
    [harness("codex", "Codex"), harness("gemini", "Gemini CLI", "gemini-2.5-flash")],
  );

  assert.deepEqual(draft, { harnessId: "gemini", model: "gemini-2.5-pro" });
});

test("canStartNewSession allows the empty-harness fallback when harnesses are unavailable", () => {
  assert.equal(canStartNewSession([], ""), true);
  assert.equal(canStartNewSession([harness("codex")], ""), false);
  assert.equal(canStartNewSession([harness("codex")], "codex"), true);
});
