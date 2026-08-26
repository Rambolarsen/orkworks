export type BackendLifecycleEvent =
  | { state: "starting" | "retrying" }
  | { state: "ready"; port: number }
  | { state: "failed" | "exhausted"; message: string };

function hasExactKeys(value: object, expected: readonly string[]): boolean {
  const keys = Reflect.ownKeys(value);
  return keys.length === expected.length && expected.every((key) => keys.includes(key));
}

export function canonicalizeBackendLifecycleEvent(data: unknown): BackendLifecycleEvent | null {
  if (!data || typeof data !== "object" || Array.isArray(data)) return null;

  try {
    const event = data as Record<string, unknown>;
    const state = event.state;
    if (state === "starting" || state === "retrying") {
      return hasExactKeys(data, ["state"]) ? { state } : null;
    }
    if (state === "ready") {
      const port = event.port;
      return hasExactKeys(data, ["state", "port"])
        && typeof port === "number"
        && Number.isInteger(port)
        && port >= 1
        && port <= 65_535
        ? { state: "ready", port }
        : null;
    }
    if (state === "failed" || state === "exhausted") {
      const message = event.message;
      return hasExactKeys(data, ["state", "message"]) && typeof message === "string"
        ? { state, message }
        : null;
    }
  } catch {
    return null;
  }

  return null;
}

export function subscribeBackendLifecycle(
  registerLive: (listener: (data: unknown) => void) => () => void,
  loadSnapshot: () => Promise<unknown>,
  callback: (event: BackendLifecycleEvent) => void,
): () => void {
  let active = true;
  let receivedLiveEvent = false;
  const unregisterLive = registerLive((data) => {
    if (!active) return;
    const event = canonicalizeBackendLifecycleEvent(data);
    if (!event) return;
    receivedLiveEvent = true;
    callback(event);
  });

  void Promise.resolve()
    .then(loadSnapshot)
    .then((data) => {
      if (!active || receivedLiveEvent) return;
      const event = canonicalizeBackendLifecycleEvent(data);
      if (event) callback(event);
    })
    .catch(() => {
      // A live event can still arrive after snapshot retrieval fails.
    });

  return () => {
    if (!active) return;
    active = false;
    unregisterLive();
  };
}
