import type { SessionInfo } from "../api";

interface LeftSidebarProps {
  sessions: SessionInfo[];
  activeSessionId: string | null;
  onSelectSession: (id: string) => void;
  onCreateSession: () => void;
  onKillSession: (id: string) => void;
}

function LeftSidebar({
  sessions,
  activeSessionId,
  onSelectSession,
  onCreateSession,
  onKillSession,
}: LeftSidebarProps) {
  return (
    <>
      <div className="panel-header">
        <span>Sessions</span>
        <button
          className="session-new-button"
          type="button"
          onClick={onCreateSession}
          title="New session"
        >
          +
        </button>
      </div>
      <div className="panel-content">
        {sessions.length === 0 ? (
          <p className="empty-state">No active sessions</p>
        ) : (
          <ul className="session-list">
            {sessions.map((s) => (
              <li
                key={s.id}
                className={`session-item ${s.id === activeSessionId ? "session-item--active" : ""}`}
                onClick={() => onSelectSession(s.id)}
              >
                <div className="session-item-main">
                  <span
                    className={`session-status session-status--${s.status}`}
                  />
                  <div className="session-item-info">
                    <span className="session-item-label">{s.label}</span>
                    <span className="session-item-meta">
                      {s.status} &middot; {s.cwd.split("/").pop() || s.cwd}
                    </span>
                  </div>
                </div>
                <button
                  className="session-kill-button"
                  type="button"
                  title="Kill session"
                  onClick={(e) => {
                    e.stopPropagation();
                    onKillSession(s.id);
                  }}
                >
                  &times;
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>
    </>
  );
}

export default LeftSidebar;
