import { useState } from "react";
import CenterPanel from "./CenterPanel";
import type { SessionInfo } from "../api";

interface SessionViewProps {
  sessions: SessionInfo[];
  activeSessionId: string | null;
  backendStatus: string;
  onKillSession: (id: string) => void;
}

function SessionView({ sessions, activeSessionId, backendStatus, onKillSession }: SessionViewProps) {
  const [subTabs, setSubTabs] = useState<Record<string, string>>({});

  const activeTab = (sessionId: string) => subTabs[sessionId] ?? "terminal";

  if (sessions.length === 0) {
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
    <div className="session-view">
      {sessions.map((session) => {
        const visible = session.id === activeSessionId;
        const currentSubTab = activeTab(session.id);

        return (
          <div
            key={session.id}
            style={{
              display: visible ? "flex" : "none",
              flexDirection: "column",
              flex: 1,
              minHeight: 0,
            }}
          >
            <div className="session-tab-bar">
              <span className="session-tab-bar-label" title={session.label}>
                {session.label}
              </span>
              {(["terminal"] as const).map((tabId) => (
                <div
                  key={tabId}
                  className={`session-tab ${tabId === currentSubTab ? "session-tab--active" : ""}`}
                  onClick={() => setSubTabs((prev) => ({ ...prev, [session.id]: tabId }))}
                >
                  <span className="session-tab-label">
                    {tabId === "terminal" ? "Terminal" : tabId}
                  </span>
                </div>
              ))}
              <div style={{ flex: 1 }} />
              <button
                className="session-tab-close"
                type="button"
                onClick={() => onKillSession(session.id)}
                title="Close session"
              >
                &times;
              </button>
            </div>
            <div className="session-tab-content" style={{ display: currentSubTab === "terminal" ? "flex" : "none", flex: 1, minHeight: 0 }}>
              <CenterPanel
                backendStatus={backendStatus}
                sessionId={session.id}
                embedded
              />
            </div>
          </div>
        );
      })}
    </div>
  );
}

export default SessionView;
