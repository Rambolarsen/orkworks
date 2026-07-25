/** Mirrors crates/orkworksd/src/harness/definition.rs LaunchCapability. */
export type HarnessLaunch =
  | { kind: "command-template"; command: string; args: string[]; modelPrefix: string | null }
  | { kind: "platform-shell"; login: boolean };

/** Mirrors crates/orkworksd/src/harness/definition.rs HarnessDefinition (v2, resolved-registry shape). */
export interface HarnessConfig {
  id: string;
  name: string;
  launch: HarnessLaunch;
  defaultModel: string | null;
  resume: unknown;
  models: unknown;
  peon: unknown;
  capacity: unknown;
  sessionSignals: unknown;
  integration: unknown;
  voice: unknown;
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
