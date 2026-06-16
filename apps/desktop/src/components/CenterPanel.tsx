import { useEffect, useRef, useCallback, useState } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";
import { terminalPtySize } from "../terminalSize";
import { orkworksTerminalTheme } from "../terminalTheme";

interface CenterPanelProps {
  backendStatus: string;
  sessionId: string | null;
}

declare global {
  interface Window {
    orkworks: {
      getBackendUrl: () => Promise<string>;
    };
  }
}

interface TerminalHandle {
  terminal: Terminal;
  ws: WebSocket;
  fitAddon: FitAddon;
}

function CenterPanel({ backendStatus, sessionId }: CenterPanelProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const terminalsRef = useRef<Map<string, TerminalHandle>>(new Map());
  const activeIdRef = useRef<string | null>(null);
  const [terminalStatus, setTerminalStatus] = useState("terminal starting");
  const pendingInputRef = useRef<Map<string, string>>(new Map());

  const sendResize = useCallback((ws: WebSocket, term: Terminal) => {
    if (ws.readyState !== WebSocket.OPEN) return;
    ws.send(
      JSON.stringify({
        type: "resize",
        ...terminalPtySize({ rows: term.rows, cols: term.cols }),
      }),
    );
  }, []);

  const attachTerminal = useCallback    (
    (id: string) => {
      const handle = terminalsRef.current.get(id);
      const container = containerRef.current;
      if (!handle || !container || !handle.terminal.element) return;

      if (activeIdRef.current && activeIdRef.current !== id) {
        const prev = terminalsRef.current.get(activeIdRef.current);
        prev?.terminal.element?.remove();
      }

      container.appendChild(handle.terminal.element);
      activeIdRef.current = id;

      try {
        handle.fitAddon.fit();
      } catch {
        /* ignore */
      }
      handle.terminal.focus();
    },
    [],
  );

  const startSession = useCallback(
    async (id: string, baseUrl: string, cancelled: () => boolean) => {
      const term = new Terminal({
        cursorBlink: true,
        fontSize: 14,
        fontFamily: "'Menlo', 'Monaco', 'Courier New', monospace",
        theme: orkworksTerminalTheme,
        allowProposedApi: true,
      });

      const fitAddon = new FitAddon();
      term.loadAddon(fitAddon);

      const wsUrl = baseUrl.replace("http", "ws") + `/sessions/${id}/terminal`;
      const ws = new WebSocket(wsUrl);
      ws.binaryType = "arraybuffer";

      const handle: TerminalHandle = { terminal: term, ws, fitAddon };
      terminalsRef.current.set(id, handle);

      setTerminalStatus("terminal connecting");

      ws.onopen = () => {
        setTerminalStatus("terminal ready");
        sendResize(ws, term);

        const pending = pendingInputRef.current.get(id);
        if (pending) {
          ws.send(JSON.stringify({ type: "input", data: pending }));
          pendingInputRef.current.delete(id);
        }
      };

      ws.onmessage = (e) => {
        if (e.data instanceof ArrayBuffer) {
          term.write(new Uint8Array(e.data));
        }
      };

      ws.onclose = () => {
        if (cancelled()) return;
        terminalsRef.current.delete(id);
        if (activeIdRef.current === id) {
          activeIdRef.current = null;
        }
        term.dispose();
        setTerminalStatus("terminal disconnected");
      };

      term.onData((data) => {
        if (ws.readyState === WebSocket.OPEN) {
          ws.send(JSON.stringify({ type: "input", data }));
        }
      });

      term.onResize(() => sendResize(ws, term));

      attachTerminal(id);
    },
    [sendResize, attachTerminal],
  );

  useEffect(() => {
    if (backendStatus !== "connected" || !sessionId) return;
    let cancelled = false;

    if (terminalsRef.current.has(sessionId)) {
      attachTerminal(sessionId);
      return;
    }

    setTerminalStatus("terminal starting");

    window.orkworks.getBackendUrl().then((baseUrl) => {
      if (cancelled) return;
      startSession(sessionId, baseUrl, () => cancelled);
    });

    return () => {
      cancelled = true;
    };
  }, [backendStatus, sessionId, startSession, attachTerminal]);

  useEffect(() => {
    const handleWindowResize = () => {
      const active = activeIdRef.current;
      if (!active) return;
      const handle = terminalsRef.current.get(active);
      if (!handle) return;
      try {
        handle.fitAddon.fit();
      } catch {
        /* ignore */
      }
      sendResize(handle.ws, handle.terminal);
    };

    window.addEventListener("resize", handleWindowResize);
    const observer = new ResizeObserver(handleWindowResize);
    if (containerRef.current) observer.observe(containerRef.current);

    return () => {
      window.removeEventListener("resize", handleWindowResize);
      observer.disconnect();
    };
  }, [sendResize]);

  useEffect(() => {
    return () => {
      for (const handle of terminalsRef.current.values()) {
        handle.ws.close();
        handle.terminal.dispose();
      }
      terminalsRef.current.clear();
    };
  }, []);

  if (backendStatus !== "connected") {
    return (
      <div className="center-placeholder">
        <p style={{ color: "#666", fontSize: 14, marginBottom: 8 }}>OrkWorks</p>
        <p style={{ color: "#555", fontSize: 12 }}>
          Mission Control for AI Agents
        </p>
        <p style={{ color: "#444", fontSize: 11, marginTop: 12 }}>
          backend: {backendStatus}
        </p>
      </div>
    );
  }

  return (
    <div className="terminal-shell" onClick={() => terminalsRef.current.get(activeIdRef.current ?? "")?.terminal.focus()}>
      <div className="terminal-toolbar">
        <div>
          <div className="terminal-title">
            {sessionId ? `Session ${sessionId.slice(0, 8)}` : "No session"}
          </div>
          <div className="terminal-subtitle">{terminalStatus}</div>
        </div>
      </div>
      <div ref={containerRef} className="terminal-container" />
    </div>
  );
}

export default CenterPanel;
