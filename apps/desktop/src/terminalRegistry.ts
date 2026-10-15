export interface TerminalRegistry<T> {
  get(id: string): T | undefined;
  set(id: string, handle: T): void;
  remove(id: string): T | undefined;
  prune(keep: ReadonlySet<string>): T[];
  liveIds(): readonly string[];
  readonly size: number;
}

export function createTerminalRegistry<T>(): TerminalRegistry<T> {
  const handles = new Map<string, T>();
  return {
    get(id) {
      return handles.get(id);
    },
    set(id, handle) {
      handles.set(id, handle);
    },
    remove(id) {
      const h = handles.get(id);
      if (h === undefined) return undefined;
      handles.delete(id);
      return h;
    },
    prune(keep) {
      const removed: T[] = [];
      for (const [id, handle] of handles) {
        if (!keep.has(id)) {
          handles.delete(id);
          removed.push(handle);
        }
      }
      return removed;
    },
    liveIds() {
      return [...handles.keys()];
    },
    get size() {
      return handles.size;
    },
  };
}
