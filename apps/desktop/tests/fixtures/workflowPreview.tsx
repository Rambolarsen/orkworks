import { useState } from "react";
import { createRoot } from "react-dom/client";
import WorkflowOverview from "../../src/components/workflow/WorkflowOverview";
import type { Workflow, WorkflowTask } from "../../src/components/workflow/workflowPresentation";
import "../../src/styles/tokens.css";

const research: WorkflowTask = { id: "research", role: "Research", label: "Find the parser boundary", phase: "session", status: "Working", result: "Unassessed", gitReference: "preview/research", skills: [
  { id: "debug", name: "systematic-debugging", version: "fixture-v1", delivery: "unconfirmed", usage: "reported", source: "Synthetic agent report", required: true },
  { id: "design", name: "codebase-design", version: "fixture-v2", delivery: "loaded", usage: "unknown", source: "Synthetic delivery receipt" },
  { id: "test", name: "test-driven-development", version: "fixture-v1", delivery: "selected", usage: "unknown", source: "Synthetic selection" },
  { id: "review", name: "requesting-code-review", version: "fixture-v1", delivery: "stale", usage: "observed", source: "Historical synthetic receipt; current evidence unavailable" },
] };
const mixed: Workflow = { id: "preview-mixed", goal: "Make session startup understandable", stages: [
  { id: "research", revision: 1, title: "Understand the failure", approval: "Approved · research", tasks: [research,
    { ...research, id: "review", role: "Review", label: "Check the failure evidence", phase: "planned", status: "Waiting", blockedReason: "Research report required", dependencies: ["Find the parser boundary"], skills: [] },
    { ...research, id: "baseline", role: "Verification", label: "Retain the baseline", phase: "remembered", status: "Ended", result: "Meets requirements", skills: [ { id: "verify", name: "verification-before-completion", version: "fixture-v1", delivery: "loaded", usage: "observed", source: "Synthetic adapter invocation" } ] },
  ] },
  { id: "execution", revision: 1, title: "Implement and verify", approval: "Awaiting exact plan approval", tasks: [
    { ...research, id: "implement", role: "Implementation", label: "Explain startup failures", phase: "planned", status: "Planned", skills: [] },
    { ...research, id: "verify", role: "Verification", label: "Check recovery and regression behavior", phase: "planned", status: "Planned", skills: [] },
  ] },
] };
const edge: Workflow = { id: "preview-edge", goal: "Preserve work across revisions", parentUnavailable: true, stages: [
  { id: "same-plan", revision: 1, title: "Previous attempt", approval: "Superseded · retained history", tasks: [
    { ...research, id: "same-task", label: 'Missing session <img data-injected src=x>', phase: "unavailable", status: "Unavailable", skills: research.skills },
  ] },
  { id: "same-plan", revision: 2, title: "New proposal", approval: "Awaiting exact plan approval", tasks: [
    { ...research, id: "same-task", label: 'Missing session <img data-injected src=x>', phase: "starting", status: "Starting", skills: [] },
  ] },
] };
const dense: Workflow = { id: "preview-dense", goal: "Review a large workflow without losing the current task", stages: [
  { id: "dense", revision: 4, title: "Implementation and verification", approval: "Approved · synthetic", tasks: Array.from({ length: 36 }, (_, index) => ({
    ...research, id: `dense-${index}`, label: `Task ${index + 1}: Preserve exact configuration and evidence across a very long assignment with names-that-must-wrap-without-clipping-even-at-compact-widths`,
    blockedReason: index % 7 === 0 ? "Waiting for the declared predecessor result" : undefined,
  })) },
] };
const scenarios = { mixed, edge, dense };
function Preview() {
  const [scenario, setScenario] = useState<keyof typeof scenarios>("mixed");
  return <main className="workflow-preview"><div className="workflow-preview-banner"><strong>Synthetic workflow preview</strong>
    <label>Scenario <select data-scenario value={scenario} onChange={event => setScenario(event.target.value as keyof typeof scenarios)}>
      <option value="mixed">Research → execution</option><option value="edge">Missing sessions & revisions</option><option value="dense">Many tasks & long labels</option>
    </select></label></div><WorkflowOverview workflow={scenarios[scenario]} /></main>;
}
createRoot(document.getElementById("root")!).render(<Preview />);
