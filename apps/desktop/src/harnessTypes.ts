export interface HarnessVoiceCapabilities {
  nativeVoice: boolean;
  requiresMicrophonePermission: boolean;
  orkworksDictation: boolean;
  orkworksVoiceCommands: boolean;
}

export interface HarnessConfig {
  id: string;
  name: string;
  harness: string;
  command: string;
  args: string[];
  defaultModel: string;
  capabilities: HarnessVoiceCapabilities;
  isBuiltin: boolean;
}

export interface CreateSessionOptions {
  harnessId?: string;
  model?: string;
  initialPrompt?: string;
}

export interface AttentionHookStatusResponse {
  installed: boolean;
  error?: string;
}
