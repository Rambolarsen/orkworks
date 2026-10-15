import CenterPanel from "./CenterPanel";
import HistoricalTerminal from "./HistoricalTerminal";
import EmptyState from "./EmptyState";
import type { SessionInfo } from "../api";
import { isSessionStarting } from "../terminalPresentation";

interface TerminalPanelProps {
  backendStatus: string;
  session: SessionInfo | null;
  onBackendUnavailable: () => void;
  onRetryBackend: () => void;
}

function TerminalPanel({ backendStatus, session, onBackendUnavailable, onRetryBackend }: TerminalPanelProps) {
  if (!session) {
    return <EmptyState message="Select a session to open its terminal." />;
  }
  return session.lifecycle === "dead" ? (
    <HistoricalTerminal sessionId={session.id} />
  ) : (
    <CenterPanel
      backendStatus={backendStatus}
      sessionId={session.id}
      starting={isSessionStarting(session)}
      onBackendUnavailable={onBackendUnavailable}
      onRetryBackend={onRetryBackend}
    />
  );
}

export default TerminalPanel;
