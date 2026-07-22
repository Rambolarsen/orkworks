export type TerminalReplayResult = "loaded" | "empty" | "error" | "stale";

export interface ReplayTerminal {
  writeln(line: string): void;
}

export async function loadTerminalReplay(
  read: () => Promise<string[]>,
  isCurrent: () => boolean,
  createTerminal: () => ReplayTerminal,
): Promise<TerminalReplayResult> {
  try {
    const lines = await read();
    if (!isCurrent()) return "stale";
    if (lines.length === 0) return "empty";
    const terminal = createTerminal();
    for (const line of lines) terminal.writeln(line);
    return "loaded";
  } catch {
    return isCurrent() ? "error" : "stale";
  }
}
