import { useCallback, useEffect, useState } from "react";
import { Pin, PinOff } from "lucide-react";
import { VOCAB } from "../labels";
import type { WorkspaceHistorySnapshot } from "../orkworksWindow";

export interface WorkspaceHistoryListProps {
  currentWorkspacePath: string | null;
  isSwitching: boolean;
  onOpenPath: (path: string) => void;
  onOpenOtherFolder: () => void;
  onError: (message: string) => void;
}

function workspaceLabel(path: string): string {
  return path.split(/[\\/]/).pop() || path;
}

export function WorkspaceHistoryList({
  currentWorkspacePath,
  isSwitching,
  onOpenPath,
  onOpenOtherFolder,
  onError,
}: WorkspaceHistoryListProps) {
  const [snapshot, setSnapshot] = useState<WorkspaceHistorySnapshot | null>(null);

  useEffect(() => {
    let cancelled = false;
    window.orkworks.getWorkspaceHistory()
      .then((next) => { if (!cancelled) setSnapshot(next); })
      .catch(() => { if (!cancelled) onError("Couldn't load workspace history."); });
    return () => { cancelled = true; };
  }, [onError]);

  const handlePin = useCallback((path: string) => {
    window.orkworks.pinWorkspacePath(path)
      .then(setSnapshot)
      .catch(() => onError("Couldn't pin workspace."));
  }, [onError]);

  const handleUnpin = useCallback((path: string) => {
    window.orkworks.unpinWorkspacePath(path)
      .then(setSnapshot)
      .catch(() => onError("Couldn't unpin workspace."));
  }, [onError]);

  const handleForget = useCallback((path: string) => {
    window.orkworks.forgetWorkspacePath(path)
      .then(setSnapshot)
      .catch(() => onError("Couldn't remove workspace from history."));
  }, [onError]);

  if (!snapshot) return null;

  const renderRow = (path: string, pinned: boolean) => {
    const isCurrent = path === currentWorkspacePath;
    return (
      <li key={path} className="workspace-history-row">
        <button
          type="button"
          className="workspace-history-row-open"
          title={path}
          disabled={isCurrent || isSwitching}
          onClick={() => onOpenPath(path)}
        >
          {workspaceLabel(path)}
          {isCurrent && (
            <span className="workspace-history-current-badge">{VOCAB.workspaceHistoryCurrent}</span>
          )}
        </button>
        <button
          type="button"
          className="workspace-history-row-action"
          aria-label={pinned ? VOCAB.unpinWorkspace : VOCAB.pinWorkspace}
          title={pinned ? VOCAB.unpinWorkspace : VOCAB.pinWorkspace}
          onClick={() => (pinned ? handleUnpin(path) : handlePin(path))}
        >
          {pinned ? <PinOff size={14} aria-hidden="true" /> : <Pin size={14} aria-hidden="true" />}
        </button>
        <button
          type="button"
          className="workspace-history-row-action"
          aria-label={VOCAB.removeWorkspace}
          title={VOCAB.removeWorkspace}
          onClick={() => handleForget(path)}
        >
          ✕
        </button>
      </li>
    );
  };

  const isEmpty = snapshot.pinned.length === 0 && snapshot.recent.length === 0;

  return (
    <div className="workspace-history-list">
      {snapshot.diagnostic && (
        <p role="alert" className="workspace-history-diagnostic">{snapshot.diagnostic.message}</p>
      )}
      {snapshot.pinned.length > 0 && (
        <>
          <p className="workspace-history-section-label">{VOCAB.workspaceHistoryPinnedSection}</p>
          <ul className="workspace-history-rows">{snapshot.pinned.map((path) => renderRow(path, true))}</ul>
        </>
      )}
      {snapshot.recent.length > 0 && (
        <>
          <p className="workspace-history-section-label">{VOCAB.workspaceHistoryRecentSection}</p>
          <ul className="workspace-history-rows">{snapshot.recent.map((path) => renderRow(path, false))}</ul>
        </>
      )}
      {isEmpty && !snapshot.diagnostic && (
        <p className="workspace-history-empty">{VOCAB.workspaceHistoryEmpty}</p>
      )}
      <button type="button" className="workspace-history-other-folder" onClick={onOpenOtherFolder}>
        {VOCAB.workspaceHistoryOpenOtherFolder}
      </button>
    </div>
  );
}
