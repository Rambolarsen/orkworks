export interface RendererHealthSample {
  capturedAt: number;
  usedJSHeapSize?: number;
  totalJSHeapSize?: number;
  jsHeapSizeLimit?: number;
  liveTerminalCount: number;
  liveTerminalIds: readonly string[];
  historicalTerminalNodeCount: number;
  panelCount: number;
}

export interface RendererHealthProbeDeps {
  panelCountProvider: () => number;
  liveTerminalCountProvider: () => number;
  liveTerminalIdsProvider: () => readonly string[];
}

function readMemory(): { usedJSHeapSize?: number; totalJSHeapSize?: number; jsHeapSizeLimit?: number } {
  try {
    const mem = (globalThis as { performance?: { memory?: { usedJSHeapSize: number; totalJSHeapSize: number; jsHeapSizeLimit: number } } }).performance?.memory;
    if (!mem) return {};
    return {
      usedJSHeapSize: mem.usedJSHeapSize,
      totalJSHeapSize: mem.totalJSHeapSize,
      jsHeapSizeLimit: mem.jsHeapSizeLimit,
    };
  } catch {
    return {};
  }
}

function countHistoricalTerminalNodes(): number {
  try {
    const doc = globalThis as { document?: Document };
    if (!doc.document) return 0;
    const shells = doc.document.querySelectorAll(".terminal-shell");
    let count = 0;
    shells.forEach((shell) => {
      const el = shell as Element;
      count += el.querySelectorAll(".xterm").length;
    });
    return count;
  } catch {
    return 0;
  }
}

export function captureRendererHealth(deps: RendererHealthProbeDeps): RendererHealthSample {
  return {
    capturedAt: Date.now(),
    ...readMemory(),
    liveTerminalCount: deps.liveTerminalCountProvider(),
    liveTerminalIds: deps.liveTerminalIdsProvider(),
    historicalTerminalNodeCount: countHistoricalTerminalNodes(),
    panelCount: deps.panelCountProvider(),
  };
}