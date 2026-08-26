import type { WorkspaceInfo } from "./api";
import type { AppSettings, DebugSettings, HotkeySettings, RetentionSettings, SaveHotkeysResult } from "./appSettingsTypes";
import type { ProviderSettings, ProviderModelsResponse, ProviderLabelsResponse, OllamaVerificationResponse, ProviderApplyStatus, RetentionApplyStatus } from "./providerTypes";
import type { HarnessConfig, IntegrationStatusResult } from "./harnessTypes";

export type BackendLifecycleEvent =
  | { state: "starting" | "retrying" }
  | { state: "ready"; port: number; workspace: WorkspaceInfo | null }
  | { state: "failed" | "exhausted"; message: string };

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
      saveHotkeys: (hotkeys: HotkeySettings) => Promise<SaveHotkeysResult>;
      saveRetention: (retention: RetentionSettings) => Promise<{ ok: boolean; retentionApplyStatus?: RetentionApplyStatus }>;
      saveDebugSettings: (debug: DebugSettings) => Promise<{ ok: true; settings: AppSettings }>;
      saveProviderSettings: (providers: ProviderSettings) => Promise<{ ok: true; settings: AppSettings; providerApplyStatus?: ProviderApplyStatus }>;
      verifyOllama: (baseUrl: string) => Promise<OllamaVerificationResponse>;
      getProviderModels: (providerId: string) => Promise<ProviderModelsResponse>;
      getProviderLabels: () => Promise<ProviderLabelsResponse>;
      getHarnessIntegrationStatus: (harnessId: string) => Promise<IntegrationStatusResult>;
      installHarnessIntegration: (harnessId: string) => Promise<IntegrationStatusResult>;
      uninstallHarnessIntegration: (harnessId: string) => Promise<IntegrationStatusResult>;
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
