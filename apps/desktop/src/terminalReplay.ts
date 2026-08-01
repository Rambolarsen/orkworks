export type TerminalReplayResult = "loaded" | "empty" | "error" | "stale";

export interface ReplayTerminal {
  write(text: string): void;
  writeln(line: string): void;
}

export type TerminalReplayRecord = string | { text: string; delimiter: string };

export function writeTerminalReplay(terminal: ReplayTerminal, records: TerminalReplayRecord[]): void {
  for (const record of records) {
    if (typeof record === "string") terminal.writeln(record);
    else terminal.write(record.text + record.delimiter);
  }
}

export async function loadTerminalReplay(
  read: () => Promise<{ lines: TerminalReplayRecord[]; cols?: number; rows?: number }>,
  isCurrent: () => boolean,
  createTerminal: (size: { cols?: number; rows?: number }) => ReplayTerminal,
): Promise<TerminalReplayResult> {
  try {
    const payload = await read();
    if (!isCurrent()) return "stale";
    if (payload.lines.length === 0) return "empty";
    const terminal = createTerminal({ cols: payload.cols, rows: payload.rows });
    writeTerminalReplay(terminal, payload.lines);
    return "loaded";
  } catch {
    return isCurrent() ? "error" : "stale";
  }
}
