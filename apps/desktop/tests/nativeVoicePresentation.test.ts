import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { nativeVoicePresentation } from "../src/nativeVoicePresentation.ts";
import { sessionCodingTool } from "../src/sessionProviderContext.ts";
import type { SessionInfo } from "../src/api.ts";

function sessionWithHarnessIdOnly(): SessionInfo {
  return {
    id: "session-1",
    label: "Copilot session",
    harnessId: "copilot",
    model: "gpt-5.3-codex",
    status: "running",
    cwd: "/tmp/project",
    created_at: "2026-08-28T10:00:00Z",
    memoryState: "live",
    resumeStrategy: "none",
  };
}

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

test("harnessId-only sessions reach native voice presentation", () => {
  const session = sessionWithHarnessIdOnly();
  assert.deepEqual(
    nativeVoicePresentation(sessionCodingTool(session), [
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
