import type { SessionInfo, WorkspaceInfo } from "../api";
import WorkspaceHeader from "./WorkspaceHeader";

interface LeftSidebarProps {
  workspace: WorkspaceInfo | null;
  onOpenWorkspace: () => void;
  sessions: SessionInfo[];
  activeSessionId: string | null;
  onSelectSession: (id: string) => void;
  onCreateSession: () => void;
  onKillSession: (id: string) => void;
}

function LeftSidebar({
  workspace,
  onOpenWorkspace,
  sessions,
  activeSessionId,
  onSelectSession,
  onCreateSession,
  onKillSession,
}: LeftSidebarProps) {
  return (
    <>
      {workspace ? (
        <>
          <WorkspaceHeader workspace={workspace} onOpenWorkspace={onOpenWorkspace} />
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
      ) : (
        <WorkspaceHeader workspace={null} onOpenWorkspace={onOpenWorkspace} />
      )}
    </>
  );
}

export default LeftSidebar;
