import { useEffect, useState } from "react";
import CenterPanel from "./CenterPanel";
import { sessionAttentionStatus, statusDotColor } from "./RightSidebarHelpers";
import type { SessionInfo } from "../api";

interface TerminalPanelProps {
  backendStatus: string;
  sessions: SessionInfo[];
  activeSessionId: string | null;
  onSelectSession: (id: string) => void;
  onKillSession: (id: string) => void;
}

function TerminalPanel({
  backendStatus,
  sessions,
  activeSessionId,
  onSelectSession,
  onKillSession,
}: TerminalPanelProps) {
  const [localActive, setLocalActive] = useState<string | null>(activeSessionId);

  useEffect(() => {
    if (activeSessionId) setLocalActive(activeSessionId);
  }, [activeSessionId]);

  const liveSessions = sessions.filter(
    (s) => s.status === "running" || s.status === "creating"
  );
  const currentSession = liveSessions.find((s) => s.id === localActive) ?? liveSessions[0] ?? null;

  if (!currentSession) {
    return (
      <div style={{ flex: 1, display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", height: "100%" }}>
        <p style={{ color: "#666", fontSize: 12 }}>No active terminal</p>
      </div>
    );
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div style={{
        display: "flex", alignItems: "stretch",
        background: "#252526", borderBottom: "1px solid #3c3c3c",
        minHeight: 30, overflowX: "auto",
      }}>
        {liveSessions.map((s) => {
          const attn = sessionAttentionStatus(s);
          const isActive = s.id === currentSession.id;
          return (
            <div
              key={s.id}
              onClick={() => {
                setLocalActive(s.id);
                onSelectSession(s.id);
              }}
              style={{
                display: "flex", alignItems: "center", gap: 6,
                padding: "4px 10px", cursor: "pointer",
                borderRight: "1px solid #2a2a2b",
                fontSize: 12, whiteSpace: "nowrap", userSelect: "none",
                color: isActive ? "#d4d4d4" : "#858585",
                background: isActive ? "#1e1e1e" : "transparent",
                borderBottom: isActive ? "1px solid #1e1e1e" : "none",
              }}
            >
              <span style={{
                width: 8, height: 8, borderRadius: "50%",
                background: statusDotColor(attn), flexShrink: 0,
              }} />
              <span style={{ overflow: "hidden", textOverflow: "ellipsis" }}>{s.label}</span>
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  onKillSession(s.id);
                }}
                style={{
                  border: "none", background: "none", color: "#666",
                  cursor: "pointer", fontSize: 14, padding: "0 2px", lineHeight: 1,
                }}
                title="Kill session"
              >
                &times;
              </button>
            </div>
          );
        })}
      </div>
      <div style={{ flex: 1, minHeight: 0, display: "flex" }}>
        <CenterPanel
          backendStatus={backendStatus}
          sessionId={currentSession.id}
          embedded
        />
      </div>
    </div>
  );
}

export default TerminalPanel;
