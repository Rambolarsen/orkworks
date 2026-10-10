import { useCallback, useEffect, useRef, useState } from "react";
import type { ShellMemoryDiagnostic, ShellPreferences } from "./orkworksWindow";
import { createShellPreferencesController, DEFAULT_SHELL_PREFERENCES } from "./shellPreferencesController";
import { pushToast } from "./feedback";

export function useShellPreferences() {
  const [preferences, setPreferences] = useState(DEFAULT_SHELL_PREFERENCES);
  const [diagnostic, setDiagnostic] = useState<ShellMemoryDiagnostic | null>(null);
  const [migrationNotice, setMigrationNotice] = useState(false);
  const controllerRef = useRef<ReturnType<typeof createShellPreferencesController> | null>(null);
  useEffect(() => {
    const controller = createShellPreferencesController({
      read: () => window.orkworks.getShellLayout(),
      save: value => window.orkworks.saveShellLayout(value),
      reset: () => window.orkworks.resetShellLayout(),
      rebuild: () => window.orkworks.rebuildShellLayout(),
      readLegacy: () => window.orkworks.getLayout(),
      onMigrationNotice: () => setMigrationNotice(true),
      onDiagnostic: setDiagnostic,
      onChange: setPreferences,
      onError: () => pushToast("info", "Shell preferences could not be saved or restored. The current view is still available."),
    });
    controllerRef.current = controller;
    void controller.load();
    return () => { controller.dispose(); if (controllerRef.current === controller) controllerRef.current = null; };
  }, []);
  const change = useCallback((next: ShellPreferences) => controllerRef.current?.change(next), []);
  const reset = useCallback(() => controllerRef.current?.reset() ?? Promise.resolve(false), []);
  const dismissMigrationNotice = () => {
    setMigrationNotice(false);
    // Explicit acknowledgement advances the existing preference revision; hydration never writes.
    controllerRef.current?.change(preferences);
  };
  const needsRebuild = diagnostic === "corrupt_record" || diagnostic === "unsupported_version";
  return { preferences, change, reset, needsRebuild, migrationNotice, dismissMigrationNotice };
}
