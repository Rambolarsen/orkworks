import { isHistoricalSession } from "../sessionRuntimeState";
import CenterPanel from "./CenterPanel";
import HistoricalTerminal from "./HistoricalTerminal";
import EmptyState from "./EmptyState";
import type { SessionInfo } from "../api";
import { isSessionStarting } from "../terminalPresentation";
import { startupFailureMessage } from "../startupDiagnostics";

interface TerminalPanelProps {
  backendStatus: string;
  session: SessionInfo | null;
  onBackendUnavailable: () => void;
  onRetryBackend: () => void;
  onSelectPlan?: (sessionId: string, path: string) => Promise<void>;
}

function TerminalPanel({ backendStatus, session, onBackendUnavailable, onRetryBackend, onSelectPlan }: TerminalPanelProps) {
  if (!session) {
    return <EmptyState message="Select a session to open its terminal." />;
  }
  const terminal = isHistoricalSession(session) ? (
    <HistoricalTerminal sessionId={session.id} onSelectPlan={onSelectPlan} />
  ) : (
    <CenterPanel
      backendStatus={backendStatus}
      sessionId={session.id}
      starting={isSessionStarting(session)}
      onBackendUnavailable={onBackendUnavailable}
      onRetryBackend={onRetryBackend}
      onSelectPlan={onSelectPlan}
    />
  );
  const diagnostic = startupFailureMessage(session.startupFailure);
  return diagnostic ? (
    <div className="terminal-session-view">
      <aside className="startup-failure-notice" role="status">
        <strong>Session could not start</strong>
        <p>{diagnostic}</p>
      </aside>
      {terminal}
    </div>
  ) : terminal;
}

export default TerminalPanel;
