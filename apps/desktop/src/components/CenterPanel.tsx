import { useEffect, useRef, useCallback } from "react";
import "@xterm/xterm/css/xterm.css";
import { disposeTerminal, ensureTerminal, getTerminal } from "../terminalStore";
import EmptyState from "./EmptyState";

interface CenterPanelProps {
  backendStatus: string;
  sessionId: string | null;
}

function CenterPanel({ backendStatus, sessionId }: CenterPanelProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const activeIdRef = useRef<string | null>(null);

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

    try {
      handle.fitAddon.fit();
    } catch {
      /* xterm not yet measured */
    }
    const listEl = document.getElementById("sessions-list");
    const listHasFocus = !!listEl && listEl.contains(document.activeElement);
    if (!handle.ended && !listHasFocus) {
      handle.terminal.focus();
    }
  }, []);

  useEffect(() => {
    const previousId = activeIdRef.current;
    if (previousId && backendStatus !== "connected") {
      disposeTerminal(previousId);
      activeIdRef.current = null;
    }
    if (backendStatus !== "connected" || !sessionId) return;
    let cancelled = false;

    if (getTerminal(sessionId)) {
      attachTerminal(sessionId);
      return;
    }

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
    let fitRaf: number | null = null;

    const handleWindowResize = () => {
      const active = activeIdRef.current;
      if (!active) return;
      const handle = getTerminal(active);
      if (!handle) return;
      if (fitRaf !== null) cancelAnimationFrame(fitRaf);
      fitRaf = requestAnimationFrame(() => {
        fitRaf = null;
        try {
          handle.fitAddon.fit();
        } catch (err) {
          console.warn("[CenterPanel] fit() failed for session", handle.id, err);
        }
      });
    };

    window.addEventListener("resize", handleWindowResize);
    const observer = new ResizeObserver(handleWindowResize);
    if (containerRef.current) observer.observe(containerRef.current);

    return () => {
      if (fitRaf !== null) cancelAnimationFrame(fitRaf);
      window.removeEventListener("resize", handleWindowResize);
      observer.disconnect();
    };
  }, []);

  if (backendStatus !== "connected") {
    return <EmptyState message="Connecting to OrkWorks…" />;
  }

  const ended = sessionId ? getTerminal(sessionId)?.ended : false;

  return (
    <div
      className="terminal-shell"
      onClick={() => {
        const active = activeIdRef.current;
        if (active) getTerminal(active)?.terminal.focus();
      }}
    >
      <div
        ref={containerRef}
        className={`terminal-container${ended ? " terminal-container--ended" : ""}`}
      />
    </div>
  );
}

export default CenterPanel;
