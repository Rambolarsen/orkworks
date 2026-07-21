export interface PollScheduler {
  set(callback: () => void, delayMs: number): unknown;
  clear(handle: unknown): void;
}

const browserScheduler: PollScheduler = {
  set: (callback, delayMs) => window.setTimeout(callback, delayMs),
  clear: (handle) => window.clearTimeout(handle as number),
};

export function startSessionPolling(
  refresh: () => Promise<void>,
  delayMs = 2_000,
  scheduler: PollScheduler = browserScheduler,
): () => void {
  let stopped = false;
  let timer: unknown;

  async function poll(): Promise<void> {
    try {
      await refresh();
    } catch {
      // Background refresh failures remain silent; retry on the next cycle.
    }
    if (!stopped) {
      timer = scheduler.set(() => void poll(), delayMs);
    }
  }

  void poll();
  return () => {
    stopped = true;
    if (timer !== undefined) scheduler.clear(timer);
  };
}
