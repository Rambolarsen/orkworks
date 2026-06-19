import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

export interface AppSettings {
  version: 1;
  hotkeys: HotkeySettings;
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

export type HotkeyAction = keyof HotkeySettings;

export interface HotkeyDefinition {
  action: HotkeyAction;
  label: string;
  required: boolean;
  menuAction: "new-session" | "focus" | "reset-layout";
  panelId?: string;
}

export type HotkeyValidationErrors = Partial<Record<HotkeyAction, string[]>>;

export type HotkeyValidationResult =
  | { ok: true; errors: HotkeyValidationErrors }
  | { ok: false; errors: HotkeyValidationErrors };

export const HOTKEY_DEFINITIONS: HotkeyDefinition[] = [
  { action: "newSession", label: "New Session", required: true, menuAction: "new-session" },
  {
    action: "toggleSessionsPanel",
    label: "Sessions Panel",
    required: true,
    menuAction: "focus",
    panelId: "sessions",
  },
  {
    action: "toggleDetailPanel",
    label: "Detail Panel",
    required: true,
    menuAction: "focus",
    panelId: "detail",
  },
  {
    action: "toggleTerminalPanel",
    label: "Terminal Panel",
    required: true,
    menuAction: "focus",
    panelId: "terminal",
  },
  {
    action: "toggleCapacityPanel",
    label: "Capacity Panel",
    required: true,
    menuAction: "focus",
    panelId: "capacity",
  },
  {
    action: "toggleRecommendationsPanel",
    label: "Recommendations Panel",
    required: true,
    menuAction: "focus",
    panelId: "recommendations",
  },
  { action: "resetLayout", label: "Reset Layout", required: false, menuAction: "reset-layout" },
];

export const DEFAULT_HOTKEYS: HotkeySettings = {
  newSession: "CmdOrCtrl+N",
  toggleSessionsPanel: "CmdOrCtrl+Shift+S",
  toggleDetailPanel: "CmdOrCtrl+Shift+D",
  toggleTerminalPanel: "CmdOrCtrl+Shift+T",
  toggleCapacityPanel: "CmdOrCtrl+Shift+C",
  toggleRecommendationsPanel: "CmdOrCtrl+Shift+R",
  resetLayout: null,
};

export const DEFAULT_SETTINGS: AppSettings = {
  version: 1,
  hotkeys: DEFAULT_HOTKEYS,
};

const fileName = "settings.json";
const modifierNames = new Set([
  "Command",
  "Cmd",
  "Control",
  "Ctrl",
  "CommandOrControl",
  "CmdOrCtrl",
  "Alt",
  "Option",
  "AltGr",
  "Shift",
  "Super",
  "Meta",
]);
const namedKeys = new Set([
  "Plus",
  "Space",
  "Tab",
  "Capslock",
  "Numlock",
  "Scrolllock",
  "Backspace",
  "Delete",
  "Insert",
  "Return",
  "Enter",
  "Up",
  "Down",
  "Left",
  "Right",
  "Home",
  "End",
  "PageUp",
  "PageDown",
  "Escape",
  "Esc",
  "VolumeUp",
  "VolumeDown",
  "VolumeMute",
  "MediaNextTrack",
  "MediaPreviousTrack",
  "MediaStop",
  "MediaPlayPause",
  "PrintScreen",
]);

export function settingsPath(userDataPath: string): string {
  return join(userDataPath, fileName);
}

export function normalizeSettings(value: unknown): AppSettings {
  if (!value || typeof value !== "object") {
    return DEFAULT_SETTINGS;
  }
  const parsed = value as Partial<AppSettings>;
  return {
    version: 1,
    hotkeys: normalizeHotkeys(parsed.hotkeys),
  };
}

export function normalizeHotkeys(value: unknown): HotkeySettings {
  const source = value && typeof value === "object" ? (value as Partial<HotkeySettings>) : {};
  return {
    newSession: stringOrDefault(source.newSession, DEFAULT_HOTKEYS.newSession),
    toggleSessionsPanel: stringOrDefault(source.toggleSessionsPanel, DEFAULT_HOTKEYS.toggleSessionsPanel),
    toggleDetailPanel: stringOrDefault(source.toggleDetailPanel, DEFAULT_HOTKEYS.toggleDetailPanel),
    toggleTerminalPanel: stringOrDefault(source.toggleTerminalPanel, DEFAULT_HOTKEYS.toggleTerminalPanel),
    toggleCapacityPanel: stringOrDefault(source.toggleCapacityPanel, DEFAULT_HOTKEYS.toggleCapacityPanel),
    toggleRecommendationsPanel: stringOrDefault(
      source.toggleRecommendationsPanel,
      DEFAULT_HOTKEYS.toggleRecommendationsPanel,
    ),
    resetLayout:
      typeof source.resetLayout === "string" && source.resetLayout.trim().length > 0
        ? source.resetLayout
        : null,
  };
}

export function readSettings(userDataPath: string): AppSettings {
  const path = settingsPath(userDataPath);
  if (!existsSync(path)) {
    return DEFAULT_SETTINGS;
  }
  try {
    return normalizeSettings(JSON.parse(readFileSync(path, "utf8")));
  } catch {
    return DEFAULT_SETTINGS;
  }
}

export function writeSettings(userDataPath: string, settings: AppSettings): void {
  mkdirSync(userDataPath, { recursive: true });
  writeFileSync(settingsPath(userDataPath), `${JSON.stringify(normalizeSettings(settings), null, 2)}\n`);
}

export function validateHotkeys(hotkeys: HotkeySettings): HotkeyValidationResult {
  const errors: HotkeyValidationErrors = {};
  const seen = new Map<string, HotkeyDefinition>();

  for (const definition of HOTKEY_DEFINITIONS) {
    const value = hotkeys[definition.action];
    const trimmed = typeof value === "string" ? value.trim() : "";

    if (!trimmed) {
      if (definition.required) addError(errors, definition.action, "Shortcut is required.");
      continue;
    }

    const syntaxError = acceleratorSyntaxError(trimmed);
    if (syntaxError) {
      addError(errors, definition.action, syntaxError);
      continue;
    }

    const key = trimmed.toLowerCase();
    const duplicate = seen.get(key);
    if (duplicate) {
      addError(errors, definition.action, `Duplicate shortcut also used by ${duplicate.label}.`);
    } else {
      seen.set(key, definition);
    }
  }

  return Object.keys(errors).length === 0 ? { ok: true, errors } : { ok: false, errors };
}

function stringOrDefault(value: unknown, fallback: string): string {
  return typeof value === "string" && value.trim().length > 0 ? value : fallback;
}

function addError(errors: HotkeyValidationErrors, action: HotkeyAction, message: string): void {
  errors[action] = [...(errors[action] ?? []), message];
}

function acceleratorSyntaxError(accelerator: string): string | null {
  const parts = accelerator
    .split("+")
    .map((part) => part.trim())
    .filter(Boolean);
  if (parts.length === 0) return "Shortcut is required.";

  const keyParts = parts.filter((part) => !modifierNames.has(part));
  if (keyParts.length === 0) return "Shortcut must include a non-modifier key.";
  if (keyParts.length > 1) return "Shortcut must contain only one non-modifier key.";

  return isSupportedKey(keyParts[0]) ? null : `Unsupported key "${keyParts[0]}".`;
}

function isSupportedKey(key: string): boolean {
  if (/^[A-Z0-9]$/.test(key)) return true;
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(key)) return true;
  return namedKeys.has(key);
}
