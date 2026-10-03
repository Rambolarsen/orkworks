import type { Terminal, IDisposable } from "@xterm/xterm";

type TrackpadEvent = Pick<WheelEvent,
  "deltaY" | "deltaMode" | "timeStamp" | "altKey" | "ctrlKey" | "shiftKey" | "metaKey"
>;

const GESTURE_PAUSE_MS = 150;

/** Coalesce pixel packets without delaying the first movement of a gesture. */
export function createTrackpadWheelFilter() {
  let direction = 0;
  let modifiers = 0;
  let lastAt = -Infinity;
  let pixels = 0;

  const reset = () => {
    direction = 0;
    modifiers = 0;
    lastAt = -Infinity;
    pixels = 0;
  };

  return {
    reset,
    filter(event: TrackpadEvent, alternate: boolean, rowHeight: number): boolean {
      if (!alternate || event.deltaMode !== 0) {
        reset();
        return true;
      }
      if (event.deltaY === 0) return true;
      if (!Number.isFinite(event.deltaY)) {
        reset();
        return false;
      }
      const nextDirection = Math.sign(event.deltaY);
      const nextModifiers = Number(event.altKey) | (Number(event.ctrlKey) << 1)
        | (Number(event.shiftKey) << 2) | (Number(event.metaKey) << 3);
      const startsGesture = nextDirection !== direction || nextModifiers !== modifiers
        || event.timeStamp - lastAt >= GESTURE_PAUSE_MS || event.timeStamp < lastAt;
      direction = nextDirection;
      modifiers = nextModifiers;
      lastAt = event.timeStamp;
      if (startsGesture) {
        pixels = 0;
        return true;
      }

      const threshold = Number.isFinite(rowHeight) && rowHeight > 0 ? rowHeight : 16;
      pixels += Math.abs(event.deltaY);
      if (pixels < threshold) return false;
      // xterm emits one direction command per event, even for a large delta.
      // Keep only the fractional remainder so large packets cannot build debt.
      pixels %= threshold;
      return true;
    },
  };
}

/** Install through an addon so the buffer subscription follows terminal lifetime. */
export function installTerminalWheelHandler(term: Terminal): void {
  const wheel = createTrackpadWheelFilter();
  let bufferChange: IDisposable | undefined;
  term.loadAddon({
    activate() {
      bufferChange = term.buffer.onBufferChange(wheel.reset);
      term.attachCustomWheelEventHandler(event => {
        const alternate = term.buffer.active.type === "alternate";
        const screenHeight = alternate && event.deltaMode === 0
          ? term.element?.querySelector<HTMLElement>(".xterm-screen")?.clientHeight ?? 0
          : 0;
        const rowHeight = screenHeight > 0 ? screenHeight / term.rows
          : (term.options.fontSize ?? 14) * (term.options.lineHeight ?? 1);
        if (wheel.filter(event, alternate, rowHeight)) return true;
        event.preventDefault();
        event.stopPropagation();
        return false;
      });
    },
    dispose() {
      bufferChange?.dispose();
    },
  });
}
