import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

export interface RetentionSettings {
  maxSessions: number;
  maxAgeDays: number;
}

export interface AppSettings {
  [key: string]: unknown;
  version: 1;
  hotkeys: HotkeySettings;
  retention: RetentionSettings;
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

export const DEFAULT_RETENTION: RetentionSettings = {
  maxSessions: 0,
  maxAgeDays: 0,
};

export const DEFAULT_SETTINGS: AppSettings = {
  version: 1,
  hotkeys: { ...DEFAULT_HOTKEYS },
  retention: { ...DEFAULT_RETENTION },
};

const fileName = "settings.json";
const modifierOrder = [
  "CommandOrControl",
  "Command",
  "Control",
  "Alt",
  "AltGr",
  "Shift",
  "Super",
  "Meta",
];
const canonicalModifierNames = new Map([
  ["Command", "Command"],
  ["Cmd", "Command"],
  ["Control", "Control"],
  ["Ctrl", "Control"],
  ["CommandOrControl", "CommandOrControl"],
  ["CmdOrCtrl", "CommandOrControl"],
  ["Alt", "Alt"],
  ["Option", "Alt"],
  ["AltGr", "AltGr"],
  ["Shift", "Shift"],
  ["Super", "Super"],
  ["Meta", "Meta"],
]);
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
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return defaultSettings();
  }
  const parsed = value as Record<string, unknown>;
  return {
    ...parsed,
    version: 1,
    hotkeys: normalizeHotkeys(parsed.hotkeys),
    retention: normalizeRetention(parsed.retention),
  };
}

export function normalizeRetention(value: unknown): RetentionSettings {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return { ...DEFAULT_RETENTION };
  }
  const raw = value as Record<string, unknown>;
  return {
    maxSessions: clampInt(raw.maxSessions, 0, 999, DEFAULT_RETENTION.maxSessions),
    maxAgeDays: clampInt(raw.maxAgeDays, 0, 999, DEFAULT_RETENTION.maxAgeDays),
  };
}

function clampInt(v: unknown, min: number, max: number, fallback: number): number {
  if (typeof v !== "number" || !Number.isFinite(v)) return fallback;
  return Math.max(min, Math.min(max, Math.round(v)));
}

export function settingsWithHotkeys(baseSettings: AppSettings, hotkeys: unknown): AppSettings {
  return {
    ...baseSettings,
    version: 1,
    hotkeys: hotkeysForSave(hotkeys),
  };
}

export function normalizeHotkeys(value: unknown): HotkeySettings {
  const source = value && typeof value === "object" ? (value as Partial<HotkeySettings>) : {};
  const hotkeys: HotkeySettings = {
    newSession: hotkeyOrDefault(source.newSession, DEFAULT_HOTKEYS.newSession),
    toggleSessionsPanel: hotkeyOrDefault(source.toggleSessionsPanel, DEFAULT_HOTKEYS.toggleSessionsPanel),
    toggleDetailPanel: hotkeyOrDefault(source.toggleDetailPanel, DEFAULT_HOTKEYS.toggleDetailPanel),
    toggleTerminalPanel: hotkeyOrDefault(source.toggleTerminalPanel, DEFAULT_HOTKEYS.toggleTerminalPanel),
    toggleCapacityPanel: hotkeyOrDefault(source.toggleCapacityPanel, DEFAULT_HOTKEYS.toggleCapacityPanel),
    toggleRecommendationsPanel: hotkeyOrDefault(
      source.toggleRecommendationsPanel,
      DEFAULT_HOTKEYS.toggleRecommendationsPanel,
    ),
    resetLayout: optionalHotkeyOrNull(source.resetLayout),
  };
  return sanitizeDuplicateHotkeys(hotkeys);
}

export function readSettings(userDataPath: string): AppSettings {
  const path = settingsPath(userDataPath);
  if (!existsSync(path)) {
    return defaultSettings();
  }
  try {
    return normalizeSettings(JSON.parse(readFileSync(path, "utf8")));
  } catch {
    return defaultSettings();
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

    const key = canonicalAccelerator(trimmed);
    const duplicate = seen.get(key);
    if (duplicate) {
      addError(errors, definition.action, `Duplicate shortcut also used by ${duplicate.label}.`);
    } else {
      seen.set(key, definition);
    }
  }

  return Object.keys(errors).length === 0 ? { ok: true, errors } : { ok: false, errors };
}

function defaultSettings(): AppSettings {
  return {
    version: 1,
    hotkeys: { ...DEFAULT_HOTKEYS },
    retention: { ...DEFAULT_RETENTION },
  };
}

function hotkeyOrDefault(value: unknown, fallback: string): string {
  if (typeof value !== "string") return fallback;
  const trimmed = value.trim();
  if (!trimmed) return fallback;
  return acceleratorSyntaxError(trimmed) ? fallback : trimmed;
}

function optionalHotkeyOrNull(value: unknown): string | null {
  if (typeof value !== "string") return null;
  const trimmed = value.trim();
  if (!trimmed) return null;
  return acceleratorSyntaxError(trimmed) ? null : trimmed;
}

function hotkeysForSave(value: unknown): HotkeySettings {
  const source = value && typeof value === "object" && !Array.isArray(value) ? (value as Partial<HotkeySettings>) : {};
  return {
    newSession: requiredHotkeyForSave(source.newSession),
    toggleSessionsPanel: requiredHotkeyForSave(source.toggleSessionsPanel),
    toggleDetailPanel: requiredHotkeyForSave(source.toggleDetailPanel),
    toggleTerminalPanel: requiredHotkeyForSave(source.toggleTerminalPanel),
    toggleCapacityPanel: requiredHotkeyForSave(source.toggleCapacityPanel),
    toggleRecommendationsPanel: requiredHotkeyForSave(source.toggleRecommendationsPanel),
    resetLayout: optionalHotkeyForSave(source.resetLayout),
  };
}

function requiredHotkeyForSave(value: unknown): string {
  return typeof value === "string" ? value.trim() : "";
}

function optionalHotkeyForSave(value: unknown): string | null {
  if (typeof value !== "string") return null;
  const trimmed = value.trim();
  return trimmed ? trimmed : null;
}

function sanitizeDuplicateHotkeys(hotkeys: HotkeySettings): HotkeySettings {
  const next = { ...hotkeys };
  const groups = new Map<string, HotkeyAction[]>();

  for (const definition of HOTKEY_DEFINITIONS) {
    const value = next[definition.action];
    if (!value) continue;
    const key = canonicalAccelerator(value);
    groups.set(key, [...(groups.get(key) ?? []), definition.action]);
  }

  for (const actions of groups.values()) {
    if (actions.length <= 1) continue;
    for (const action of actions) {
      resetHotkeyToDefault(next, action);
    }
  }

  return next;
}

function resetHotkeyToDefault(hotkeys: HotkeySettings, action: HotkeyAction): void {
  switch (action) {
    case "newSession":
      hotkeys.newSession = DEFAULT_HOTKEYS.newSession;
      return;
    case "toggleSessionsPanel":
      hotkeys.toggleSessionsPanel = DEFAULT_HOTKEYS.toggleSessionsPanel;
      return;
    case "toggleDetailPanel":
      hotkeys.toggleDetailPanel = DEFAULT_HOTKEYS.toggleDetailPanel;
      return;
    case "toggleTerminalPanel":
      hotkeys.toggleTerminalPanel = DEFAULT_HOTKEYS.toggleTerminalPanel;
      return;
    case "toggleCapacityPanel":
      hotkeys.toggleCapacityPanel = DEFAULT_HOTKEYS.toggleCapacityPanel;
      return;
    case "toggleRecommendationsPanel":
      hotkeys.toggleRecommendationsPanel = DEFAULT_HOTKEYS.toggleRecommendationsPanel;
      return;
    case "resetLayout":
      hotkeys.resetLayout = DEFAULT_HOTKEYS.resetLayout;
      return;
  }
}

function addError(errors: HotkeyValidationErrors, action: HotkeyAction, message: string): void {
  errors[action] = [...(errors[action] ?? []), message];
}

function acceleratorSyntaxError(accelerator: string): string | null {
  const rawParts = accelerator.split("+").map((part) => part.trim());
  const hasEmptyMiddleSegment = rawParts.slice(0, -1).some((part) => part.length === 0);
  if (hasEmptyMiddleSegment) return "Shortcut has invalid separator syntax.";
  if (
    rawParts.length > 1 &&
    rawParts[rawParts.length - 1] === "" &&
    rawParts.slice(0, -1).some((part) => !modifierNames.has(part))
  ) {
    return "Shortcut has invalid separator syntax.";
  }

  const parts = rawParts.filter(Boolean);
  if (parts.length === 0) return "Shortcut is required.";

  const seenModifiers = new Set<string>();
  for (const part of parts) {
    if (!modifierNames.has(part)) continue;
    const canonicalModifier = canonicalModifierNames.get(part) ?? part;
    if (seenModifiers.has(canonicalModifier)) {
      return `Shortcut contains duplicate modifier "${part}".`;
    }
    seenModifiers.add(canonicalModifier);
  }

  const keyParts = parts.filter((part) => !modifierNames.has(part));
  if (keyParts.length === 0) return "Shortcut must include a non-modifier key.";
  if (keyParts.length > 1) return "Shortcut must contain only one non-modifier key.";
  if (seenModifiers.size === 0 && !isFunctionKey(keyParts[0])) return "Shortcut must include a modifier.";

  return isSupportedKey(keyParts[0]) ? null : `Unsupported key "${keyParts[0]}".`;
}

function canonicalAccelerator(accelerator: string): string {
  const parts = accelerator
    .split("+")
    .map((part) => part.trim())
    .filter(Boolean);
  const modifiers = parts
    .filter((part) => modifierNames.has(part))
    .map((part) => canonicalModifierNames.get(part) ?? part)
    .sort((a, b) => modifierOrder.indexOf(a) - modifierOrder.indexOf(b));
  const key = parts.find((part) => !modifierNames.has(part)) ?? "";

  return [...modifiers, key].join("+").toLowerCase();
}

function isSupportedKey(key: string): boolean {
  if (/^[A-Z0-9]$/.test(key)) return true;
  if (isFunctionKey(key)) return true;
  return namedKeys.has(key);
}

function isFunctionKey(key: string): boolean {
  return /^F([1-9]|1[0-9]|2[0-4])$/.test(key);
}
