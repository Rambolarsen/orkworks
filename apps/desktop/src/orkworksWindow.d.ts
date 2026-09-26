import type { SessionAttention, WorkflowRecommendation, WorkspaceInfo } from "./api";
import type { AppSettings, DebugSettings, HotkeySettings, RetentionSettings, SaveHotkeysResult } from "./appSettingsTypes";
import type { ProviderSettings, ProviderModelsResponse, ProviderLabelsResponse, OllamaVerificationResponse, ProviderApplyStatus, RetentionApplyStatus, PeonAppliedState, PeonProviderVerificationResponse, PeonSelectionSaveResult, PeonSelection } from "./providerTypes";
import type { HarnessConfig, IntegrationStatusResult } from "./harnessTypes";

export type BackendLifecycleEvent =
  | { state: "picker"; failure?: WorkspaceLifecycleFailure }
  | { state: "opening" | "closing" }
  | { state: "starting" | "retrying" }
  | { state: "ready"; port: number; workspace: WorkspaceInfo | null; historyDiagnostic: WorkspaceHistoryDiagnostic | null }
  | { state: "unresolved"; failure: WorkspaceLifecycleFailure }
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
  | { state: "unavailable"; reason: "development" | "unsupported-platform" | "unsupported-version"; sequence: number }
  | { state: "never-checked"; channel: "latest" | "nightly"; currentVersion: string; sequence: number }
  | { state: "checking"; channel: "latest" | "nightly"; currentVersion: string; sequence: number }
  | { state: "up-to-date"; channel: "latest" | "nightly"; currentVersion: string; checkedAt: string; sequence: number }
  | { state: "available"; currentVersion: string; candidate: UpdateCandidate; sequence: number }
  | { state: "downloading"; currentVersion: string; candidate: UpdateCandidate; progress: { percent: number; transferred: number; total: number }; sequence: number }
  | { state: "downloaded"; currentVersion: string; candidate: UpdateCandidate; sequence: number }
  | { state: "installing"; currentVersion: string; candidate: UpdateCandidate; sequence: number }
  | {
      state: "error";
      currentVersion: string;
      operation: "check" | "download" | "install";
      message: string;
      retryable: true;
      candidate?: UpdateCandidate;
      sequence: number;
    };
export type BackendRetryResult =
  | { ok: true; state: "ready" }
  | { ok: false; state: "picker" | "unresolved"; failure: WorkspaceLifecycleFailure };

export type WorkspaceLifecycleFailure = {
  code: "invalid_destination" | "cleanup_failed" | "cleanup_timeout" | "destination_conflict" | "readiness_failed" | "restoration_failed" | "quit_failed";
  message: string;
};

export type WorkspaceHistoryDiagnostic = {
  code: "corrupt_history" | "history_lock_timeout" | "history_write_failed" | "pin_limit_reached";
  message: string;
};

export type WorkspaceHistorySnapshot = {
  pinned: string[];
  recent: string[];
  diagnostic: WorkspaceHistoryDiagnostic | null;
};

export type InitialWorkspaceSnapshot = {
  workspace: WorkspaceInfo | null;
  historyDiagnostic: WorkspaceHistoryDiagnostic | null;
};

export type AcceptRecommendationOptions = {
  sessionId: string;
  prompt?: string;
  packetRevision?: number;
  evidenceFingerprint?: string;
  idempotencyKey?: string;
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
      retryBackend: () => Promise<BackendRetryResult>;
      getUpdateStatus: () => Promise<UpdateStatus>;
      checkForUpdates: () => Promise<UpdateStatus>;
      downloadUpdate: () => Promise<UpdateStatus>;
      requestUpdateInstall: () => Promise<UpdateStatus>;
      onUpdateStatus: (callback: (status: UpdateStatus) => void) => () => void;
      onBackendLifecycle: (callback: (event: BackendLifecycleEvent) => void) => () => void;
      getInitialWorkspace: () => Promise<InitialWorkspaceSnapshot>;
      openWorkspace: () => Promise<WorkspaceInfo | null>;
      getWorkspaceHistory: () => Promise<WorkspaceHistorySnapshot>;
      pinWorkspacePath: (path: string) => Promise<WorkspaceHistorySnapshot>;
      unpinWorkspacePath: (path: string) => Promise<WorkspaceHistorySnapshot>;
      forgetWorkspacePath: (path: string) => Promise<WorkspaceHistorySnapshot>;
      openRememberedWorkspace: (path: string) => Promise<WorkspaceInfo | null>;
      getLayout: () => Promise<string | null>;
      saveLayout: (json: string) => Promise<void>;
      getSettings: () => Promise<AppSettings>;
      getTaskmasterSettings: () => Promise<import("./taskmasterSettings").TaskmasterSettingsStatus>;
      requestTaskmasterAnalysis: () => Promise<import("./api").ManualTaskmasterAnalysisResponse>;
      dismissTaskmasterRecommendation: (id: string, reason?: string) => Promise<void>;
      acceptTaskmasterRecommendation: (id: string, options: AcceptRecommendationOptions) => Promise<WorkflowRecommendation>;
      applyDebugAttention: (id: string, attention: SessionAttention, message?: string) => Promise<void>;
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
