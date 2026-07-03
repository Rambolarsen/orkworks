import type { WorkspaceInfo } from "./api";
import type { AppSettings, DebugSettings, HotkeySettings, RetentionSettings, SaveHotkeysResult } from "./appSettingsTypes";
import type { ProviderSettings, ProviderModelsResponse, ProviderLabelsResponse } from "./providerTypes";
import type { AttentionHookStatusResponse } from "./harnessTypes";

declare global {
  interface Window {
    orkworks: {
      platform: string;
      getBackendUrl: () => Promise<string>;
      getInitialWorkspace: () => Promise<WorkspaceInfo | null>;
      openWorkspace: () => Promise<WorkspaceInfo | null>;
      getLayout: () => Promise<string | null>;
      saveLayout: (json: string) => Promise<void>;
      getSettings: () => Promise<AppSettings>;
      saveHotkeys: (hotkeys: HotkeySettings) => Promise<SaveHotkeysResult>;
      saveRetention: (retention: RetentionSettings) => Promise<{ ok: boolean }>;
      saveDebugSettings: (debug: DebugSettings) => Promise<{ ok: true; settings: AppSettings }>;
      saveProviderSettings: (providers: ProviderSettings) => Promise<{ ok: true; settings: AppSettings }>;
      getProviderModels: (providerId: string) => Promise<ProviderModelsResponse>;
      getProviderLabels: () => Promise<ProviderLabelsResponse>;
      getClaudeCodeHookStatus: () => Promise<AttentionHookStatusResponse>;
      installClaudeCodeHook: () => Promise<AttentionHookStatusResponse>;
      setHotkeyCaptureActive: (active: boolean) => void;
      onMenuCommand: (callback: (data: { action: string; panelId?: string }) => void) => () => void;
      notifyPanelVisibility: (panelId: string, visible: boolean) => void;
    };
  }
}
