import CenterPanel from "./CenterPanel";
import EmptyState from "./EmptyState";
import type { SessionInfo } from "../api";

interface TerminalPanelProps {
  backendStatus: string;
  session: SessionInfo | null;
}

function TerminalPanel({ backendStatus, session }: TerminalPanelProps) {
  if (!session) {
    return <EmptyState message="Select a session to open its terminal." />;
  }
  return <CenterPanel backendStatus={backendStatus} sessionId={session.id} />;
}

export default TerminalPanel;
