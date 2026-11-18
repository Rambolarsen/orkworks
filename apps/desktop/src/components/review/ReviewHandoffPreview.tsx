import { useEffect, useId, useRef } from "react";
import type { ReviewHandoffState } from "./reviewHandoffPresentation";
import "./ReviewHandoffPreview.css";

export default function ReviewHandoffPreview({ state, onClose, onRefresh }: {
  state: ReviewHandoffState; onClose: () => void; onRefresh: () => void;
}) {
  const heading = useRef<HTMLHeadingElement>(null);
  const status = useRef<HTMLParagraphElement>(null);
  const id = useId();
  useEffect(() => { heading.current?.focus(); }, []);
  useEffect(() => { if (state.refreshCount > 0) status.current?.focus(); }, [state.refreshCount]);
  const snapshot = state.snapshot;
  if (!snapshot) return null;
  const artifact = snapshot.artifact;
  const location = (label: string, value: typeof snapshot.sourceLocation) => <div>
    <dt>{label}</dt><dd>{value.worktree ?? "Working location unavailable"}<br />{value.branch ?? "Branch unavailable"}</dd>
  </div>;
  return <section className="handoff-preview" data-handoff-detail aria-labelledby={id}
    onKeyDown={event => { if (event.key === "Escape") { event.stopPropagation(); onClose(); } }}>
    <header><div><p className="handoff-eyebrow">Review recommendation · synthetic</p>
      <h2 id={id} ref={heading} tabIndex={-1} data-handoff-heading>Preview review handoff</h2></div>
      <button type="button" onClick={onClose} data-handoff-close aria-label="Close review handoff preview">Close</button></header>
    <dl className="handoff-fields">
      <div><dt>Source session</dt><dd>{snapshot.source.title} · {snapshot.source.id}<br />{snapshot.source.tool} · runtime {snapshot.source.runtime}</dd></div>
      <div><dt>Proposed reviewer · Not started</dt><dd>{snapshot.reviewer.label} · {snapshot.reviewer.role}<br />Coding tool: {snapshot.reviewer.tool}<br />Model: {snapshot.reviewer.model}</dd></div>
      {location("Source worktree / branch", snapshot.sourceLocation)}
      {location("Proposed review working directory / branch", snapshot.reviewerLocation)}
      <div><dt>Plan / specification</dt><dd>{artifact.state === "available" ? <>{artifact.path}<br />Anchored to {artifact.anchor}<br />Inspected version {artifact.version} · {artifact.pinned ? "Pinned" : "Latest selection"}</>
        : artifact.state === "missing" ? "No plan selected" : <>Plan unavailable · {artifact.reason}</>}</dd></div>
      <div><dt>Proposal</dt><dd>{snapshot.proposalId} · revision {snapshot.revision}<br />Role / skill configuration is separate from this user instruction.</dd></div>
    </dl>
    <p ref={status} tabIndex={-1} role="status" aria-live="polite" aria-atomic="true" data-handoff-status className="handoff-status">
      {state.request !== null ? "Refreshing preview. " : state.refreshCount > 0 && !state.staleReason ? "Synthetic context refreshed. " : "Synthetic context. "}
      {state.staleReason ? <>Review context changed: {state.staleReason}. Previous inspected context below is stale.</> : "Inspection grants no approval."}
    </p>
    <details open data-handoff-instruction><summary>{state.staleReason ? "Previous proposed instruction · stale" : "Proposed instruction"}</summary><pre>{snapshot.instruction}</pre></details>
    <footer><button type="button" onClick={onRefresh} data-handoff-refresh>Refresh preview</button>
      <p id={`${id}-unavailable`}>{snapshot.unavailableReason}</p>
      <button type="button" disabled aria-describedby={`${id}-unavailable`} data-handoff-continue>Continue to existing approval</button>
      <p>Opening, reading and refreshing send no prompt.</p></footer>
  </section>;
}
