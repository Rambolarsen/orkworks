import CenterPanel from "./CenterPanel";
import HistoricalTerminal from "./HistoricalTerminal";
import EmptyState from "./EmptyState";
import type { SessionInfo } from "../api";
import { isSessionStarting, renderTerminalPresentation } from "../terminalPresentation";

interface TerminalPanelProps {
  backendStatus: string;
  session: SessionInfo | null;
  onBackendUnavailable: () => void;
}

function TerminalPanel({ backendStatus, session, onBackendUnavailable }: TerminalPanelProps) {
  if (!session) {
    return <EmptyState message="Select a session to open its terminal." />;
  }
  return renderTerminalPresentation(
    session.lifecycle,
    () => (
      <CenterPanel
        backendStatus={backendStatus}
        sessionId={session.id}
        starting={isSessionStarting(session)}
        onBackendUnavailable={onBackendUnavailable}
      />
    ),
    () => <HistoricalTerminal sessionId={session.id} />,
  );
}

export default TerminalPanel;
