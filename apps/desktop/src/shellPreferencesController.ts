import type { ShellPreferences } from "./orkworksWindow";

export const DEFAULT_SHELL_PREFERENCES: ShellPreferences = {
  sessionsWidth: 240, inspectorWidth: 320, sessionsVisible: true, density: "low",
};

type Result = { ok: boolean };
type Dependencies = {
  read: () => Promise<{ preferences: ShellPreferences; diagnostic: unknown }>;
  save: (preferences: ShellPreferences) => Promise<Result>;
  reset: () => Promise<Result>;
  onChange: (preferences: ShellPreferences) => void;
  onError: () => void;
  schedule?: (callback: () => void) => () => void;
};

/** Owns only presentation intents; Electron owns the revisioned persistent store. */
export function createShellPreferencesController(deps: Dependencies) {
  let preferences = { ...DEFAULT_SHELL_PREFERENCES };
  let version = 0;
  let disposed = false;
  let cancelSave: (() => void) | null = null;
  let writes = Promise.resolve();
  let loading: Promise<void> | null = null;
  const schedule = deps.schedule ?? ((callback: () => void) => {
    const timer = setTimeout(callback, 500);
    return () => clearTimeout(timer);
  });
  const report = async (operation: () => Promise<Result>) => {
    try { if (!(await operation()).ok && !disposed) deps.onError(); }
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
      if (snapshot.diagnostic) deps.onError();
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
      if (disposed) return;
      ++version;
      cancelSave?.();
      cancelSave = null;
      preferences = { ...DEFAULT_SHELL_PREFERENCES };
      deps.onChange(preferences);
      // Reset is an ordering barrier even when a newer resize is already queued.
      writes = writes.then(async () => {
        await load();
        if (!disposed) await report(deps.reset);
      });
      await writes;
    },
    dispose() { disposed = true; cancelSave?.(); cancelSave = null; },
  };
}
