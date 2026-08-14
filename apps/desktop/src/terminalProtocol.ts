/**
 * DEC private-mode-off sequences for the mouse-tracking variants a terminal
 * program can enable (X10 1000, button-event 1002, any-event 1003, UTF-8 1005,
 * SGR 1006, urxvt 1015, SGR-pixels 1016). Fed directly into xterm.js's local
 * parser (never sent over the wire) to clear its internal tracking state when
 * OrkWorks ends a session's terminal, in case the terminated process enabled
 * mouse tracking for its own TUI and never got to turn it back off.
 */
export const MOUSE_TRACKING_RESET_SEQUENCE =
  "\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1005l\x1b[?1006l\x1b[?1015l\x1b[?1016l";

export type TerminalControlMessage =
  | { type: "replay-start"; cursor: number }
  | { type: "replay-end"; cursor: number }
  | { type: "ended"; status: string }
  | { type: "error"; code: string; message: string }
  | { type: "terminal-unavailable"; reason: string }
  | { type: "input-dropped" };

export function parseTerminalControlMessage(
  raw: string,
): TerminalControlMessage | null {
  try {
    const value = JSON.parse(raw) as Record<string, unknown>;
    switch (value.type) {
      case "replay-start":
      case "replay-end":
        if (typeof value.cursor === "number") {
          return { type: value.type, cursor: value.cursor };
        }
        return null;
      case "ended":
        if (typeof value.status === "string") {
          return { type: "ended", status: value.status };
        }
        return null;
      case "error":
        if (typeof value.code === "string" && typeof value.message === "string") {
          return { type: "error", code: value.code, message: value.message };
        }
        return null;
      case "terminal-unavailable":
        if (typeof value.reason === "string") {
          return { type: "terminal-unavailable", reason: value.reason };
        }
        return null;
      case "input-dropped":
        return { type: "input-dropped" };
      default:
        return null;
    }
  } catch {
    return null;
  }
}

export function appendPendingInput(
  current: string,
  incoming: string,
  maxLength: number,
): { next: string; dropped: boolean } {
  if (current.length + incoming.length > maxLength) {
    return { next: current, dropped: true };
  }
  return { next: current + incoming, dropped: false };
}

export function canSendTerminalInput(
  bufferedAmount: number,
  payloadLength: number,
  maxBufferedAmount: number,
): boolean {
  return bufferedAmount + payloadLength <= maxBufferedAmount;
}

export function shouldReplayTerminalOutputOnClose({
  disposed,
  receivedData,
}: {
  disposed: boolean;
  receivedData: boolean;
}): boolean {
  return !disposed && !receivedData;
}
