import { useEffect, useState } from "react";
import { getTaskmasterRecommendations, type WorkflowRecommendation } from "./api";
import type { ActionsSourceStatus } from "./actionsOverview";

interface Snapshot {
  scope: string;
  status: ActionsSourceStatus;
  items: WorkflowRecommendation[];
}

export function useActionsRecommendations(scope: string, enabled: boolean): Snapshot {
  const [snapshot, setSnapshot] = useState<Snapshot>({ scope: "", status: "loading", items: [] });
  useEffect(() => {
    let cancelled = false;
    let timer: ReturnType<typeof setTimeout>;
    setSnapshot({ scope, status: enabled ? "loading" : "unavailable", items: [] });
    if (!enabled) return;

    const refresh = async () => {
      try {
        const base = await window.orkworks.getBackendUrl();
        if (cancelled) return;
        const response = await getTaskmasterRecommendations(base);
        if (cancelled) return;
        setSnapshot({
          scope,
          status: response.diagnostics.length ? "partial" : "ready",
          items: response.recommendations,
        });
      } catch {
        if (!cancelled) {
          setSnapshot(old => ({ scope, status: "unavailable", items: old.scope === scope ? old.items : [] }));
        }
      } finally {
        // Schedule after completion so slow requests never overlap or reorder snapshots.
        if (!cancelled) timer = setTimeout(refresh, 5000);
      }
    };
    void refresh();
    return () => { cancelled = true; clearTimeout(timer); };
  }, [scope, enabled]);

  // Fence the render too: an effect cleanup alone leaves one render of the old scope.
  return snapshot.scope === scope && enabled
    ? snapshot
    : { scope, status: enabled ? "loading" : "unavailable", items: [] };
}
