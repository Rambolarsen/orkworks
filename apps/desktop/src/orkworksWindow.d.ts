import type { WorkspaceInfo } from "./api";
import type { AppSettings, DebugSettings, HotkeySettings, RetentionSettings, SaveHotkeysResult } from "./appSettingsTypes";
import type { ProviderSettings, ProviderModelsResponse, ProviderLabelsResponse, OllamaVerificationResponse, ProviderApplyStatus, RetentionApplyStatus, PeonAppliedState, PeonProviderVerificationResponse, PeonSelectionSaveResult, PeonSelection } from "./providerTypes";
import type { HarnessConfig, IntegrationStatusResult } from "./harnessTypes";

export type BackendLifecycleEvent =
  | { state: "starting" | "retrying" }
  | { state: "ready"; port: number; workspace: WorkspaceInfo | null }
  | { state: "failed" | "exhausted"; message: string };

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
      onBackendLifecycle: (callback: (event: BackendLifecycleEvent) => void) => () => void;
      getInitialWorkspace: () => Promise<WorkspaceInfo | null>;
      openWorkspace: () => Promise<WorkspaceInfo | null>;
      getLayout: () => Promise<string | null>;
      saveLayout: (json: string) => Promise<void>;
      getSettings: () => Promise<AppSettings>;
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
      reconcileHarnessIntegration: (adapterId: string, targetId: string) => Promise<ActiveHarnessIntegrationResult>;
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
