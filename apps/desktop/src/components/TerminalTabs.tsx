import { useState, useCallback, forwardRef, useImperativeHandle } from "react";
import CenterPanel from "./CenterPanel";

interface Tab {
  id: string;
  label: string;
}

export interface TerminalTabsHandle {
  addTab: (sessionId: string, tab: Tab) => void;
  removeTab: (sessionId: string, tabId: string) => void;
  setActiveTab: (sessionId: string, tabId: string) => void;
}

interface TerminalTabsProps {
  backendStatus: string;
  activeSessionId: string | null;
  sessions: Array<{ id: string; label: string }>;
}

const TerminalTabs = forwardRef<TerminalTabsHandle, TerminalTabsProps>(
  function TerminalTabs({ backendStatus, activeSessionId, sessions }, ref) {
    const [tabs, setTabs] = useState<Tab[]>([]);
    const [activeTabId, setActiveTabId] = useState<string | null>(null);

    const addTab = useCallback((_sessionId: string, tab: Tab) => {
      setTabs((prev) => {
        if (prev.some((t) => t.id === tab.id)) return prev;
        return [...prev, tab];
      });
      setActiveTabId(tab.id);
    }, []);

    const removeTab = useCallback((_sessionId: string, tabId: string) => {
      setTabs((prev) => prev.filter((t) => t.id !== tabId));
      setActiveTabId((prev) => (prev === tabId ? null : prev));
    }, []);

    const setActiveTab = useCallback((_sessionId: string, tabId: string) => {
      setActiveTabId(tabId);
    }, []);

    useImperativeHandle(ref, () => ({ addTab, removeTab, setActiveTab }), [addTab, removeTab, setActiveTab]);

    if (!activeSessionId) {
      return (
        <div className="terminal-tabs-empty">
          <p style={{ color: "#666", fontSize: 12 }}>No session selected</p>
          <p style={{ color: "#555", fontSize: 11, marginTop: 4 }}>
            Select or create a session to get started
          </p>
        </div>
      );
    }

    const activeSession = sessions.find((s) => s.id === activeSessionId);
    const displayTabId = activeTabId && tabs.some((t) => t.id === activeTabId) ? activeTabId : null;

    return (
      <div className="terminal-tabs">
        <div className="terminal-tab-bar">
          {activeSession && (
            <div className="terminal-tab terminal-tab--active">
              <span className="terminal-tab-dot" />
              <span className="terminal-tab-label">{activeSession.label}</span>
            </div>
          )}
          {tabs.map((tab) => (
            <div
              key={tab.id}
              className={`terminal-tab ${tab.id === displayTabId ? "terminal-tab--active" : ""}`}
              onClick={() => setActiveTabId(tab.id)}
            >
              <span className="terminal-tab-label">{tab.label}</span>
            </div>
          ))}
        </div>
        <div className="terminal-tab-content">
          {!displayTabId ? (
            sessions.map((s) => (
              <div
                key={s.id}
                style={{
                  display: s.id === activeSessionId ? "flex" : "none",
                  flex: 1,
                  minHeight: 0,
                }}
              >
                <CenterPanel
                  backendStatus={backendStatus}
                  sessionId={s.id}
                  embedded
                />
              </div>
            ))
          ) : (
            <div className="terminal-tabs-empty">
              <p style={{ color: "#858585", fontSize: 12 }}>{displayTabId}</p>
            </div>
          )}
        </div>
      </div>
    );
  },
);

export default TerminalTabs;
