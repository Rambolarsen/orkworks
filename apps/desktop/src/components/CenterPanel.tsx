import { useEffect, useRef, useCallback, useState } from "react";
import "@xterm/xterm/css/xterm.css";
import { ensureTerminal, getTerminal } from "../terminalStore";
import type { AppSettings, HotkeySettings, SaveHotkeysResult } from "../appSettingsTypes";

interface CenterPanelProps {
  backendStatus: string;
  sessionId: string | null;
  embedded?: boolean;
}

import { type WorkspaceInfo } from "../api";

declare global {
  interface Window {
    orkworks: {
      getBackendUrl: () => Promise<string>;
      getInitialWorkspace: () => Promise<WorkspaceInfo | null>;
      openWorkspace: () => Promise<WorkspaceInfo | null>;
      getLayout: () => Promise<string | null>;
      saveLayout: (json: string) => Promise<void>;
      getSettings: () => Promise<AppSettings>;
      saveHotkeys: (hotkeys: HotkeySettings) => Promise<SaveHotkeysResult>;
      setHotkeyCaptureActive: (active: boolean) => void;
      onMenuCommand: (callback: (data: { action: string; panelId?: string }) => void) => () => void;
      notifyPanelVisibility: (panelId: string, visible: boolean) => void;
    };
  }
}

function CenterPanel({ backendStatus, sessionId, embedded }: CenterPanelProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const activeIdRef = useRef<string | null>(null);
  const [terminalStatus, setTerminalStatus] = useState("");

  const attachTerminal = useCallback((id: string) => {
    const container = containerRef.current;
    const handle = getTerminal(id);
    if (!container || !handle) return;

    if (handle.wrapper.parentElement !== container) {
      container.appendChild(handle.wrapper);
    }
    if (!handle.terminal.element) {
      handle.terminal.open(handle.wrapper);
    }

    for (const child of Array.from(container.children) as HTMLElement[]) {
      if (!(child instanceof HTMLDivElement)) continue;
      child.style.visibility = child === handle.wrapper ? "visible" : "hidden";
    }

    activeIdRef.current = id;
    setTerminalStatus(
      handle.ended
        ? "session ended"
        : handle.ws.readyState === WebSocket.OPEN
          ? "terminal ready"
          : "terminal connecting",
    );

    try {
      handle.fitAddon.fit();
    } catch {
      /* ignore */
    }
    const listEl = document.getElementById("sessions-list");
    const listHasFocus = !!listEl && listEl.contains(document.activeElement);
    if (!handle.ended && !listHasFocus) {
      handle.terminal.focus();
    }
  }, []);

  useEffect(() => {
    if (backendStatus !== "connected" || !sessionId) return;
    let cancelled = false;

    if (getTerminal(sessionId)) {
      attachTerminal(sessionId);
      return;
    }

    setTerminalStatus("terminal starting");
    window.orkworks.getBackendUrl().then((baseUrl) => {
      if (cancelled) return;
      ensureTerminal(sessionId, baseUrl);
      attachTerminal(sessionId);
    });

    return () => {
      cancelled = true;
    };
  }, [backendStatus, sessionId, attachTerminal]);

  useEffect(() => {
    const handleWindowResize = () => {
      const active = activeIdRef.current;
      if (!active) return;
      const handle = getTerminal(active);
      if (!handle) return;
      try {
        handle.fitAddon.fit();
      } catch {
        /* ignore */
      }
    };

    window.addEventListener("resize", handleWindowResize);
    const observer = new ResizeObserver(handleWindowResize);
    if (containerRef.current) observer.observe(containerRef.current);

    return () => {
      window.removeEventListener("resize", handleWindowResize);
      observer.disconnect();
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

  const ended = sessionId ? getTerminal(sessionId)?.ended : false;

  return (
    <div className="terminal-shell" onClick={() => {
      const active = activeIdRef.current;
      if (active) getTerminal(active)?.terminal.focus();
    }}>
      {!embedded && (
        <div className="terminal-toolbar">
          <div>
            <div className="terminal-title">
              {sessionId ? `Session ${sessionId.slice(0, 8)}` : "No session"}
            </div>
            <div className="terminal-subtitle">{terminalStatus}</div>
          </div>
        </div>
      )}
      <div
        ref={containerRef}
        className={`terminal-container${ended ? " terminal-container--ended" : ""}`}
      />
    </div>
  );
}

export default CenterPanel;
