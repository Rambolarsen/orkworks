import { existsSync, mkdirSync, mkdtempSync, readFileSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { peonSelectionMatchesAppliedState, type PeonAppliedState, type PeonSelection, type ProviderId, type ProviderCapacityState, type ProviderSettings, type ProviderSettingsEntry } from "./providerTypes.ts";

export interface RetentionSettings {
  maxSessions: number;
  maxAgeDays: number;
}

export interface DebugSettings {
  showSessionIds: boolean;
  rendererHealthLogMs: number;
}

export interface AppSettings {
  [key: string]: unknown;
  version: 1;
  hotkeys: HotkeySettings;
  retention: RetentionSettings;
  debug: DebugSettings;
  providers: ProviderSettings;
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

export const DEFAULT_DEBUG_SETTINGS: DebugSettings = {
  showSessionIds: false,
  rendererHealthLogMs: 0,
};

const VALID_PROVIDER_IDS = new Set<ProviderId>(["opencode", "claude-code", "codex", "aider", "copilot", "ollama"]);
const VALID_CAPACITY_STATES = new Set<ProviderCapacityState>(["healthy", "degraded", "capped", "unknown"]);

export const DEFAULT_PROVIDER_SETTINGS: ProviderSettings = {
  version: 2,
  revision: 0,
  peonSelection: null,
  peonModel: null,
  ollamaBaseUrl: "http://127.0.0.1:11434",
  providers: [
    { id: "opencode", model: null, enabled: true, fallbackOrder: 0, defaultState: "healthy", overrideState: null },
    { id: "claude-code", model: null, enabled: true, fallbackOrder: 1, defaultState: "unknown", overrideState: null },
    { id: "codex", model: null, enabled: true, fallbackOrder: 2, defaultState: "unknown", overrideState: null },
    { id: "aider", model: null, enabled: true, fallbackOrder: 3, defaultState: "unknown", overrideState: null },
    { id: "copilot", model: null, enabled: true, fallbackOrder: 4, defaultState: "unknown", overrideState: null },
    { id: "ollama", model: null, enabled: true, fallbackOrder: 5, defaultState: "unknown", overrideState: null },
  ],
};

export const DEFAULT_SETTINGS: AppSettings = {
  version: 1,
  hotkeys: { ...DEFAULT_HOTKEYS },
  retention: { ...DEFAULT_RETENTION },
  debug: { ...DEFAULT_DEBUG_SETTINGS },
  providers: { ...DEFAULT_PROVIDER_SETTINGS, providers: DEFAULT_PROVIDER_SETTINGS.providers.map((p) => ({ ...p })) },
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
    debug: normalizeDebugSettings(parsed.debug),
    providers: normalizeProviderSettings(parsed.providers),
  };
}

export function normalizeDebugSettings(value: unknown): DebugSettings {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return { ...DEFAULT_DEBUG_SETTINGS };
  }
  const raw = value as Record<string, unknown>;
  const showSessionIds =
    typeof raw.showSessionIds === "boolean"
      ? raw.showSessionIds
      : DEFAULT_DEBUG_SETTINGS.showSessionIds;
  const rendererHealthLogMs =
    typeof raw.rendererHealthLogMs === "number" && Number.isFinite(raw.rendererHealthLogMs) && raw.rendererHealthLogMs >= 0
      ? Math.floor(raw.rendererHealthLogMs)
      : DEFAULT_DEBUG_SETTINGS.rendererHealthLogMs;
  return { showSessionIds, rendererHealthLogMs };
}

export function normalizeProviderSettings(value: unknown): ProviderSettings {
  const raw = value && typeof value === "object" && !Array.isArray(value) ? (value as Record<string, unknown>) : {};
  const entries = Array.isArray(raw.providers) ? raw.providers : [];
  const normalizedById = new Map<ProviderId, ProviderSettingsEntry>();

  for (const entry of entries) {
    if (!entry || typeof entry !== "object") continue;
    const candidate = normalizeProviderEntry(entry as Record<string, unknown>);
    if (candidate) normalizedById.set(candidate.id, candidate);
  }

  for (const defaultEntry of DEFAULT_PROVIDER_SETTINGS.providers) {
    if (!normalizedById.has(defaultEntry.id)) normalizedById.set(defaultEntry.id, { ...defaultEntry });
  }

  const providers = Array.from(normalizedById.values())
    .sort((a, b) => a.fallbackOrder - b.fallbackOrder || a.id.localeCompare(b.id))
    .map((entry, index) => ({ ...entry, fallbackOrder: index }));

  return {
    version: 2,
    revision:
      typeof raw.revision === "number" && Number.isFinite(raw.revision)
        ? Math.max(0, Math.trunc(raw.revision))
        : DEFAULT_PROVIDER_SETTINGS.revision,
    peonSelection: normalizePeonSelection(raw, normalizedById),
    peonModel: normalizePeonModel(raw),
    ollamaBaseUrl: normalizeOllamaBaseUrl(raw),
    providers,
  };
}

function normalizePeonSelection(
  raw: Record<string, unknown>,
  entries: Map<ProviderId, ProviderSettingsEntry>,
): PeonSelection | null {
  const candidate = raw.peonSelection;
  if (candidate && typeof candidate === "object" && !Array.isArray(candidate)) {
    const selection = candidate as Record<string, unknown>;
    return normalizedSelection(selection.provider, selection.model, selection.ollamaBaseUrl);
  }

  if (raw.version === 2) return null;

  const modeled = Array.from(entries.values()).filter((entry) => entry.model !== null);
  if (modeled.length !== 1) return null;
  const entry = modeled[0];
  return normalizedSelection(entry.id, entry.model, raw.ollamaBaseUrl);
}

function normalizedSelection(provider: unknown, model: unknown, ollamaBaseUrl: unknown): PeonSelection | null {
  if (!VALID_PROVIDER_IDS.has(provider as ProviderId) || typeof model !== "string") return null;
  const normalizedModel = model.trim();
  if (!normalizedModel) return null;
  if (provider !== "ollama") return { provider: provider as ProviderId, model: normalizedModel };
  const normalizedUrl = ollamaBaseUrl == null
    ? DEFAULT_PROVIDER_SETTINGS.ollamaBaseUrl
    : parseOllamaBaseUrl(ollamaBaseUrl);
  if (!normalizedUrl) return null;
  return { provider: "ollama", model: normalizedModel, ollamaBaseUrl: normalizedUrl };
}

function normalizeOllamaBaseUrl(raw: Record<string, unknown>): string {
  return normalizeOllamaBaseUrlValue(raw.ollamaBaseUrl);
}

function normalizeOllamaBaseUrlValue(value: unknown): string {
  return parseOllamaBaseUrl(value) ?? DEFAULT_PROVIDER_SETTINGS.ollamaBaseUrl;
}

function parseOllamaBaseUrl(value: unknown): string | null {
  if (typeof value !== "string") return null;
  try {
    const parsed = new URL(value.trim().replace(/\/+$/, ""));
    if (!(parsed.protocol === "http:" || parsed.protocol === "https:")
      || parsed.username
      || parsed.password
      || parsed.pathname !== "/"
      || parsed.search
      || parsed.hash) return null;
    return parsed.origin;
  } catch {
    return null;
  }
}

function normalizePeonModel(raw: Record<string, unknown>): string | null {
  const top = raw.peonModel ?? raw.defaultPeonModel;
  if (typeof top === "string" && top) return top;

  const entries = Array.isArray(raw.providers) ? raw.providers : [];
  for (const entry of entries) {
    if (entry && typeof entry === "object") {
      const v = (entry as Record<string, unknown>).peonModel;
      if (typeof v === "string" && v) return v;
    }
  }

  return null;
}

function normalizeProviderEntry(raw: Record<string, unknown>): ProviderSettingsEntry | null {
  const id = raw.id;
  if (!VALID_PROVIDER_IDS.has(id as ProviderId)) return null;
  const defaultEntry = DEFAULT_PROVIDER_SETTINGS.providers.find((p) => p.id === id)!;
  const model = typeof raw.model === "string" ? raw.model.trim() || null : null;
  return {
    id: id as ProviderId,
    model,
    enabled: typeof raw.enabled === "boolean" ? raw.enabled : defaultEntry.enabled,
    fallbackOrder: clampInt(raw.fallbackOrder, 0, Number.MAX_SAFE_INTEGER, defaultEntry.fallbackOrder),
    defaultState: VALID_CAPACITY_STATES.has(raw.defaultState as ProviderCapacityState)
      ? (raw.defaultState as ProviderCapacityState)
      : "unknown",
    overrideState:
      raw.overrideState !== null && VALID_CAPACITY_STATES.has(raw.overrideState as ProviderCapacityState)
        ? (raw.overrideState as ProviderCapacityState)
        : null,
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
  return readSettingsWithMigration(userDataPath).settings;
}

export function readSettingsWithMigration(userDataPath: string): { settings: AppSettings; migrated: boolean } {
  const path = settingsPath(userDataPath);
  if (!existsSync(path)) {
    return { settings: defaultSettings(), migrated: false };
  }
  try {
    const raw = JSON.parse(readFileSync(path, "utf8"));
    const migrated = migrateRawProviderSettings(raw);
    const settings = normalizeSettings(migrated.value);
    const rawProviders = migrated.value && typeof migrated.value === "object" && !Array.isArray(migrated.value)
      ? (migrated.value as Record<string, unknown>).providers
      : undefined;
    return { settings, migrated: migrated.migrated || !jsonValuesEqual(settings.providers, rawProviders) };
  } catch {
    return { settings: defaultSettings(), migrated: false };
  }
}

export function loadSettingsForStartup(
  userDataPath: string,
  persist: (path: string, settings: AppSettings) => void = writeSettings,
): AppSettings {
  const loaded = readSettingsWithMigration(userDataPath);
  if (loaded.migrated) {
    try {
      persist(userDataPath, loaded.settings);
    } catch {
      // The repaired settings remain safe for this process; retry persistence next startup.
    }
  }
  return loaded.settings;
}

export function writeSettings(userDataPath: string, settings: AppSettings): void {
  mkdirSync(userDataPath, { recursive: true });
  const target = settingsPath(userDataPath);
  const temporaryDirectory = mkdtempSync(join(userDataPath, ".settings-"));
  const temporary = join(temporaryDirectory, fileName);
  try {
    writeFileSync(temporary, `${JSON.stringify(normalizeSettings(settings), null, 2)}\n`);
    try {
      renameSync(temporary, target);
    } catch (error) {
      // Windows refuses to rename over an existing file. Removing the target
      // only on that platform preserves the atomic same-directory rename on
      // Unix while allowing subsequent settings saves on Windows.
      const code = error && typeof error === "object" && "code" in error
        ? (error as { code?: string }).code
        : undefined;
      if (process.platform !== "win32" || !["EEXIST", "EPERM", "EBUSY"].includes(code ?? "")) throw error;
      const backup = `${target}.backup`;
      rmSync(backup, { force: true });
      renameSync(target, backup);
      try {
        renameSync(temporary, target);
      } catch (replacementError) {
        renameSync(backup, target);
        throw replacementError;
      }
      rmSync(backup, { force: true });
    }
  } finally {
    rmSync(temporaryDirectory, { recursive: true, force: true });
  }
}

export function settingsWithPeonSelection(baseSettings: AppSettings, selection: PeonSelection): AppSettings {
  const providers = normalizeProviderSettings({
    ...baseSettings.providers,
    version: 2,
    revision: baseSettings.providers.revision + 1,
    peonSelection: selection,
  });
  if (!providers.peonSelection) throw new Error("Invalid Peon provider selection.");
  return { ...baseSettings, version: 1, providers };
}

export function savePeonSelection(
  userDataPath: string,
  selection: PeonSelection,
  persist: (path: string, settings: AppSettings) => void = writeSettings,
): AppSettings {
  const nextSettings = settingsWithPeonSelection(readSettings(userDataPath), selection);
  persist(userDataPath, nextSettings);
  return nextSettings;
}

export { peonSelectionMatchesAppliedState };
export type { PeonAppliedState, PeonSelection };

function migrateRawProviderSettings(value: unknown): { value: unknown; migrated: boolean } {
  if (!value || typeof value !== "object" || Array.isArray(value)) return { value, migrated: false };
  const root = value as Record<string, unknown>;
  const providersValue = root.providers;
  if (!providersValue || typeof providersValue !== "object" || Array.isArray(providersValue)) {
    return { value, migrated: false };
  }
  const providerSettings = providersValue as Record<string, unknown>;
  if (!Array.isArray(providerSettings.providers)) return { value, migrated: false };

  const hasCanonicalCopilot = providerSettings.providers.some(
    (entry) => entry && typeof entry === "object" && !Array.isArray(entry) && (entry as Record<string, unknown>).id === "copilot",
  );
  let migrated = providerSettings.version !== 2;
  let migratedLegacyCopilot = false;
  const providers = providerSettings.providers.flatMap((entry) => {
    if (!entry || typeof entry !== "object" || Array.isArray(entry)) return [entry];
    const raw = entry as Record<string, unknown>;
    if (raw.id === "gh-copilot") {
      migrated = true;
      if (hasCanonicalCopilot || migratedLegacyCopilot) return [];
      migratedLegacyCopilot = true;
      return [{ ...raw, id: "copilot" }];
    }
    if (raw.id === "gemini") {
      migrated = true;
      return [];
    }
    return [entry];
  });

  const nextProviderSettings: Record<string, unknown> = { ...providerSettings, version: 2 };
  if (providerSettings.version !== 2 && !nextProviderSettings.peonSelection) {
    const modeled = providers
      .map((entry) => (entry && typeof entry === "object" && !Array.isArray(entry) ? entry as Record<string, unknown> : null))
      .filter((entry): entry is Record<string, unknown> => entry !== null && VALID_PROVIDER_IDS.has(entry.id as ProviderId) && typeof entry.model === "string" && entry.model.trim().length > 0);
    if (modeled.length === 1) {
      const entry = modeled[0];
      nextProviderSettings.peonSelection = {
        provider: entry.id,
        model: (entry.model as string).trim(),
        ...(entry.id === "ollama" ? { ollamaBaseUrl: providerSettings.ollamaBaseUrl } : {}),
      };
    } else {
      nextProviderSettings.peonSelection = null;
    }
  }

  if (!migrated && JSON.stringify(nextProviderSettings) === JSON.stringify(providerSettings)) return { value, migrated: false };
  return {
    value: { ...root, providers: { ...nextProviderSettings, providers } },
    migrated: true,
  };
}

function jsonValuesEqual(left: unknown, right: unknown): boolean {
  if (left === right) return true;
  if (Array.isArray(left) && Array.isArray(right)) {
    return left.length === right.length && left.every((value, index) => jsonValuesEqual(value, right[index]));
  }
  if (left && typeof left === "object" && right && typeof right === "object") {
    const leftRecord = left as Record<string, unknown>;
    const rightRecord = right as Record<string, unknown>;
    const leftKeys = Object.keys(leftRecord).sort();
    const rightKeys = Object.keys(rightRecord).sort();
    return leftKeys.length === rightKeys.length
      && leftKeys.every((key, index) => key === rightKeys[index] && jsonValuesEqual(leftRecord[key], rightRecord[key]));
  }
  return false;
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
    debug: { ...DEFAULT_DEBUG_SETTINGS },
    providers: { ...DEFAULT_PROVIDER_SETTINGS, providers: DEFAULT_PROVIDER_SETTINGS.providers.map((p) => ({ ...p })) },
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
