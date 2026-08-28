import type { HarnessVoiceCapability } from "./harnessTypes.ts";

type VoiceHarness = {
  id: string;
  name: string;
  voice: HarnessVoiceCapability | null;
};

export interface NativeVoicePresentation {
  label: string;
  detail: string;
}

export function nativeVoicePresentation(
  sessionHarness: string | undefined,
  harnesses: readonly VoiceHarness[],
): NativeVoicePresentation | null {
  if (!sessionHarness) return null;
  const harness = harnesses.find((entry) => entry.id === sessionHarness || entry.name === sessionHarness);
  const voice = harness?.voice;
  if (!voice?.nativeVoice) return null;
  return {
    label: "Voice: native supported",
    detail: voice.requiresMicrophonePermission
      ? "Microphone permission not verified"
      : "Microphone permission not required",
  };
}
