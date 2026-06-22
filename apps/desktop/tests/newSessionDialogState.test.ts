import test from "node:test";
import assert from "node:assert/strict";

import { canStartNewSession, syncDraftWithHarnesses } from "../src/newSessionDialogState.ts";
import type { HarnessConfig } from "../src/harnessTypes.ts";

function harness(id: string, name = id, defaultModel = ""): HarnessConfig {
  return {
    id,
    name,
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

test("syncDraftWithHarnesses hydrates an empty draft when harnesses arrive later", () => {
  const draft = syncDraftWithHarnesses(
    { harnessId: "", model: "" },
    [harness("codex", "Codex", "gpt-5-codex")],
  );

  assert.equal(draft.harnessId, "codex");
  assert.equal(draft.model, "gpt-5-codex");
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
