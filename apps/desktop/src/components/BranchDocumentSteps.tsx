import { useState } from "react";
import { ChevronRight, FileText, GitBranch, NotebookPen } from "lucide-react";
import type { BranchDocuments, PlanDocumentKind } from "../api";

const sources: Record<string, string> = {
  git_history: "Found in Git history", git_worktree: "Found in worktree changes",
  agent_reported: "Reported by agent", hook_reported: "Reported by coding tool",
  terminal_fallback: "Detected in terminal", user_selected: "Selected by you", legacy: "Saved session document",
};

interface Props {
  sessionId: string;
  documents: BranchDocuments;
  onOpen: (kind: PlanDocumentKind) => void;
  onUseLatest: (kind: PlanDocumentKind, selectionId?: string) => Promise<void>;
}

export default function BranchDocumentSteps({ documents, onOpen, onUseLatest }: Props) {
  const [pending, setPending] = useState<PlanDocumentKind | null>(null);
  const [error, setError] = useState<string | null>(null);
  return <details className="branch-documents" aria-label="Design and implementation documents">
    <summary className="session-menu-item">
      <span className="session-menu-icon"><NotebookPen size={16} aria-hidden="true" /></span>
      <span className="session-menu-text">
        <span className="session-menu-label">Design &amp; plan</span>
        <span className="session-menu-description">Specification and implementation plan</span>
      </span>
      <ChevronRight className="branch-documents-chevron" size={14} aria-hidden="true" />
    </summary>
    <div className="branch-documents-content">
      <div className="branch-documents-meta">
        {documents.branch && <p className="branch-documents-branch"><GitBranch size={14} aria-hidden="true" />{documents.branch}</p>}
        {documents.sharedSessions > 1 && <p className="detail-muted">Shared by {documents.sharedSessions} sessions</p>}
      </div>
      <ol className="branch-document-steps">
        {(["design", "implementation"] as const).map(kind => {
          const step = documents.steps.find(step => step.kind === kind);
          return <li key={kind}>
            <div className="branch-document-body">
              <button type="button" className="branch-document-open" data-document-kind={kind}
                disabled={!step?.available} onClick={() => onOpen(kind)}>
                <FileText size={16} aria-hidden="true" />
                <span className="branch-document-text">
                  <span className="branch-document-label">{kind === "design" ? "Design / specification" : "Implementation plan"}</span>
                  {step?.title && <strong>{step.title}</strong>}
                  <span className="branch-document-state">{step?.available ? "Available · Open document" : step?.pinned ? "Selected document unavailable" : "No document found yet"}</span>
                </span>
                {step?.available && <ChevronRight size={14} aria-hidden="true" />}
              </button>
              {step?.source && <details className="branch-document-source">
                <summary>{sources[step.source] || "Saved document"}{step.pinned ? " · Pinned" : ""}</summary>
                {step.fileName && <p>{step.fileName}</p>}
              </details>}
              {step?.pinned && <button type="button" className="branch-document-latest" disabled={pending !== null}
                onClick={() => {
                  if (pending !== null) return;
                  setPending(kind); setError(null);
                  void onUseLatest(kind, step.selectionId ?? undefined).catch(() => setError("Couldn’t restore the latest document. Try again."))
                    .finally(() => setPending(null));
                }}>{pending === kind ? "Updating…" : "Use latest"}</button>}
            </div>
          </li>;
        })}
      </ol>
      {documents.notice && <p className="branch-document-notice">{documents.notice}</p>}
      {error && <p role="alert">{error}</p>}
    </div>
  </details>;
}
