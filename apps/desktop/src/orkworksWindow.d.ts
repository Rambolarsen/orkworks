import type { WorkspaceInfo } from "./api";
import type { AppSettings, HotkeySettings, RetentionSettings, SaveHotkeysResult } from "./appSettingsTypes";
import type { ProviderSettings, ProviderModelsResponse, ProviderLabelsResponse } from "./providerTypes";

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
      saveProviderSettings: (providers: ProviderSettings) => Promise<{ ok: true; settings: AppSettings }>;
      getProviderModels: (providerId: string) => Promise<ProviderModelsResponse>;
      getProviderLabels: () => Promise<ProviderLabelsResponse>;
      setHotkeyCaptureActive: (active: boolean) => void;
      onMenuCommand: (callback: (data: { action: string; panelId?: string }) => void) => () => void;
      notifyPanelVisibility: (panelId: string, visible: boolean) => void;
    };
  }
}
