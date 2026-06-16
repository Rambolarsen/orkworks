import type { WorkspaceInfo } from "../api";

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

export default WorkspaceHeader;
