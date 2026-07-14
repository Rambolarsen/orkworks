import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { WebglAddon } from "@xterm/addon-webgl";
import { terminalPtySize } from "./terminalSize";
import { orkworksTerminalTheme } from "./terminalTheme";
import { getTerminalOutput } from "./api";
import {
  parseTerminalControlMessage,
  shouldReplayTerminalOutputOnClose,
  appendPendingInput,
  canSendTerminalInput,
} from "./terminalProtocol";

const MAX_PENDING_INPUT_LENGTH = 64 * 1024;

export interface TerminalHandle {
  id: string;
  terminal: Terminal;
  ws: WebSocket;
  fitAddon: FitAddon;
  wrapper: HTMLDivElement;
  ended: boolean;
  disposed: boolean;
  pendingInput: string;
  pendingInputOverflowed: boolean;
  resizeObserver: ResizeObserver;
}

const terminals = new Map<string, TerminalHandle>();

function sendResize(ws: WebSocket, term: Terminal): void {
  if (ws.readyState !== WebSocket.OPEN) return;
  ws.send(
    JSON.stringify({
      type: "resize",
      ...terminalPtySize({ rows: term.rows, cols: term.cols }),
    }),
  );
}

export function getTerminal(id: string): TerminalHandle | undefined {
  return terminals.get(id);
}

export function ensureTerminal(id: string, baseUrl: string): TerminalHandle {
  const existing = terminals.get(id);
  if (existing) return existing;

  const term = new Terminal({
    cursorBlink: true,
    fontSize: 14,
    fontFamily: "'Menlo', 'Monaco', 'Courier New', monospace",
    theme: orkworksTerminalTheme,
    allowProposedApi: true,
    scrollback: 2000,
    scrollSensitivity: 3,
    fastScrollSensitivity: 10,
    overviewRuler: { width: 8 },
  });

  const fitAddon = new FitAddon();
  term.loadAddon(fitAddon);

  try {
    const webglAddon = new WebglAddon();
    term.loadAddon(webglAddon);
  } catch {
    // WebGL unavailable, canvas renderer fallback
  }

  term.attachCustomKeyEventHandler((event) => {
    if (event.type !== "keydown") return true;
    const mod = event.metaKey || event.ctrlKey;
    if (mod && event.shiftKey && event.key.length === 1) return false;
    if (mod && event.key === "n" && !event.shiftKey && !event.altKey) return false;
    return true;
  });

  const wrapper = document.createElement("div");
  wrapper.dataset.sessionId = id;
  wrapper.style.cssText = "position:absolute;inset:0;visibility:hidden;";

  const wsUrl = baseUrl.replace("http", "ws") + `/sessions/${id}/terminal`;
  const ws = new WebSocket(wsUrl);
  ws.binaryType = "arraybuffer";

  let fitRaf: number | null = null;
  const resizeObserver = new ResizeObserver(() => {
    if (fitRaf !== null) cancelAnimationFrame(fitRaf);
    fitRaf = requestAnimationFrame(() => {
      fitRaf = null;
      try {
        fitAddon.fit();
      } catch (err) {
        console.warn("[terminalStore] fit() failed for session", id, err);
      }
    });
  });
  resizeObserver.observe(wrapper);

  const handle: TerminalHandle = {
    id,
    terminal: term,
    ws,
    fitAddon,
    wrapper,
    ended: false,
    disposed: false,
    pendingInput: "",
    pendingInputOverflowed: false,
    resizeObserver,
  };
  terminals.set(id, handle);

  let receivedData = false;

  ws.onopen = () => {
    try {
      fitAddon.fit();
    } catch {
      /* ignore */
    }
    sendResize(ws, term);
    if (handle.pendingInput) {
      ws.send(JSON.stringify({ type: "input", data: handle.pendingInput }));
      handle.pendingInput = "";
    }
    handle.pendingInputOverflowed = false;
  };

  ws.onmessage = (e) => {
    if (e.data instanceof ArrayBuffer) {
      receivedData = true;
      term.write(new Uint8Array(e.data));
      return;
    }

    if (typeof e.data !== "string" || handle.disposed) {
      return;
    }

    const message = parseTerminalControlMessage(e.data);
    switch (message?.type) {
      case "replay-start":
      case "replay-end":
        break;
      case "ended":
        term.options.disableStdin = true;
        term.options.cursorBlink = false;
        handle.ended = true;
        break;
      case "error":
        term.options.disableStdin = true;
        term.options.cursorBlink = false;
        handle.ended = true;
        term.writeln(`\r\n[terminal error: ${message.code}] ${message.message}`);
        break;
      case "terminal-unavailable":
        term.options.disableStdin = true;
        term.options.cursorBlink = false;
        term.writeln(`\r\n[terminal unavailable: ${message.reason}]`);
        break;
      default:
        break;
    }
  };

  ws.onclose = () => {
    if (handle.disposed) {
      return;
    }
    term.options.disableStdin = true;
    term.options.cursorBlink = false;
    if (
      shouldReplayTerminalOutputOnClose({
        disposed: handle.disposed,
        receivedData,
      })
    ) {
      getTerminalOutput(baseUrl, id).then((lines) => {
        for (const line of lines) term.writeln(line);
      }).catch(() => {
        /* silently ignore fetch failures */
      });
    }
  };

  term.onData((data) => {
    if (ws.readyState === WebSocket.OPEN) {
      const payload = JSON.stringify({ type: "input", data });
      if (canSendTerminalInput(
        ws.bufferedAmount,
        new TextEncoder().encode(payload).byteLength,
        MAX_PENDING_INPUT_LENGTH,
      )) {
        ws.send(payload);
        return;
      }
      if (!handle.pendingInputOverflowed) {
        handle.pendingInputOverflowed = true;
        term.writeln(
          "\r\n[input buffer full — further keystrokes are being dropped until it drains]",
        );
      }
      return;
    }
    const { next, dropped } = appendPendingInput(
      handle.pendingInput,
      data,
      MAX_PENDING_INPUT_LENGTH,
    );
    handle.pendingInput = next;
    if (dropped && !handle.pendingInputOverflowed) {
      handle.pendingInputOverflowed = true;
      term.writeln(
        "\r\n[input buffer full while disconnected — further keystrokes are being dropped until reconnect]",
      );
    }
  });

  term.onResize(() => sendResize(ws, term));

  return handle;
}

export function disposeTerminal(id: string): void {
  const handle = terminals.get(id);
  if (!handle) return;
  handle.disposed = true;
  handle.resizeObserver.disconnect();
  try {
    handle.ws.close();
  } catch {
    /* ignore */
  }
  try {
    handle.terminal.dispose();
  } catch {
    /* ignore */
  }
  handle.wrapper.remove();
  terminals.delete(id);
}

export function pruneTerminals(keepLiveSessionIds: ReadonlySet<string>): void {
  for (const id of [...terminals.keys()]) {
    if (!keepLiveSessionIds.has(id)) disposeTerminal(id);
  }
}

export function disposeAllTerminals(): void {
  for (const id of [...terminals.keys()]) disposeTerminal(id);
}

if (typeof window !== "undefined") {
  window.addEventListener("beforeunload", disposeAllTerminals);
}
