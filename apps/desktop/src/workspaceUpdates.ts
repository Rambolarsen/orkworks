import type { SessionInfo } from "./api.ts";
import type { PollScheduler } from "./sessionPolling.ts";

interface Result { sessions?: SessionInfo[]; recommendationsInvalidated?: boolean }
const invalid = (): never => { throw new Error("Workspace update requires resynchronization"); };
const record = (value: unknown): Record<string, unknown> => {
  if (!value || typeof value !== "object" || Array.isArray(value)) return invalid();
  return value as Record<string, unknown>;
};
function rows(value: unknown): SessionInfo[] {
  if (!Array.isArray(value)) return invalid();
  const ids = new Set<string>();
  for (const valueRow of value) {
    const row = record(valueRow);
    for (const field of ["id", "label", "cwd", "created_at", "status", "lifecycle", "memoryState", "resumeStrategy"]) {
      if (typeof row[field] !== "string") return invalid();
    }
    if (!row.id || ids.has(row.id as string) || !["creating", "alive", "stopping", "dead"].includes(row.lifecycle as string)) return invalid();
    ids.add(row.id as string);
  }
  return value as SessionInfo[];
}
function ids(value: unknown): string[] {
  if (!Array.isArray(value) || value.some(id => typeof id !== "string" || !id) || new Set(value).size !== value.length) return invalid();
  return value;
}

/** Validate a whole message before committing either its baseline or cursor. */
export function createStreamState(workspacePath: string) {
  let identity: { instance: string; generation: number } | null = null;
  let sequence = -1;
  let baseline: SessionInfo[] = [];
  return { apply(input: unknown): Result {
    const message = record(input);
    if (message.version !== 1 || message.workspace !== workspacePath || typeof message.instance !== "string"
      || !Number.isSafeInteger(message.generation) || (message.generation as number) < 0
      || !Number.isSafeInteger(message.sequence) || (message.sequence as number) < 0) return invalid();
    if (message.type === "snapshot") {
      if (identity) return invalid();
      const next = rows(message.sessions);
      identity = { instance: message.instance, generation: message.generation as number };
      sequence = message.sequence as number; baseline = next;
      return { sessions: next, recommendationsInvalidated: true };
    }
    if (!identity || message.instance !== identity.instance || message.generation !== identity.generation
      || message.base !== sequence || message.sequence !== sequence + 1) return invalid();
    if (message.type === "recommendations-invalidated") {
      sequence++; return { recommendationsInvalidated: true };
    }
    if (message.type !== "sessions-delta") return invalid();
    const changed = rows(message.changed), removed = ids(message.removed);
    const next = new Map(baseline.map(row => [row.id, row]));
    for (const id of removed) { if (!next.delete(id)) return invalid(); }
    for (const row of changed) { if (removed.includes(row.id)) return invalid(); next.set(row.id, row); }
    const order = message.order === undefined ? [...next.keys()] : ids(message.order);
    if (order.length !== next.size || order.some(id => !next.has(id))) return invalid();
    const result = order.map(id => next.get(id)!);
    baseline = result; sequence++;
    return { sessions: result };
  } };
}

interface Options {
  workspacePath: string;
  getBackendUrl(): Promise<string>;
  onSessions(rows: SessionInfo[]): void;
  onHealthy(healthy: boolean): void;
  onRecommendationsInvalidated(): void;
  createSocket?: (url: string) => WebSocket;
  scheduler?: PollScheduler;
}

export function startWorkspaceUpdates(options: Options): { dispose(): void; resync(): void } {
  const scheduler = options.scheduler ?? { set: (fn: () => void, ms: number) => setTimeout(fn, ms), clear: (id: unknown) => clearTimeout(id as ReturnType<typeof setTimeout>) };
  let disposed = false, healthy = false, epoch = 0, attempt = 0;
  let socket: WebSocket | null = null;
  let retry: unknown, heartbeat: unknown;
  const setHealthy = (next: boolean) => { if (next !== healthy) { healthy = next; options.onHealthy(next); } };
  options.onHealthy(false);

  function fail(current: number) {
    if (disposed || current !== epoch) return;
    epoch++;
    scheduler.clear(heartbeat);
    const old = socket; socket = null;
    if (old) { old.onmessage = null; old.onerror = null; old.onclose = null; old.close(); }
    setHealthy(false);
    const delay = Math.min(30_000, 1000 * 2 ** Math.min(attempt++, 5));
    retry = scheduler.set(() => { void connect(); }, delay * (0.8 + Math.random() * 0.2));
  }
  function armHeartbeat(current: number) {
    scheduler.clear(heartbeat);
    heartbeat = scheduler.set(() => fail(current), 30_000);
  }
  async function connect() {
    const current = ++epoch;
    try {
      const base = await options.getBackendUrl();
      if (disposed || current !== epoch) return;
      const url = new URL("/workspace/updates", base); url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
      url.searchParams.set("workspace", options.workspacePath);
      const candidate = (options.createSocket ?? (url => new WebSocket(url)))(url.toString());
      socket = candidate;
      const state = createStreamState(options.workspacePath);
      armHeartbeat(current);
      candidate.onmessage = event => {
        if (disposed || current !== epoch) return;
        try {
          const message: unknown = JSON.parse(String(event.data));
          if (record(message).type === "heartbeat") { armHeartbeat(current); return; }
          const result = state.apply(message);
          // Fence an older REST query before publishing the first stream snapshot.
          if (!healthy) { setHealthy(true); attempt = 0; }
          if (result.sessions) options.onSessions(result.sessions);
          if (result.recommendationsInvalidated) options.onRecommendationsInvalidated();
          armHeartbeat(current);
        } catch { fail(current); }
      };
      candidate.onerror = () => fail(current);
      candidate.onclose = () => fail(current);
    } catch { fail(current); }
  }
  const focus = () => { if (!disposed && healthy && socket?.readyState === 1) socket.send("refresh"); };
  if (typeof window !== "undefined") window.addEventListener("focus", focus);
  void connect();
  return {
    dispose() {
      if (disposed) return;
      if (typeof window !== "undefined") window.removeEventListener("focus", focus);
      disposed = true; epoch++; scheduler.clear(retry); scheduler.clear(heartbeat);
      if (socket) { socket.onmessage = null; socket.onclose = null; socket.onerror = null; socket.close(); socket = null; }
      setHealthy(false);
    },
    resync() {
      // Explicit REST mutation/refresh invalidates the old delta baseline.
      if (disposed) return;
      scheduler.clear(retry);
      const current = epoch; fail(current);
      scheduler.clear(retry); attempt = 0; void connect();
    },
  };
}
