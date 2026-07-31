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
  read: () => Promise<TerminalReplayRecord[]>,
  isCurrent: () => boolean,
  createTerminal: () => ReplayTerminal,
): Promise<TerminalReplayResult> {
  try {
    const records = await read();
    if (!isCurrent()) return "stale";
    if (records.length === 0) return "empty";
    const terminal = createTerminal();
    writeTerminalReplay(terminal, records);
    return "loaded";
  } catch {
    return isCurrent() ? "error" : "stale";
  }
}
