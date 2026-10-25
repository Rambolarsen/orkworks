import { useCallback, useEffect, useRef, useState } from "react";
import { getTaskmasterRecommendations, type RecommendationListResponse } from "./api";
import { createRecommendationSource, type RecommendationSnapshot } from "./recommendationSource";
import { subscribeWorkspaceUpdateSignal, workspaceUpdateSignal } from "./workspaceUpdateSignals";

type Source = ReturnType<typeof createRecommendationSource>;
interface Entry { source: Source; users: number; unsubscribeSignal(): void }
const sources = new Map<string, Entry>();
const empty: RecommendationSnapshot = { response: { recommendations: [], diagnostics: [] }, status: "loading", revision: 0 };
function acquire(scope: string, workspacePath: string): Entry {
  let entry = sources.get(scope);
  if (!entry) {
    const source = createRecommendationSource({ load: async () => getTaskmasterRecommendations(await window.orkworks.getBackendUrl()) });
    source.setHealthy(workspaceUpdateSignal(workspacePath).healthy);
    const unsubscribeSignal = subscribeWorkspaceUpdateSignal(workspacePath, () => {
      source.setHealthy(workspaceUpdateSignal(workspacePath).healthy); source.invalidate();
    });
    entry = { source, users: 0, unsubscribeSignal }; sources.set(scope, entry);
  }
  entry.users++; return entry;
}

export interface ActionsRecommendationSnapshot extends RecommendationSnapshot {
  scope: string;
  items: RecommendationListResponse["recommendations"];
  refresh(): Promise<RecommendationListResponse>;
}

export function useActionsRecommendations(scope: string, enabled: boolean, workspacePath = scope): ActionsRecommendationSnapshot {
  const [snapshot, setSnapshot] = useState<RecommendationSnapshot & { scope: string }>({ ...empty, scope: "" });
  const sourceRef = useRef<{ scope: string; source: Source } | null>(null);
  useEffect(() => {
    if (!enabled) return;
    let cancelled = false;
    const entry = acquire(scope,workspacePath);
    sourceRef.current = { scope, source: entry.source };
    const update = () => { if (!cancelled) setSnapshot({ ...entry.source.getSnapshot(), scope }); };
    const unsubscribe = entry.source.subscribe(update); update();
    return () => {
      cancelled = true; unsubscribe(); sourceRef.current = null;
      if (--entry.users === 0) { entry.unsubscribeSignal(); sources.delete(scope); }
    };
  }, [scope,enabled,workspacePath]);
  const refresh = useCallback(() => {
    const current = sourceRef.current;
    if (!enabled || !current || current.scope !== scope) return Promise.reject(new Error("Workspace recommendations unavailable"));
    return current.source.refresh();
  }, [scope,enabled]);
  // Fast Refresh can retain the old items-only state until the subscription effect reruns.
  const current = enabled && snapshot.scope === scope && snapshot.response ? snapshot : { ...empty, status: enabled ? "loading" as const : "unavailable" as const };
  return { ...current, scope, items: current.response.recommendations, refresh };
}
