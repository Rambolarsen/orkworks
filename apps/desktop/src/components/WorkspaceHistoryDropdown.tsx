import { useCallback, useEffect, useRef, useState } from "react";
import { WorkspaceHistoryList, type WorkspaceHistoryListProps } from "./WorkspaceHistoryList";

export interface WorkspaceHistoryDropdownProps extends WorkspaceHistoryListProps {
  triggerLabel: string;
  triggerClassName: string;
  triggerAriaLabel: string;
}

export function WorkspaceHistoryDropdown({
  triggerLabel,
  triggerClassName,
  triggerAriaLabel,
  onOpenPath,
  onOpenOtherFolder,
  ...listProps
}: WorkspaceHistoryDropdownProps) {
  const [open, setOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement | null>(null);
  const close = useCallback(() => setOpen(false), []);

  useEffect(() => {
    if (!open) return;
    function handlePointerDown(event: MouseEvent) {
      if (containerRef.current && !containerRef.current.contains(event.target as Node)) close();
    }
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") close();
    }
    document.addEventListener("mousedown", handlePointerDown);
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("mousedown", handlePointerDown);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [open, close]);

  return (
    <div className="workspace-history-dropdown" ref={containerRef}>
      <button
        type="button"
        className={triggerClassName}
        aria-label={triggerAriaLabel}
        title={triggerAriaLabel}
        onClick={() => setOpen((value) => !value)}
      >
        {triggerLabel}
      </button>
      {open && (
        <div className="workspace-history-panel">
          <WorkspaceHistoryList
            {...listProps}
            onOpenPath={(path) => { close(); onOpenPath(path); }}
            onOpenOtherFolder={() => { close(); onOpenOtherFolder(); }}
          />
        </div>
      )}
    </div>
  );
}
