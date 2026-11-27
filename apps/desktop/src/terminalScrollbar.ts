import type { Terminal } from "@xterm/xterm";

/** Track overflow so app styles can keep the thumb visible when scrolling is possible. */
export function markTerminalScrollback(terminal: Terminal): void {
  terminal.onRender(() => {
    terminal.element?.classList.toggle("has-scrollback", terminal.buffer.active.baseY > 0);
  });
}
