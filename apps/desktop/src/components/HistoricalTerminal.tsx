import { useEffect, useRef, useState } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";
import { getTerminalOutput } from "../api";
import { loadTerminalReplay } from "../terminalReplay";
import { orkworksTerminalTheme } from "../terminalTheme";
import EmptyState from "./EmptyState";

export default function HistoricalTerminal({ sessionId }: { sessionId: string }) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [state, setState] = useState<"loading" | "empty" | "error" | "loaded">("loading");

  useEffect(() => {
    let current = true;
    let terminal: Terminal | null = null;
    let observer: ResizeObserver | null = null;

    void window.orkworks.getBackendUrl()
      .then((baseUrl) => loadTerminalReplay(
        () => getTerminalOutput(baseUrl, sessionId),
        () => current,
        () => {
          terminal = new Terminal({ theme: orkworksTerminalTheme, disableStdin: true, cursorBlink: false, scrollback: 2000 });
          const fitAddon = new FitAddon();
          terminal.loadAddon(fitAddon);
          if (containerRef.current) {
            terminal.open(containerRef.current);
            try { fitAddon.fit(); } catch { /* container not measured yet */ }
            observer = new ResizeObserver(() => {
              try { fitAddon.fit(); } catch { /* container not measured yet */ }
            });
            observer.observe(containerRef.current);
          }
          return terminal;
        },
      ))
      .then((result) => {
        if (!current || result === "stale") return;
        setState(result);
      })
      .catch(() => {
        if (current) setState("error");
      });

    return () => {
      current = false;
      observer?.disconnect();
      terminal?.dispose();
    };
  }, [sessionId]);

  if (state === "empty") return <EmptyState message="No saved terminal output for this session." />;
  if (state === "error") return <EmptyState message="Saved terminal output is unavailable." />;
  return <div className="terminal-shell"><div ref={containerRef} className="terminal-container" aria-label={state === "loading" ? "Loading saved terminal output" : "Saved terminal output"} /></div>;
}
