import { useEffect, useRef, useState } from "react";
import type { KeyboardEvent } from "react";
import { initialPresentation, outline, transition } from "./workflowPresentation";
import type { OutlineNode, PresentationEvent, Workflow, WorkflowSkill } from "./workflowPresentation";
import "./WorkflowOverview.css";

const deliveryLabel = (skill: WorkflowSkill) => ({ selected: "Selected", unconfirmed: "Delivery unconfirmed", loaded: "Loaded", stale: "Evidence unavailable" })[skill.delivery];
const usageLabel = (skill: WorkflowSkill) => skill.delivery === "stale" ? "Usage unknown" : ({ unknown: "Usage unknown", reported: "Reported use", observed: "Observed use" })[skill.usage];
const phaseLabel = { planned: "Planned · not started", starting: "Starting · session not available", session: "Live session", remembered: "Remembered session", unavailable: "Session unavailable" };

export default function WorkflowOverview({ workflow }: { workflow: Workflow }) {
  return <Overview key={workflow.id} workflow={workflow} />;
}
function Overview({ workflow }: { workflow: Workflow }) {
  const [state, setState] = useState(() => initialPresentation(workflow));
  const [view, setView] = useState<"timeline" | "tree">("timeline");
  const elements = useRef(new Map<string, HTMLDivElement>());
  const detailsHeading = useRef<HTMLHeadingElement>(null);
  const typeahead = useRef({ text: "", at: 0 });
  const rows = outline(workflow, state.collapsed);
  const focused = rows.find(row => row.key === state.focusedKey) ?? rows[0];
  const inspected = rows.find(row => row.key === state.inspectedKey) ?? rows[0];
  const send = (event: PresentationEvent) => setState(current => transition(workflow, current, event));
  useEffect(() => { setState(current => transition(workflow, current, { type: "reconcile" })); }, [workflow]);

  const focus = (key: string) => {
    send({ type: "focus", key });
    elements.current.get(key)?.focus();
  };
  const inspect = (row: OutlineNode, enterDetails = false) => {
    send({ type: "inspect", key: row.key });
    if (enterDetails) requestAnimationFrame(() => detailsHeading.current?.focus());
  };
  const navigate = (event: KeyboardEvent, row: OutlineNode) => {
    const index = rows.findIndex(node => node.key === row.key);
    let target: string | undefined;
    switch (event.key) {
      case "ArrowDown": target = rows[Math.min(index + 1, rows.length - 1)].key; break;
      case "ArrowUp": target = rows[Math.max(index - 1, 0)].key; break;
      case "Home": target = rows[0].key; break;
      case "End": target = rows[rows.length - 1].key; break;
      case "ArrowRight":
        if (row.kind !== "task") {
          if (state.collapsed.includes(row.key)) send({ type: "toggle", key: row.key });
          else target = rows[index + 1]?.parentKey === row.key ? rows[index + 1].key : undefined;
        }
        break;
      case "ArrowLeft":
        if (row.kind !== "task" && !state.collapsed.includes(row.key)) send({ type: "toggle", key: row.key });
        else target = row.parentKey ?? undefined;
        break;
      case " ": inspect(row); break;
      case "Enter": inspect(row, true); break;
      default: {
        if (event.key.length !== 1 || event.ctrlKey || event.metaKey || event.altKey) return;
        const now = Date.now();
        const previous = now - typeahead.current.at < 700 ? typeahead.current.text : "";
        const text = previous + event.key.toLocaleLowerCase();
        typeahead.current = { text, at: now };
        const ordered = [...rows.slice(index + 1), ...rows.slice(0, index + 1)];
        const name = (node: OutlineNode) => node.task ? `${node.task.role} ${node.label}` : node.label;
        target = ordered.find(node => name(node).toLocaleLowerCase().startsWith(text) || node.label.toLocaleLowerCase().startsWith(text))?.key;
      }
    }
    event.preventDefault();
    if (target) focus(target);
  };
  const name = (row: OutlineNode) => row.task
    ? `${row.task.role}: ${row.label}. ${phaseLabel[row.task.phase]}. ${row.task.status}. ${row.task.result}`
    : row.stage ? `Stage: ${row.label}, revision ${row.stage.revision}. ${row.stage.approval}` : `Orchestrator: ${row.label}`;
  const task = inspected.task;
  return <section className="workflow-overview" data-view={view} aria-label="Workflow overview">
    <header className="workflow-header">
      <div><p className="workflow-eyebrow">WORKFLOW / SYNTHETIC DATA</p><h1>{workflow.goal}</h1>
        <p>One orchestrator. Declared work, in stage order.</p></div>
      <div className="workflow-view-switch" role="group" aria-label="Workflow presentation">
        <button data-view="timeline" aria-pressed={view === "timeline"} onClick={() => setView("timeline")}>Timeline</button>
        <button data-view="tree" aria-pressed={view === "tree"} onClick={() => setView("tree")}>Tree</button>
      </div>
    </header>
    <div className="workflow-summary"><span>{workflow.stages.length} {workflow.stages.length === 1 ? "stage" : "stages"}</span><span>{workflow.stages.reduce((sum, stage) => sum + stage.tasks.length, 0)} tasks</span>
      <strong>{rows[0].blockerCount} blocker{rows[0].blockerCount === 1 ? "" : "s"}</strong>
      {workflow.parentUnavailable && <span>Parent session unavailable</span>}</div>
    <div className="workflow-layout">
      <div className="workflow-outline-pane">
        <div role="tree" aria-label="Workflow tasks" className="workflow-outline">
          {rows.map(row => <div key={row.key} ref={element => { if (element) elements.current.set(row.key, element); else elements.current.delete(row.key); }}
            role="treeitem" aria-level={row.level} aria-posinset={row.position} aria-setsize={row.setSize}
            aria-label={name(row)} aria-selected={row.key === inspected.key}
            aria-expanded={row.kind === "task" ? undefined : !state.collapsed.includes(row.key)}
            tabIndex={row.key === focused.key ? 0 : -1} data-kind={row.kind} className="workflow-node"
            onFocus={() => send({ type: "focus", key: row.key })}
            onClick={() => { focus(row.key); inspect(row); }} onKeyDown={event => navigate(event, row)}>
            <span className="workflow-node-glyph" aria-hidden="true">{row.kind === "run" ? "◉" : row.kind === "stage" ? state.collapsed.includes(row.key) ? "▸" : "▾" : "○"}</span>
            <div className="workflow-node-body">
              <span className="workflow-node-role">{row.task?.role ?? (row.stage ? `Stage · revision ${row.stage.revision}` : "Orchestrator")}</span>
              <span className="workflow-node-label">{row.label}</span>
              <span className="workflow-node-status">{row.task ? `${phaseLabel[row.task.phase]} · ${row.task.status}` : row.stage?.approval ?? "Run overview"}</span>
              {row.task?.blockedReason && <span className="workflow-blocker">Blocked · {row.task.blockedReason}</span>}
              {row.kind !== "task" && row.blockerCount > 0 && <span className="workflow-blocker">{row.blockerCount} blocker{row.blockerCount === 1 ? "" : "s"} in this group</span>}
              {row.task && <div className="workflow-badges">{row.task.skills.slice(0, 3).map(skill =>
                <span key={skill.id} data-delivery={skill.delivery} className="workflow-skill">{skill.name} · {deliveryLabel(skill)}{skill.usage !== "unknown" && skill.delivery !== "stale" ? ` · ${usageLabel(skill)}` : ""}</span>)}
                {row.task.skills.length > 3 && <span>+{row.task.skills.length - 3} {row.task.skills.length === 4 ? "skill" : "skills"} · inspect for all</span>}</div>}
            </div>
          </div>)}
        </div>
        <div role="toolbar" aria-label={`Commands for ${focused.label}`} className="workflow-node-toolbar">
          <button onClick={() => inspect(focused, true)}>Inspect details</button>
          {focused.kind !== "task" && <button data-disclosure onClick={() => send({ type: "toggle", key: focused.key })}>{state.collapsed.includes(focused.key) ? "Expand group" : "Collapse group"}</button>}
          <button data-inspect-run onClick={() => inspect(rows[0])}>Inspect run</button>
        </div>
        {state.collapseReasons.length > 0 && <div role="status" data-collapse-reasons className="workflow-collapse-reasons">{state.collapseReasons.map(reason => <p key={reason}>{reason}</p>)}</div>}
        <p className="workflow-keyboard-hint">↑ ↓ to move · Space to inspect · Enter for details · ← → to fold or unfold</p>
      </div>
      <section className="workflow-details" data-workflow-details aria-label="Inspected workflow details" onKeyDown={event => {
        if (event.key === "Escape") { event.preventDefault(); focus(inspected.key); }
      }}>
        <p className="workflow-eyebrow">INSPECTING {task ? "TASK" : inspected.stage ? "STAGE" : "RUN"}</p>
        <h2 ref={detailsHeading} data-details-heading tabIndex={-1}>{inspected.label}</h2>
        <dl><dt>Role</dt><dd>{task?.role ?? (inspected.stage ? "Non-agent stage" : "Orchestrator")}</dd>
          <dt>Status</dt><dd>{task ? `${phaseLabel[task.phase]} · ${task.status}` : inspected.stage?.approval ?? "Synthetic run"}</dd>
          <dt>Result</dt><dd>{task?.result ?? "Unassessed"}</dd>
          {task?.blockedReason && <><dt>Blocker</dt><dd>{task.blockedReason}</dd></>}
          {task?.dependencies?.length ? <><dt>Declared dependencies</dt><dd>{task.dependencies.join(", ")}</dd></> : null}
          <dt>Git reference</dt><dd>{task?.gitReference ?? "Unavailable"}</dd>
          <dt>History</dt><dd>History unavailable</dd></dl>
        {task && <><h3>Skills <span>({task.skills.length})</span></h3>{task.skills.length ? task.skills.map(skill => <article key={skill.id} className="workflow-skill-detail">
          <h4>{skill.name}</h4><p>{skill.required ? "Required" : "Optional"} · {skill.version}</p><p>{deliveryLabel(skill)} · {usageLabel(skill)}</p><p>Source: {skill.source}</p>
        </article>) : <p>No selected skills.</p>}</>}
        {task && (task.phase === "session" || task.phase === "remembered") && <><button disabled data-terminal-command>{task.phase === "remembered" ? "Open history" : "Open terminal"}</button><p>Unavailable in preview.</p></>}
        {!task && <p className="workflow-detail-note">Inspect a task to see its assignment, skill evidence and declared dependencies. All displayed outcomes are supplied by synthetic fixtures.</p>}
        <p className="workflow-detail-note">Inspection does not select, acknowledge or start a session.</p>
      </section>
    </div>
  </section>;
}
