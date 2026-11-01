import { useState } from "react";
import { GitBranch } from "lucide-react";
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
  onUseLatest: (kind: PlanDocumentKind) => Promise<void>;
}

export default function BranchDocumentSteps({ documents, onOpen, onUseLatest }: Props) {
  const [pending, setPending] = useState<PlanDocumentKind | null>(null);
  const [error, setError] = useState<string | null>(null);
  return <section className="branch-documents" aria-label="Design and implementation documents">
    <h3>Design &amp; plan</h3>
    {documents.branch && <p className="branch-documents-branch"><GitBranch size={14} aria-hidden="true" />{documents.branch}</p>}
    {documents.sharedSessions > 1 && <p className="detail-muted">Shared by {documents.sharedSessions} sessions</p>}
    <ol className="branch-document-steps">
      {(["design", "implementation"] as const).map((kind, index) => {
        const step = documents.steps.find(step => step.kind === kind);
        return <li key={kind}>
          <span className="branch-document-number" aria-hidden="true">{index + 1}</span>
          <div className="branch-document-body">
            <button type="button" className="branch-document-open" data-document-kind={kind}
              disabled={!step?.available} onClick={() => onOpen(kind)}>
              <span className="branch-document-label">{kind === "design" ? "Design / specification" : "Implementation plan"}</span>
              <strong>{step?.title || "No document found yet"}</strong>
              <span className="branch-document-state">{step?.available ? "Available · Open document" : step?.pinned ? "Selected document unavailable" : "No document found yet"}</span>
            </button>
            {step?.source && <details className="branch-document-source">
              <summary>{sources[step.source] || "Saved document"}{step.pinned ? " · Pinned" : ""}</summary>
              {step.fileName && <p>{step.fileName}</p>}
            </details>}
            {step?.pinned && <button type="button" className="branch-document-latest" disabled={pending !== null}
              onClick={() => {
                if (pending !== null) return;
                setPending(kind); setError(null);
                void onUseLatest(kind).catch(() => setError("Couldn’t restore the latest document. Try again."))
                  .finally(() => setPending(null));
              }}>{pending === kind ? "Updating…" : "Use latest"}</button>}
          </div>
        </li>;
      })}
    </ol>
    {documents.notice && <p className="branch-document-notice">{documents.notice}</p>}
    {error && <p role="alert">{error}</p>}
  </section>;
}
