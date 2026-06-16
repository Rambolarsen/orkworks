const MIN_PTY_COLS = 80;
const MIN_PTY_ROWS = 24;

interface TerminalSize {
  rows: number;
  cols: number;
}

export function terminalPtySize(size: TerminalSize): TerminalSize {
  return {
    rows: Math.max(MIN_PTY_ROWS, size.rows),
    cols: Math.max(MIN_PTY_COLS, size.cols),
  };
}
