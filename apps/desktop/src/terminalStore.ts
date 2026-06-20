import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { terminalPtySize } from "./terminalSize";
import { orkworksTerminalTheme } from "./terminalTheme";
import { getTerminalOutput } from "./api";

export interface TerminalHandle {
  id: string;
  terminal: Terminal;
  ws: WebSocket;
  fitAddon: FitAddon;
  wrapper: HTMLDivElement;
  ended: boolean;
  pendingInput: string;
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
  });

  const fitAddon = new FitAddon();
  term.loadAddon(fitAddon);

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

  const handle: TerminalHandle = {
    id,
    terminal: term,
    ws,
    fitAddon,
    wrapper,
    ended: false,
    pendingInput: "",
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
  };

  ws.onmessage = (e) => {
    receivedData = true;
    if (e.data instanceof ArrayBuffer) {
      term.write(new Uint8Array(e.data));
    }
  };

  ws.onclose = () => {
    term.options.disableStdin = true;
    term.options.cursorBlink = false;
    handle.ended = true;
    if (!receivedData) {
      getTerminalOutput(baseUrl, id).then((lines) => {
        if (lines.length > 0) {
          term.writeln(lines.join("\n"));
        }
      }).catch(() => {
        /* silently ignore fetch failures */
      });
    }
  };

  term.onData((data) => {
    if (ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({ type: "input", data }));
    } else {
      handle.pendingInput += data;
    }
  });

  term.onResize(() => sendResize(ws, term));

  return handle;
}

export function disposeTerminal(id: string): void {
  const handle = terminals.get(id);
  if (!handle) return;
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

export function disposeAllTerminals(): void {
  for (const id of [...terminals.keys()]) disposeTerminal(id);
}

if (typeof window !== "undefined") {
  window.addEventListener("beforeunload", disposeAllTerminals);
}
