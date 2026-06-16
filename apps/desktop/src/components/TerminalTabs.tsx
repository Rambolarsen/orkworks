import { useState, useCallback, forwardRef, useImperativeHandle, useEffect } from "react";
import CenterPanel from "./CenterPanel";

interface Tab {
  id: string;
  label: string;
}

export interface TerminalTabsHandle {
  open: (id: string, label: string) => void;
  close: (id: string) => void;
  focus: (id: string) => void;
}

interface TerminalTabsProps {
  backendStatus: string;
  onKillSession: (id: string) => void;
}

const TerminalTabs = forwardRef<TerminalTabsHandle, TerminalTabsProps>(
  function TerminalTabs({ backendStatus, onKillSession }, ref) {
    const [tabs, setTabs] = useState<Tab[]>([]);
    const [activeId, setActiveId] = useState<string | null>(null);

    const close = useCallback(
      (id: string) => {
        setTabs((prev) => prev.filter((t) => t.id !== id));
        setActiveId((prev) => (prev === id ? null : prev));
      },
      [],
    );

    const open = useCallback((id: string, label: string) => {
      setTabs((prev) => {
        if (prev.some((t) => t.id === id)) return prev;
        return [...prev, { id, label }];
      });
      setActiveId(id);
    }, []);

    const focus = useCallback((id: string) => {
      setTabs((prev) => {
        if (prev.some((t) => t.id === id)) {
          setActiveId(id);
        }
        return prev;
      });
    }, []);

    useImperativeHandle(ref, () => ({ open, close, focus }), [open, close, focus]);

    useEffect(() => {
      if (activeId === null && tabs.length > 0) {
        setActiveId(tabs[tabs.length - 1].id);
      }
    }, [activeId, tabs]);

    const activeExists = tabs.some((t) => t.id === activeId);
    const displayId = activeExists ? activeId : tabs.length > 0 ? tabs[tabs.length - 1].id : null;

    const handleTabClose = useCallback(
      (id: string, e: React.MouseEvent) => {
        e.stopPropagation();
        close(id);
        onKillSession(id);
      },
      [close, onKillSession],
    );

    if (tabs.length === 0) {
      return (
        <div className="terminal-tabs-empty">
          <p style={{ color: "#666", fontSize: 12 }}>No terminal sessions</p>
          <p style={{ color: "#555", fontSize: 11, marginTop: 4 }}>
            Create a session to get started
          </p>
        </div>
      );
    }

    return (
      <div className="terminal-tabs">
        <div className="terminal-tab-bar">
          {tabs.map((tab) => (
            <div
              key={tab.id}
              className={`terminal-tab ${tab.id === displayId ? "terminal-tab--active" : ""}`}
              onClick={() => setActiveId(tab.id)}
            >
              <span className="terminal-tab-dot" />
              <span className="terminal-tab-label">{tab.label}</span>
              <button
                className="terminal-tab-close"
                type="button"
                onClick={(e) => handleTabClose(tab.id, e)}
                title="Close session"
              >
                &times;
              </button>
            </div>
          ))}
        </div>
        <div className="terminal-tab-content">
          {tabs.map((tab) => (
            <div
              key={tab.id}
              style={{
                display: tab.id === displayId ? "flex" : "none",
                flex: 1,
                minHeight: 0,
              }}
            >
              <CenterPanel
                backendStatus={backendStatus}
                sessionId={tab.id}
                embedded
              />
            </div>
          ))}
        </div>
      </div>
    );
  },
);

export default TerminalTabs;
