import type { RecommendationListResponse } from "./api.ts";
import type { ActionsSourceStatus } from "./actionsOverview.ts";
import type { PollScheduler } from "./sessionPolling.ts";

export interface RecommendationSnapshot {
  response: RecommendationListResponse;
  status: ActionsSourceStatus;
  error?: string;
  revision: number;
}
export function createRecommendationSource(options: { load(): Promise<RecommendationListResponse>; scheduler?: PollScheduler }) {
  const scheduler = options.scheduler ?? { set: (fn: () => void, ms: number) => setTimeout(fn,ms), clear: (id: unknown) => clearTimeout(id as ReturnType<typeof setTimeout>) };
  let snapshot: RecommendationSnapshot = { response: { recommendations: [], diagnostics: [] }, status: "loading", revision: 0 };
  const listeners = new Set<() => void>();
  let healthy = false, active = false, dirty = false, generation = 0;
  let pending: Promise<RecommendationListResponse> | null = null;
  let timer: unknown;
  const emit = () => { if (active) listeners.forEach(listener => listener()); };
  function schedule() {
    scheduler.clear(timer); timer = undefined;
    if (active && !healthy && !pending) timer = scheduler.set(() => { void refresh().catch(() => {}); }, 5000);
  }
  function refresh(): Promise<RecommendationListResponse> {
    if (pending) return pending;
    scheduler.clear(timer); timer = undefined;
    const current = generation;
    // Establish the shared promise before notifying consumers that a refresh began.
    pending = Promise.resolve().then(options.load).then(response => {
      if (active && current === generation) {
        snapshot = { response, revision: snapshot.revision, status: response.diagnostics.length ? "partial" : "ready" }; emit();
      }
      return response;
    }, (error: unknown) => {
      if (active && current === generation) {
        snapshot = { ...snapshot, status: "unavailable", error: error instanceof Error ? error.message : "Couldn't load recommendations." }; emit();
      }
      throw error;
    }).finally(() => {
      if (current !== generation) return;
      pending = null;
      if (active && dirty) { dirty = false; void refresh().catch(() => {}); } else schedule();
    });
    snapshot = { ...snapshot, revision: snapshot.revision + 1 }; emit();
    return pending;
  }
  return {
    getSnapshot: () => snapshot,
    refresh,
    invalidate() { if (!active) return; if (pending) dirty = true; else void refresh().catch(() => {}); },
    setHealthy(next: boolean) { if (healthy === next) return; healthy = next; schedule(); },
    subscribe(listener: () => void) {
      listeners.add(listener);
      if (!active) { active = true; void refresh().catch(() => {}); }
      return () => {
        listeners.delete(listener);
        if (!listeners.size) { active = false; generation++; pending = null; dirty = false; scheduler.clear(timer); }
      };
    },
  };
}
