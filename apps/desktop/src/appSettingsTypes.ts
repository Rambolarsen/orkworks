import type { ProviderSettings } from "./providerTypes.ts";

export interface RetentionSettings {
  maxSessions: number;
  maxAgeDays: number;
}

export interface HotkeySettings {
  newSession: string;
  toggleSessionsPanel: string;
  toggleDetailPanel: string;
  toggleTerminalPanel: string;
  toggleCapacityPanel: string;
  toggleRecommendationsPanel: string;
  resetLayout: string | null;
}

export interface AppSettings {
  [key: string]: unknown;
  version: 1;
  hotkeys: HotkeySettings;
  defaultHotkeys: HotkeySettings;
  retention: RetentionSettings;
  providers: ProviderSettings;
}

export type SaveHotkeysResult =
  | { ok: true; settings: AppSettings }
  | { ok: false; errors: Partial<Record<keyof HotkeySettings, string[]>> };
