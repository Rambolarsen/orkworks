import { useCallback, useEffect, useRef, useState } from "react";
import type { ShellPreferences } from "./orkworksWindow";
import { createShellPreferencesController, DEFAULT_SHELL_PREFERENCES } from "./shellPreferencesController";
import { pushToast } from "./feedback";

export function useShellPreferences() {
  const [preferences, setPreferences] = useState(DEFAULT_SHELL_PREFERENCES);
  const controllerRef = useRef<ReturnType<typeof createShellPreferencesController> | null>(null);
  useEffect(() => {
    const controller = createShellPreferencesController({
      read: () => window.orkworks.getShellLayout(),
      save: value => window.orkworks.saveShellLayout(value),
      reset: () => window.orkworks.resetShellLayout(),
      onChange: setPreferences,
      onError: () => pushToast("info", "Shell preferences could not be saved or restored. The current view is still available."),
    });
    controllerRef.current = controller;
    void controller.load();
    return () => { controller.dispose(); if (controllerRef.current === controller) controllerRef.current = null; };
  }, []);
  const change = useCallback((next: ShellPreferences) => controllerRef.current?.change(next), []);
  const reset = useCallback(() => { void controllerRef.current?.reset(); }, []);
  return { preferences, change, reset };
}
