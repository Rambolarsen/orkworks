import type { ShellMemoryDiagnostic, ShellPreferences } from "./orkworksWindow";

export const DEFAULT_SHELL_PREFERENCES: ShellPreferences = {
  sessionsWidth: 240, inspectorWidth: 320, sessionsVisible: true, density: "low",
};

type Result = { ok: boolean; diagnostic?: ShellMemoryDiagnostic | "user_cancelled" };
type Dependencies = {
  read: () => Promise<{ preferences: ShellPreferences; diagnostic: ShellMemoryDiagnostic | null; revision?: number }>;
  save: (preferences: ShellPreferences) => Promise<Result>;
  reset: () => Promise<Result>;
  rebuild: () => Promise<Result>;
  readLegacy: () => Promise<string | null>;
  onMigrationNotice: () => void;
  onDiagnostic: (diagnostic: ShellMemoryDiagnostic | null) => void;
  onChange: (preferences: ShellPreferences) => void;
  onError: () => void;
  schedule?: (callback: () => void) => () => void;
};

/** Owns only presentation intents; Electron owns the revisioned persistent store. */
export function createShellPreferencesController(deps: Dependencies) {
  let preferences = { ...DEFAULT_SHELL_PREFERENCES };
  let version = 0;
  let disposed = false;
  let diagnostic: ShellMemoryDiagnostic | null = null;
  let cancelSave: (() => void) | null = null;
  let writes = Promise.resolve();
  let loading: Promise<void> | null = null;
  const schedule = deps.schedule ?? ((callback: () => void) => {
    const timer = setTimeout(callback, 500);
    return () => clearTimeout(timer);
  });
  const reportFailure = (result: Result) => {
    if (result.diagnostic === "user_cancelled") return;
    if (result.diagnostic === "corrupt_record" || result.diagnostic === "unsupported_version") {
      diagnostic = result.diagnostic;
      deps.onDiagnostic(diagnostic);
    }
    deps.onError();
  };
  const report = async (operation: () => Promise<Result>) => {
    try {
      const result = await operation();
      if (!result.ok && !disposed) reportFailure(result);
    }
    catch { if (!disposed) deps.onError(); }
  };
  const load = () => loading ??= (async () => {
    const initialVersion = version;
    try {
      const snapshot = await deps.read();
      if (disposed) return;
      if (initialVersion === 0 && version === 0) {
        preferences = { ...snapshot.preferences };
        deps.onChange(preferences);
      }
      diagnostic = snapshot.diagnostic;
      deps.onDiagnostic(diagnostic);
      if (diagnostic) deps.onError();
      if (!diagnostic && snapshot.revision === 0) {
        const legacy = await deps.readLegacy();
        if (!disposed && legacy !== null) deps.onMigrationNotice();
      }
    } catch { if (!disposed) deps.onError(); }
  })();
  return {
    load,
    change(next: ShellPreferences) {
      if (disposed) return;
      version++;
      preferences = { ...next };
      deps.onChange(preferences);
      cancelSave?.();
      const intent = version;
      cancelSave = schedule(() => {
        cancelSave = null;
        writes = writes.then(async () => {
          await load();
          if (!disposed && intent === version) await report(() => deps.save(preferences));
        });
      });
    },
    async reset() {
      if (disposed) return false;
      let completed = false;
      const intent = ++version;
      cancelSave?.();
      cancelSave = null;
      // Reset is an ordering barrier even when a newer resize is already queued.
      writes = writes.then(async () => {
        await load();
        if (disposed) return;
        try {
          const needsRebuild = diagnostic === "corrupt_record" || diagnostic === "unsupported_version";
          const result = await (needsRebuild ? deps.rebuild() : deps.reset());
          if (disposed) return;
          if (result.ok) {
            diagnostic = null;
            deps.onDiagnostic(null);
            if (intent === version) {
              completed = true;
              preferences = { ...DEFAULT_SHELL_PREFERENCES };
              deps.onChange(preferences);
            }
          } else reportFailure(result);
        } catch { if (!disposed) deps.onError(); }
      });
      await writes;
      return completed;
    },
    dispose() { disposed = true; cancelSave?.(); cancelSave = null; },
  };
}
