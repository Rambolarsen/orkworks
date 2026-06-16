import { useState, useCallback, forwardRef, useImperativeHandle, useEffect } from "react";
import CenterPanel from "./CenterPanel";

interface Tab {
  id: string;
  label: string;
}

interface SessionTabs {
  sessionId: string;
  label: string;
  tabs: Tab[];
  activeTabId: string | null;
}

export interface TerminalTabsHandle {
  openSession: (id: string, label: string) => void;
  closeSession: (id: string) => void;
  focusSession: (id: string) => void;
}

interface TerminalTabsProps {
  backendStatus: string;
}

const TerminalTabs = forwardRef<TerminalTabsHandle, TerminalTabsProps>(
  function TerminalTabs({ backendStatus }, ref) {
    const [groups, setGroups] = useState<SessionTabs[]>([]);
    const [activeSessionId, setActiveSessionId] = useState<string | null>(null);

    const openSession = useCallback((id: string, label: string) => {
      setGroups((prev) => {
        if (prev.some((g) => g.sessionId === id)) return prev;
        return [...prev, { sessionId: id, label, tabs: [], activeTabId: null }];
      });
      setActiveSessionId(id);
    }, []);

    const closeSession = useCallback((id: string) => {
      setGroups((prev) => prev.filter((g) => g.sessionId !== id));
      setActiveSessionId((prev) => (prev === id ? null : prev));
    }, []);

    const focusSession = useCallback((id: string) => {
      setActiveSessionId(id);
    }, []);

    useImperativeHandle(ref, () => ({ openSession, closeSession, focusSession }), [openSession, closeSession, focusSession]);

    useEffect(() => {
      if (activeSessionId === null && groups.length > 0) {
        setActiveSessionId(groups[groups.length - 1].sessionId);
      }
    }, [activeSessionId, groups]);

    const activeGroup = groups.find((g) => g.sessionId === activeSessionId);

    if (groups.length === 0) {
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
          {groups.map((g) => (
            <div
              key={g.sessionId}
              className={`terminal-tab ${g.sessionId === activeSessionId ? "terminal-tab--active" : ""}`}
              onClick={() => setActiveSessionId(g.sessionId)}
            >
              <span className="terminal-tab-dot" />
              <span className="terminal-tab-label">{g.label}</span>
            </div>
          ))}
        </div>
        <div className="terminal-tab-content">
          {activeGroup && (
            <CenterPanel
              key={activeGroup.sessionId}
              backendStatus={backendStatus}
              sessionId={activeGroup.sessionId}
              embedded
            />
          )}
        </div>
      </div>
    );
  },
);

export default TerminalTabs;
