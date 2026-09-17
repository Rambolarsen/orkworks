import type { WorkspaceInfo } from "./api";
import type { AppSettings, DebugSettings, HotkeySettings, RetentionSettings, SaveHotkeysResult } from "./appSettingsTypes";
import type { ProviderSettings, ProviderModelsResponse, ProviderLabelsResponse, OllamaVerificationResponse, ProviderApplyStatus, RetentionApplyStatus, PeonAppliedState, PeonProviderVerificationResponse, PeonSelectionSaveResult, PeonSelection } from "./providerTypes";
import type { HarnessConfig, IntegrationStatusResult } from "./harnessTypes";

export type BackendLifecycleEvent =
  | { state: "starting" | "retrying" }
  | { state: "ready"; port: number; workspace: WorkspaceInfo | null }
  | { state: "failed" | "exhausted"; message: string };

export interface UpdateCandidateIdentity {
  channel: "latest" | "nightly";
  version: string;
  tag: string;
  metadataUrl: string;
  metadataDigest: string;
  payloadDigest: string;
}

export interface UpdateCandidate {
  identity: UpdateCandidateIdentity;
  releaseNotes: string | null;
  publishedAt: string | null;
}

export type UpdateStatus =
  | { state: "unavailable"; reason: "development" | "unsupported-version"; sequence: number }
  | { state: "never-checked"; channel: "latest" | "nightly"; currentVersion: string; sequence: number }
  | { state: "checking"; channel: "latest" | "nightly"; currentVersion: string; sequence: number }
  | { state: "up-to-date"; channel: "latest" | "nightly"; currentVersion: string; checkedAt: string; sequence: number }
  | { state: "available"; candidate: UpdateCandidate; sequence: number }
  | { state: "downloading"; candidate: UpdateCandidate; progress: { percent: number; transferred: number; total: number }; sequence: number }
  | { state: "downloaded"; candidate: UpdateCandidate; sequence: number }
  | { state: "installing"; candidate: UpdateCandidate; sequence: number }
  | {
      state: "error";
      operation: "check" | "download" | "install";
      message: string;
      retryable: true;
      candidate?: UpdateCandidate;
      sequence: number;
    };

export type IntegrationKey = {
  adapterId: string;
  targetId: string;
};

export type IntegrationConsumer = {
  harnessId: string;
  harnessName: string;
};

export type GroupedIntegrationStatus = {
  key: IntegrationKey;
  consumers: IntegrationConsumer[];
  status: {
    harnessId: string;
    enabled: boolean;
    toolDetected: boolean;
    registration: "unsupported" | "absent" | "installed" | "drifted" | "error";
    ownership: "none" | "ork_works" | "ambiguous";
    activation: "active" | "needs_trust" | "disabled" | "unknown" | "not_applicable";
    coverage: "full" | "limited" | "none";
    diagnostics: Array<{ code: string; message: string; action?: string }>;
    confirmation: {
      toolName: string;
      workspaceLabel: string;
      coverageSummary: string;
      relativePaths: string[];
      executableCodeWarning: boolean;
    } | null;
  };
};

export type GroupedIntegrationStatusResult =
  | { ok: true; group: GroupedIntegrationStatus }
  | { ok: false; error: string; code?: string };

export type ActiveHarnessIntegrationResult = {
  key: IntegrationKey;
  consumerHarnessIds: string[];
  operation: "install" | "repair" | "uninstall" | "skipped";
  outcome: "succeeded" | "failed" | "unsupported" | "stale_workspace";
  registration: "unsupported" | "absent" | "installed" | "drifted" | "error";
  activation: "active" | "needs_trust" | "disabled" | "unknown" | "not_applicable";
  coverage: "full" | "limited" | "none";
  diagnosticCode?: string;
  message?: string;
};

export type ActiveHarnessSaveResult = {
  activeHarnesses: {
    outcome: "persisted" | "failed" | "stale_workspace";
    message?: string;
  };
  integrations: Record<string, ActiveHarnessIntegrationResult>;
};

declare global {
  interface Window {
    orkworks: {
      platform: string;
      getBackendUrl: () => Promise<string>;
      retryBackend: () => Promise<void>;
      getUpdateStatus: () => Promise<UpdateStatus>;
      checkForUpdates: () => Promise<UpdateStatus>;
      downloadUpdate: () => Promise<UpdateStatus>;
      requestUpdateInstall: () => Promise<UpdateStatus>;
      onUpdateStatus: (callback: (status: UpdateStatus) => void) => () => void;
      onBackendLifecycle: (callback: (event: BackendLifecycleEvent) => void) => () => void;
      getInitialWorkspace: () => Promise<WorkspaceInfo | null>;
      openWorkspace: () => Promise<WorkspaceInfo | null>;
      getLayout: () => Promise<string | null>;
      saveLayout: (json: string) => Promise<void>;
    getSettings: () => Promise<AppSettings>;
      getTaskmasterSettings: () => Promise<import("./taskmasterSettings").TaskmasterSettingsStatus>;
      getInferenceTrust: () => Promise<import("./inferenceTrust").InferenceAdapterView[]>;
      approveInferenceAdapter: (request: import("./inferenceTrust").InferenceTrustRequest) => Promise<boolean>;
      revokeInferenceAdapter: (request: import("./inferenceTrust").InferenceTrustRequest) => Promise<void>;
    saveTaskmasterSettings: (settings: import("./taskmasterSettings").TaskmasterSettings) => Promise<import("./taskmasterSettings").TaskmasterSettingsStatus>;
      verifyPeonProvider: (provider: string, ollamaBaseUrl?: string) => Promise<PeonProviderVerificationResponse>;
      testAndApplyPeonProvider: (selection: PeonSelection) => Promise<PeonAppliedState>;
      getAppliedPeonProvider: () => Promise<PeonAppliedState>;
      savePeonSelection: (selection: PeonSelection) => Promise<PeonSelectionSaveResult | { ok: false; error: string }>;
      saveHotkeys: (hotkeys: HotkeySettings) => Promise<SaveHotkeysResult>;
      saveRetention: (retention: RetentionSettings) => Promise<{ ok: boolean; retentionApplyStatus?: RetentionApplyStatus }>;
      saveDebugSettings: (debug: DebugSettings) => Promise<{ ok: true; settings: AppSettings }>;
      saveProviderSettings: (providers: ProviderSettings) => Promise<{ ok: true; settings: AppSettings; providerApplyStatus?: ProviderApplyStatus }>;
      verifyOllama: (baseUrl: string) => Promise<OllamaVerificationResponse>;
      getProviderModels: (providerId: string) => Promise<ProviderModelsResponse>;
      getProviderLabels: () => Promise<ProviderLabelsResponse>;
      saveActiveHarnessesWithIntegrations: (ids: string[]) => Promise<ActiveHarnessSaveResult>;
      enableHarnessIntegrationImmediate: (ids: string[], adapterId: string, targetId: string) => Promise<ActiveHarnessSaveResult>;
      getHarnessIntegrationStatus: (harnessId: string) => Promise<IntegrationStatusResult>;
      getGroupedHarnessIntegrationStatus: (adapterId: string, targetId: string) => Promise<GroupedIntegrationStatusResult>;
      setHarnessCommandOverride: (
        harnessId: string,
        commandPath: string,
      ) => Promise<{ ok: true; harness: HarnessConfig } | { ok: false; error: string }>;
      clearHarnessCommandOverride: (
        harnessId: string,
      ) => Promise<{ ok: true } | { ok: false; error: string }>;
      openExternalLink: (url: string) => Promise<void>;
      getPlanContent: (sessionId: string) => Promise<string>;
      requestPlanReview: (sessionId: string) => Promise<void>;
      selectTerminalPlan: (sessionId: string, printedPath: string) => Promise<void>;
      setHotkeyCaptureActive: (active: boolean) => void;
      onMenuCommand: (callback: (data: { action: string; panelId?: string }) => void) => () => void;
      notifyPanelVisibility: (panelId: string, visible: boolean) => void;
    };
  }
}
