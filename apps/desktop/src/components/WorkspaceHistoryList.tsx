import { useEffect, useLayoutEffect, useRef, useState } from "react";
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
  const list = useRef<HTMLDivElement>(null);
  const openOtherFolder = useRef<HTMLButtonElement>(null);
  const focusAfterChange = useRef<{ invoker: HTMLButtonElement; pinPath?: string } | null>(null);

  useEffect(() => {
    let cancelled = false;
    window.orkworks.getWorkspaceHistory()
      .then((next) => { if (!cancelled) setSnapshot(next); })
      .catch(() => { if (!cancelled) onError("Couldn't load workspace history."); });
    return () => { cancelled = true; };
  }, [onError]);

  const updateHistory = (
    operation: Promise<WorkspaceHistorySnapshot>,
    invoker: HTMLButtonElement,
    error: string,
    pinPath?: string,
  ) => {
    void operation.then(next => {
      focusAfterChange.current = document.activeElement === invoker ? { invoker, pinPath } : null;
      setSnapshot(next);
    }).catch(() => onError(error));
  };

  useLayoutEffect(() => {
    const pending = focusAfterChange.current;
    focusAfterChange.current = null;
    if (!pending || pending.invoker.isConnected || document.activeElement !== document.body) return;
    const pin = pending.pinPath === undefined
      ? null
      : list.current?.querySelector<HTMLButtonElement>(`[data-workspace-pin-path="${CSS.escape(pending.pinPath)}"]`);
    (pin ?? openOtherFolder.current)?.focus();
  }, [snapshot]);

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
          data-workspace-pin-path={path}
          onClick={event => updateHistory(
            pinned ? window.orkworks.unpinWorkspacePath(path) : window.orkworks.pinWorkspacePath(path),
            event.currentTarget,
            pinned ? "Couldn't unpin workspace." : "Couldn't pin workspace.",
            path,
          )}
        >
          {pinned ? <PinOff size={14} aria-hidden="true" /> : <Pin size={14} aria-hidden="true" />}
        </button>
        <button
          type="button"
          className="workspace-history-row-action"
          aria-label={VOCAB.removeWorkspace}
          title={VOCAB.removeWorkspace}
          onClick={event => updateHistory(window.orkworks.forgetWorkspacePath(path), event.currentTarget, "Couldn't remove workspace from history.")}
        >
          ✕
        </button>
      </li>
    );
  };

  const isEmpty = snapshot.pinned.length === 0 && snapshot.recent.length === 0;

  return (
    <div ref={list} className="workspace-history-list">
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
      <button
        type="button"
        ref={openOtherFolder}
        className="workspace-history-other-folder"
        disabled={isSwitching}
        onClick={onOpenOtherFolder}
      >
        {VOCAB.workspaceHistoryOpenOtherFolder}
      </button>
    </div>
  );
}
