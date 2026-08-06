export interface TerminalRegistry<T> {
  get(id: string): T | undefined;
  set(id: string, handle: T): void;
  remove(id: string): T | undefined;
  prune(keep: ReadonlySet<string>): T[];
  isDisposed(id: string): boolean;
  liveIds(): readonly string[];
  readonly size: number;
}

export function createTerminalRegistry<T>(): TerminalRegistry<T> {
  const handles = new Map<string, T>();
  const disposed = new Set<string>();
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
      disposed.add(id);
      return h;
    },
    prune(keep) {
      const removed: T[] = [];
      for (const [id, handle] of handles) {
        if (!keep.has(id)) {
          handles.delete(id);
          disposed.add(id);
          removed.push(handle);
        }
      }
      return removed;
    },
    isDisposed(id) {
      return disposed.has(id);
    },
    liveIds() {
      return [...handles.keys()];
    },
    get size() {
      return handles.size;
    },
  };
}