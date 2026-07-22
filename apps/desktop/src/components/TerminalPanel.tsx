import CenterPanel from "./CenterPanel";
import HistoricalTerminal from "./HistoricalTerminal";
import EmptyState from "./EmptyState";
import type { SessionInfo } from "../api";
import { renderTerminalPresentation } from "../terminalPresentation";

interface TerminalPanelProps {
  backendStatus: string;
  session: SessionInfo | null;
}

function TerminalPanel({ backendStatus, session }: TerminalPanelProps) {
  if (!session) {
    return <EmptyState message="Select a live session to open its terminal." />;
  }
  return renderTerminalPresentation(
    session.lifecycle,
    () => <CenterPanel backendStatus={backendStatus} sessionId={session.id} />,
    () => <HistoricalTerminal sessionId={session.id} />,
  );
}

export default TerminalPanel;
