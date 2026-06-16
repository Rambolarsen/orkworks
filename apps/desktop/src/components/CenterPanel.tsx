import { useEffect, useRef, useCallback, useState } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";
import { terminalLaunchInput } from "../terminalLaunch";
import { terminalPtySize } from "../terminalSize";
import { orkworksTerminalTheme } from "../terminalTheme";

interface CenterPanelProps {
  backendStatus: string;
}

declare global {
  interface Window {
    orkworks: {
      getBackendUrl: () => Promise<string>;
    };
  }
}

function CenterPanel({ backendStatus }: CenterPanelProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<Terminal | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  const pendingInputRef = useRef<string | null>(null);
  const [terminalStatus, setTerminalStatus] = useState("terminal starting");
  const [wsReady, setWsReady] = useState(false);

  const sendResize = useCallback(() => {
    const term = termRef.current;
    const ws = wsRef.current;
    if (!term || !ws || ws.readyState !== WebSocket.OPEN) return;
    ws.send(JSON.stringify({ type: "resize", ...terminalPtySize({ rows: term.rows, cols: term.cols }) }));
  }, []);

  const sendTerminalInput = useCallback((input: string) => {
    const ws = wsRef.current;
    if (ws?.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({ type: "input", data: input }));
      return;
    }

    pendingInputRef.current = input;
  }, []);

  const launchClaudeCode = useCallback(() => {
    sendTerminalInput(terminalLaunchInput("claude-code"));
    setTerminalStatus(wsReady ? "Claude Code launched" : "Claude Code queued");
  }, [sendTerminalInput, wsReady]);

  useEffect(() => {
    if (backendStatus !== "connected" || !containerRef.current) return;
    let cancelled = false;
    setTerminalStatus("terminal starting");

    const term = new Terminal({
      cursorBlink: true,
      fontSize: 14,
      fontFamily: "'Menlo', 'Monaco', 'Courier New', monospace",
      theme: orkworksTerminalTheme,
      allowProposedApi: true,
    });

    const fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.open(containerRef.current);
    term.focus();

    fitAddonRef.current = fitAddon;
    termRef.current = term;

    try {
      fitAddon.fit();
      term.focus();
    } catch {
      /* container may not be sized yet */
    }

    window.orkworks.getBackendUrl().then((baseUrl) => {
      if (cancelled) return;

      const wsUrl = baseUrl.replace("http", "ws") + "/terminal";
      const ws = new WebSocket(wsUrl);
      ws.binaryType = "arraybuffer";
      wsRef.current = ws;
      setTerminalStatus("terminal connecting");

      ws.onopen = () => {
        setWsReady(true);
        setTerminalStatus("terminal ready");
        sendResize();
        if (pendingInputRef.current) {
          ws.send(JSON.stringify({ type: "input", data: pendingInputRef.current }));
          pendingInputRef.current = null;
          setTerminalStatus("Claude Code launched");
        }
      };

      ws.onmessage = (e) => {
        if (e.data instanceof ArrayBuffer) {
          term.write(new Uint8Array(e.data));
        }
      };

      ws.onclose = () => {
        if (cancelled) return;
        setWsReady(false);
        setTerminalStatus("terminal disconnected");
        term.dispose();
        termRef.current = null;
      };

      term.onData((data) => {
        if (ws.readyState === WebSocket.OPEN) {
          ws.send(JSON.stringify({ type: "input", data }));
        }
      });

      term.onResize(sendResize);
    });

    const handleWindowResize = () => {
      try {
        fitAddonRef.current?.fit();
      } catch {
        /* ignore */
      }
      sendResize();
    };

    window.addEventListener("resize", handleWindowResize);
    const resizeObserver = new ResizeObserver(handleWindowResize);
    resizeObserver.observe(containerRef.current);

    return () => {
      cancelled = true;
      window.removeEventListener("resize", handleWindowResize);
      resizeObserver.disconnect();
      wsRef.current?.close();
      term.dispose();
      termRef.current = null;
      wsRef.current = null;
      setWsReady(false);
    };
  }, [backendStatus, sendResize]);

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
    <div className="terminal-shell" onClick={() => termRef.current?.focus()}>
      <div className="terminal-toolbar">
        <div>
          <div className="terminal-title">Terminal</div>
          <div className="terminal-subtitle">{terminalStatus}</div>
        </div>
        <div className="terminal-actions">
          <button
            className="terminal-launch-button"
            type="button"
            onClick={launchClaudeCode}
            disabled={backendStatus !== "connected"}
          >
            Start Claude Code
          </button>
        </div>
      </div>
      <div ref={containerRef} className="terminal-container" />
    </div>
  );
}

export default CenterPanel;
