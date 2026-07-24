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

export type IntegrationRegistration = "unsupported" | "absent" | "installed" | "drifted" | "error";
export type IntegrationOwnership = "none" | "ork_works" | "ambiguous";
export type IntegrationActivation = "active" | "needs_trust" | "disabled" | "unknown" | "not_applicable";
export type IntegrationCoverage = "full" | "limited" | "none";

export interface IntegrationDiagnostic {
  code: string;
  message: string;
  action?: string;
}

export interface IntegrationConfirmation {
  toolName: string;
  workspaceLabel: string;
  coverageSummary: string;
  relativePaths: string[];
  executableCodeWarning: boolean;
}

export interface IntegrationStatus {
  harnessId: string;
  enabled: boolean;
  toolDetected: boolean;
  registration: IntegrationRegistration;
  ownership: IntegrationOwnership;
  activation: IntegrationActivation;
  coverage: IntegrationCoverage;
  diagnostics: IntegrationDiagnostic[];
  confirmation: IntegrationConfirmation | null;
}

export type IntegrationStatusResult =
  | { ok: true; status: IntegrationStatus }
  | { ok: false; error: string };
