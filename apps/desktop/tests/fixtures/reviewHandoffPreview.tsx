import { useReducer, useRef, useState } from "react";
import ReviewHandoffPreview from "../../src/components/review/ReviewHandoffPreview";
import { initialReviewHandoffState, reviewHandoffReducer, type ReviewHandoffSnapshot } from "../../src/components/review/reviewHandoffPresentation";

const base: ReviewHandoffSnapshot = {
  scope: "demo/backend-1", proposalId: "review-1", revision: "1",
  source: { id: "S41", runtime: "S41/3", title: "Retry implementation", tool: "Codex" },
  reviewer: { label: "Proposed new session R7", role: "Review", tool: "Copilot", model: "Configured reviewer · availability unverified" },
  sourceLocation: { worktree: "/work/demo", branch: "feature/retry" }, reviewerLocation: { worktree: "/work/demo", branch: "feature/retry" },
  artifact: { state: "available", path: "specs/retry.md", anchor: "/work/demo", version: "A", pinned: false },
  instruction: "Review S41’s retry change against specs/retry.md (version A).\nCheck missing requirements, unsafe retry assumptions and verification gaps.\nReport findings and evidence; shared edits have unknown authorship. Do not infer ownership or approval from companion S42.",
  unavailableReason: "Unavailable in preview: reviewer capability and launch approval gates are not satisfied.",
};
function scenarioSnapshot(name: string): ReviewHandoffSnapshot {
  if (name === "missing") return { ...base, artifact: { state: "missing" }, instruction: "Review the selected plan. Plan reference: missing; this instruction cannot be dispatched.", unavailableReason: "A plan is required for this plan-review proposal." };
  if (name === "unavailable") return { ...base, artifact: { state: "unavailable", reason: "Anchor unavailable" }, reviewerLocation: { worktree: null, branch: null }, reviewer: { ...base.reviewer, tool: "Unavailable", model: "Unknown" } };
  if (name === "long") return { ...base, source: { ...base.source, title: "Long-source-".repeat(20) }, artifact: { state: "available", path: "specs/" + "long-plan-".repeat(30) + ".md", anchor: "/work/demo", version: "A", pinned: true }, instruction: '<img data-injected src=x>\n' + "Inspect the complete instruction and a-long-unbroken-reference-that-must-wrap-without-clipping. ".repeat(35) };
  return base;
}
export default function ReviewHandoffScenario() {
  const [state, dispatch] = useReducer(reviewHandoffReducer, initialReviewHandoffState);
  const [scenario, setScenario] = useState("current");
  const [latest, setLatest] = useState(base);
  const [removed, setRemoved] = useState(false);
  const [delay, setDelay] = useState(false);
  const [pendingCount, setPendingCount] = useState(0);
  const pending = useRef(new Map<number, () => void>());
  const invoker = useRef<HTMLButtonElement>(null);
  const heading = useRef<HTMLHeadingElement>(null);
  const focusFrame = useRef<number | null>(null);
  const restoreFocus = () => {
    if (focusFrame.current !== null) cancelAnimationFrame(focusFrame.current);
    focusFrame.current = requestAnimationFrame(() => { focusFrame.current = null; (invoker.current ?? heading.current)?.focus(); });
  };
  const open = () => {
    if (focusFrame.current !== null) cancelAnimationFrame(focusFrame.current);
    focusFrame.current = null;
    dispatch({ type: "open", snapshot: latest });
  };
  const refresh = () => {
    const request = state.epoch + 1;
    dispatch({ type: "refresh-started" });
    const snapshot = latest;
    const response = delay ? new Promise<ReviewHandoffSnapshot>(resolve => {
      pending.current.set(request, () => resolve(snapshot)); setPendingCount(pending.current.size);
    }) : Promise.resolve(snapshot);
    void response.then(snapshot => dispatch({ type: "refresh-completed", request, snapshot }));
  };
  const complete = (newest: boolean) => {
    const keys = [...pending.current.keys()]; const key = newest ? keys.at(-1) : keys[0];
    if (key === undefined) return;
    const resolve = pending.current.get(key); pending.current.delete(key); setPendingCount(pending.current.size); resolve?.();
  };
  return <div className="handoff-fixture"><strong>Synthetic review handoff preview</strong>
    <h1 ref={heading} tabIndex={-1} data-recommendations-heading>Recommendations</h1>
    <p>Selected synthetic session S41 · unread 2 · attention unchanged. Companion S42 shares feature/retry; concurrent edit attribution is unknown.</p>
    <div className="handoff-fixture-controls"><label>Review scenario <select data-handoff-scenario value={scenario} onChange={event => {
      setScenario(event.target.value); setLatest(scenarioSnapshot(event.target.value)); setRemoved(false); dispatch({ type: "reset" });
    }}><option value="current">Shared branch</option><option value="missing">No selected plan</option><option value="unavailable">Unavailable context</option><option value="long">Long labels and instruction</option></select></label>
      {!removed && <button ref={invoker} data-handoff-open disabled={state.snapshot !== null} onClick={open}>Preview review handoff</button>}
      <button data-change-plan onClick={() => { setLatest({ ...latest, revision: "2", artifact: { state: "available", path: "specs/retry.md", anchor: "/work/demo", version: "B", pinned: false }, instruction: "Review S41 against specs/retry.md (version B). Report findings and evidence." }); dispatch({ type: "changed", reason: "Plan changed from version A to version B" }); }}>Change plan</button>
      <button data-replace-runtime onClick={() => { setLatest({ ...latest, source: { ...latest.source, runtime: latest.source.runtime + "-replaced" } }); dispatch({ type: "reset" }); restoreFocus(); }}>Replace runtime</button>
      <button data-replace-workspace onClick={() => { setLatest({ ...latest, scope: latest.scope + "-replaced" }); dispatch({ type: "reset" }); restoreFocus(); }}>Replace workspace</button>
      <button data-remove-recommendation onClick={() => { setRemoved(true); dispatch({ type: "reset" }); restoreFocus(); }}>Remove recommendation</button>
      <label><input type="checkbox" data-delay-refresh checked={delay} onChange={e => setDelay(e.target.checked)} />Delay refresh</label>
      <button data-complete-oldest disabled={!pendingCount} onClick={() => complete(false)}>Complete oldest refresh</button>
      <button data-complete-newest disabled={!pendingCount} onClick={() => complete(true)}>Complete newest refresh</button>
    </div>
    {state.snapshot && <ReviewHandoffPreview state={state} onRefresh={refresh} onClose={() => { dispatch({ type: "close" }); restoreFocus(); }} />}
  </div>;
}
