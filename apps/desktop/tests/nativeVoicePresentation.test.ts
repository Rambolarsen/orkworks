import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

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

test("nativeVoicePresentation resolves a session harness display name", () => {
  assert.deepEqual(
    nativeVoicePresentation("GitHub Copilot CLI", [
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

test("SessionDetailPanel uses the normalized session coding tool identity", () => {
  const source = readFileSync(new URL("../src/components/SessionDetailPanel.tsx", import.meta.url), "utf8");
  assert.match(source, /nativeVoicePresentation\(sessionCodingTool\(active\), harnesses\)/);
});
