import { useEffect, useState } from "react";
import { workspaceHookNotices, type GroupedIntegrationStatus } from "../harnessIntegrationPresentation";

interface Props {
  activeHarnessIds: readonly string[];
  settingsOpen: boolean;
  onOpenSettings: () => void;
}

export default function WorkspaceHookNotice({ activeHarnessIds, settingsOpen, onOpenSettings }: Props) {
  const [notices, setNotices] = useState<ReturnType<typeof workspaceHookNotices>>([]);

  useEffect(() => {
    if (settingsOpen) return;
    if (activeHarnessIds.length === 0) {
      setNotices([]);
      return;
    }

    let request: AbortController | undefined;
    let refreshTimer: number | undefined;
    async function refresh() {
      window.clearTimeout(refreshTimer);
      request?.abort();
      const controller = new AbortController();
      request = controller;
      try {
        const baseUrl = await window.orkworks.getBackendUrl();
        if (controller.signal.aborted) return;
        const response = await fetch(baseUrl + "/workspace/integrations", { signal: controller.signal });
        if (!response.ok) throw new Error("Workspace integration status unavailable");
        const groups = await response.json() as GroupedIntegrationStatus[];
        if (controller.signal.aborted) return;

        const next = workspaceHookNotices(groups);
        setNotices(next);
        // Trust can be granted in a terminal without changing app focus.
        if (next.length) refreshTimer = window.setTimeout(() => void refresh(), 15_000);
      } catch {
        if (!controller.signal.aborted) setNotices([]);
      }
    }

    void refresh();
    window.addEventListener("focus", refresh);
    return () => {
      window.removeEventListener("focus", refresh);
      window.clearTimeout(refreshTimer);
      request?.abort();
    };
  }, [activeHarnessIds, settingsOpen]);

  if (notices.length === 0 || activeHarnessIds.length === 0) return null;
  return <div className="shell-preferences-notice workspace-hook-notice">
    <div role="status">
      {notices.map(({ key, text }) => <div key={key}>{text}</div>)}
    </div>
    <button type="button" onClick={onOpenSettings}>Coding tools</button>
  </div>;
}
