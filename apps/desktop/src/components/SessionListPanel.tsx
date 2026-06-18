import type { SessionInfo, WorkspaceInfo } from "../api";
import {
  needsAttention,
  sessionAttentionStatus,
  sourceColor,
  statusDotColor,
  attentionBorderColor,
} from "./RightSidebarHelpers.ts";

interface WorkspaceHeaderProps {
  workspace: WorkspaceInfo | null;
  onOpenWorkspace: () => void;
}

function WorkspaceHeader({ workspace, onOpenWorkspace }: WorkspaceHeaderProps) {
  if (!workspace) {
    return (
      <div className="workspace-header workspace-header--empty">
        <div className="workspace-header-title">Workspace</div>
        <button
          className="workspace-open-button"
          type="button"
          onClick={onOpenWorkspace}
        >
          Open Folder
        </button>
      </div>
    );
  }

  const name = workspace.path.split("/").pop() || workspace.path;

  return (
    <div className="workspace-header">
      <div className="workspace-header-title">
        <span>Workspace</span>
        <button
          className="workspace-switch-button"
          type="button"
          onClick={onOpenWorkspace}
          title="Switch workspace"
        >
          &#x21C4;
        </button>
      </div>
      <div className="workspace-info">
        <div className="workspace-name">{name}</div>
        <div className="workspace-path">{workspace.path}</div>
        {workspace.branch && (
          <div className="workspace-git">
            <span>{workspace.branch}</span>
            <span className="workspace-git-sep">&middot;</span>
            <span className={workspace.dirty ? "workspace-dirty" : "workspace-clean"}>
              {workspace.dirty ? "dirty" : "clean"}
            </span>
          </div>
        )}
      </div>
    </div>
  );
}

interface SessionListPanelProps {
  workspace: WorkspaceInfo | null;
  onOpenWorkspace: () => void;
  sessions: SessionInfo[];
  activeSessionId: string | null;
  onSelectSession: (id: string) => void;
  onKillSession: (id: string) => void;
}

function SessionListPanel({
  workspace,
  onOpenWorkspace,
  sessions,
  activeSessionId,
  onSelectSession,
  onKillSession,
}: SessionListPanelProps) {
  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      {workspace ? (
        <>
          <WorkspaceHeader workspace={workspace} onOpenWorkspace={onOpenWorkspace} />
          <div className="panel-content">
            {sessions.length === 0 ? (
              <p className="empty-state">No active sessions</p>
            ) : (
              <ul className="session-list">
                {sessions.map((s) => {
                  const attn = sessionAttentionStatus(s);
                  return (
                    <li
                      key={s.id}
                      className={[
                        "session-item",
                        s.id === activeSessionId ? "session-item--active" : "",
                        s.memoryState !== "live" ? "session-item--remembered" : "",
                        s.memoryState === "resumable" ? "session-item--resumable" : "",
                      ].filter(Boolean).join(" ")}
                      style={{ borderLeft: `3px solid ${attentionBorderColor(attn)}` }}
                      onClick={() => onSelectSession(s.id)}
                    >
                      <div className="session-item-main">
                        <span
                          className="session-status"
                          style={{ background: statusDotColor(attn) }}
                        />
                        <div className="session-item-info">
                          <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
                            {needsAttention(attn) && (
                              <span className="session-item-alert" title="Needs attention">&#x26A0;</span>
                            )}
                            <span className="session-item-label">{s.label}</span>
                          </div>
                          <span className="session-item-meta">
                            {attn} &middot; {s.cwd.split("/").pop() || s.cwd}
                          </span>
                          {s.metadataSource && (
                            <span
                              className="session-item-badge"
                              style={{
                                background: sourceColor(s.metadataSource) + "22",
                                color: sourceColor(s.metadataSource),
                              }}
                            >
                              {s.metadataSource} &middot; {Math.round((s.metadataConfidence ?? 1) * 100)}%
                            </span>
                          )}
                          {s.memoryState !== "live" && (
                            <span className="session-memory-badge">
                              {s.memoryState === "resumable" ? "resumable" : "remembered"}
                            </span>
                          )}
                        </div>
                      </div>
                      {s.memoryState === "live" && (
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
                      )}
                    </li>
                  );
                })}
              </ul>
            )}
          </div>
        </>
      ) : (
        <WorkspaceHeader workspace={null} onOpenWorkspace={onOpenWorkspace} />
      )}
    </div>
  );
}

export default SessionListPanel;
