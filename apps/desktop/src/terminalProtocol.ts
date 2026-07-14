export type TerminalControlMessage =
  | { type: "replay-start"; cursor: number }
  | { type: "replay-end"; cursor: number }
  | { type: "ended"; status: string }
  | { type: "error"; code: string; message: string }
  | { type: "terminal-unavailable"; reason: string };

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

export function shouldReplayTerminalOutputOnClose({
  disposed,
  receivedData,
}: {
  disposed: boolean;
  receivedData: boolean;
}): boolean {
  return !disposed && !receivedData;
}
