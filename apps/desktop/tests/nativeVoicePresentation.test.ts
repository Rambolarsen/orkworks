import test from "node:test";
import assert from "node:assert/strict";

import { nativeVoicePresentation } from "../src/nativeVoicePresentation.ts";

test("nativeVoicePresentation reports native voice and unverified microphone state", () => {
  assert.deepEqual(
    nativeVoicePresentation("copilot", [
      {
        id: "copilot",
        name: "GitHub Copilot CLI",
        voice: {
          nativeVoice: true,
          requiresMicrophonePermission: true,
          orkworksDictation: false,
          orkworksVoiceCommands: false,
        },
      },
    ]),
    { label: "Voice: native supported", detail: "Microphone permission not verified" },
  );
});

test("nativeVoicePresentation ignores harnesses without native voice", () => {
  assert.equal(nativeVoicePresentation("codex", [
    { id: "codex", name: "Codex", voice: null },
  ]), null);
});
