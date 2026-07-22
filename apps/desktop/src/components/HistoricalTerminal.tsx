import { useEffect, useRef, useState } from "react";
import { Terminal } from "@xterm/xterm";
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

    void window.orkworks.getBackendUrl()
      .then((baseUrl) => loadTerminalReplay(
        () => getTerminalOutput(baseUrl, sessionId),
        () => current,
        () => {
          terminal = new Terminal({ theme: orkworksTerminalTheme, disableStdin: true, cursorBlink: false, scrollback: 2000 });
          if (containerRef.current) terminal.open(containerRef.current);
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
      terminal?.dispose();
    };
  }, [sessionId]);

  if (state === "empty") return <EmptyState message="No saved terminal output for this session." />;
  if (state === "error") return <EmptyState message="Saved terminal output is unavailable." />;
  return <div ref={containerRef} className="terminal-container" aria-label={state === "loading" ? "Loading saved terminal output" : "Saved terminal output"} />;
}
