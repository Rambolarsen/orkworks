import CenterPanel from "./CenterPanel";
import type { SessionInfo } from "../api";

interface TerminalPanelProps {
  backendStatus: string;
  session: SessionInfo | null;
  onKillSession: (id: string) => void;
}

function TerminalPanel({
  backendStatus,
  session,
  onKillSession,
}: TerminalPanelProps) {
  if (!session) {
    return (
      <div style={{ flex: 1, display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", height: "100%" }}>
        <p style={{ color: "#666", fontSize: 12 }}>No active terminal</p>
      </div>
    );
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div style={{
        display: "flex", alignItems: "center",
        background: "#252526", borderBottom: "1px solid #3c3c3c",
        minHeight: 30, padding: "0 10px",
      }}>
        <span style={{ color: "#d4d4d4", fontSize: 12, flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
          {session.label}
        </span>
        <button
          onClick={() => onKillSession(session.id)}
          style={{
            border: "none", background: "none", color: "#666",
            cursor: "pointer", fontSize: 14, padding: "0 2px", lineHeight: 1,
          }}
          title="Kill session"
        >
          &times;
        </button>
      </div>
      <div style={{ flex: 1, minHeight: 0, display: "flex" }}>
        <CenterPanel
          backendStatus={backendStatus}
          sessionId={session.id}
          embedded
        />
      </div>
    </div>
  );
}

export default TerminalPanel;
