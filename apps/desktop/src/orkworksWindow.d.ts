import type { WorkspaceInfo } from "./api";
import type { AppSettings, HotkeySettings, SaveHotkeysResult } from "./appSettingsTypes";

declare global {
  interface Window {
    orkworks: {
      getBackendUrl: () => Promise<string>;
      getInitialWorkspace: () => Promise<WorkspaceInfo | null>;
      openWorkspace: () => Promise<WorkspaceInfo | null>;
      getLayout: () => Promise<string | null>;
      saveLayout: (json: string) => Promise<void>;
      getSettings: () => Promise<AppSettings>;
      saveHotkeys: (hotkeys: HotkeySettings) => Promise<SaveHotkeysResult>;
      setHotkeyCaptureActive: (active: boolean) => void;
      onMenuCommand: (callback: (data: { action: string; panelId?: string }) => void) => () => void;
      notifyPanelVisibility: (panelId: string, visible: boolean) => void;
    };
  }
}
