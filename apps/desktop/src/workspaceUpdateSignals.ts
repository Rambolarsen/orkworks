// Transient transport health/invalidation only; session data belongs to its controller.
export interface WorkspaceUpdateSignal { healthy: boolean; revision: number }
const states = new Map<string, WorkspaceUpdateSignal>();
const listeners = new Map<string, Set<() => void>>();
export function workspaceUpdateSignal(path: string): WorkspaceUpdateSignal {
  return states.get(path) ?? { healthy: false, revision: 0 };
}
export function publishWorkspaceUpdateSignal(path: string, healthy?: boolean): void {
  const previous = workspaceUpdateSignal(path);
  states.set(path, { healthy: healthy ?? previous.healthy, revision: previous.revision + 1 });
  listeners.get(path)?.forEach(listener => listener());
  // No retained history for closed workspaces without recommendation consumers.
  if (healthy === false && !listeners.get(path)?.size) states.delete(path);
}
export function subscribeWorkspaceUpdateSignal(path: string, listener: () => void): () => void {
  const group = listeners.get(path) ?? new Set(); group.add(listener); listeners.set(path, group);
  return () => { group.delete(listener); if (!group.size) { listeners.delete(path); if (!states.get(path)?.healthy) states.delete(path); } };
}
