// Presentation-only inputs. These are not orchestration wire records.
export interface WorkflowSkill {
  id: string;
  name: string;
  version: string;
  delivery: "selected" | "unconfirmed" | "loaded" | "stale";
  usage: "unknown" | "reported" | "observed";
  source: string;
  required?: boolean;
}
export interface WorkflowTask {
  id: string;
  role: string;
  label: string;
  phase: "planned" | "starting" | "session" | "remembered" | "unavailable";
  status: string;
  result: "Meets requirements" | "Needs rework" | "Unassessed";
  blockedReason?: string;
  dependencies?: readonly string[];
  gitReference?: string;
  skills: readonly WorkflowSkill[];
}
export interface WorkflowStage {
  id: string;
  revision: number;
  title: string;
  approval: string;
  tasks: readonly WorkflowTask[];
}
export interface Workflow {
  id: string;
  goal: string;
  parentUnavailable?: boolean;
  stages: readonly WorkflowStage[];
}
export interface OutlineNode {
  key: string;
  parentKey: string | null;
  kind: "run" | "stage" | "task";
  label: string;
  level: number;
  position: number;
  setSize: number;
  blockerCount: number;
  stage?: WorkflowStage;
  task?: WorkflowTask;
}
const rootKey = (workflow: Workflow) => JSON.stringify([workflow.id]);
const stageKey = (workflow: Workflow, stage: WorkflowStage) => JSON.stringify([workflow.id, stage.id, stage.revision]);
const belongsTo = (key: string, ancestor: string) => {
  const parts: unknown[] = JSON.parse(key);
  const parent: unknown[] = JSON.parse(ancestor);
  return parts.length > parent.length && parent.every((part, index) => part === parts[index]);
};
export function outline(workflow: Workflow, collapsed: readonly string[]): OutlineNode[] {
  const root = rootKey(workflow);
  const count = (tasks: readonly WorkflowTask[]) => tasks.filter(task => Boolean(task.blockedReason)).length;
  const rows: OutlineNode[] = [{ key: root, parentKey: null, kind: "run", label: workflow.goal,
    level: 1, position: 1, setSize: 1, blockerCount: workflow.stages.reduce((sum, stage) => sum + count(stage.tasks), 0) }];
  if (collapsed.includes(root)) return rows;
  workflow.stages.forEach((stage, index) => {
    const key = stageKey(workflow, stage);
    rows.push({ key, parentKey: root, kind: "stage", label: stage.title, level: 2,
      position: index + 1, setSize: workflow.stages.length, blockerCount: count(stage.tasks), stage });
    if (collapsed.includes(key)) return;
    stage.tasks.forEach((task, taskIndex) => rows.push({ key: JSON.stringify([workflow.id, stage.id, stage.revision, task.id]),
      parentKey: key, kind: "task", label: task.label, level: 3, position: taskIndex + 1,
      setSize: stage.tasks.length, blockerCount: task.blockedReason ? 1 : 0, stage, task }));
  });
  return rows;
}
export interface PresentationState {
  focusedKey: string;
  inspectedKey: string;
  collapsed: readonly string[];
  collapseReasons: readonly string[];
}
export function initialPresentation(workflow: Workflow): PresentationState {
  return { focusedKey: rootKey(workflow), inspectedKey: rootKey(workflow), collapsed: [], collapseReasons: [] };
}
export type PresentationEvent =
  | { type: "focus" | "inspect" | "toggle"; key: string }
  | { type: "reconcile" };
export function transition(workflow: Workflow, state: PresentationState, event: PresentationEvent): PresentationState {
  const rows = outline(workflow, state.collapsed);
  if (event.type === "reconcile") {
    const allKeys = new Set(outline(workflow, []).map(row => row.key));
    const collapsed = state.collapsed.filter(key => allKeys.has(key));
    const visible = new Set(outline(workflow, collapsed).map(row => row.key));
    const fallback = (key: string): string => {
      if (visible.has(key)) return key;
      const parts: unknown[] = JSON.parse(key);
      if (parts.length === 4) parts.pop();
      const stage = JSON.stringify(parts);
      return visible.has(stage) ? stage : rootKey(workflow);
    };
    return { ...state, focusedKey: fallback(state.focusedKey), inspectedKey: fallback(state.inspectedKey), collapsed, collapseReasons: [] };
  }
  const row = rows.find(node => node.key === event.key);
  if (!row) return state;
  if (event.type === "focus") return { ...state, focusedKey: event.key, collapseReasons: [] };
  if (event.type === "inspect") return { ...state, inspectedKey: event.key, collapseReasons: [] };
  if (row.kind === "task") return state;
  if (state.collapsed.includes(row.key)) return { ...state, collapsed: state.collapsed.filter(key => key !== row.key), collapseReasons: [] };
  const collapseReasons: string[] = [];
  if (belongsTo(state.inspectedKey, row.key)) collapseReasons.push("Inspect the run or another stage to collapse this group.");
  if (belongsTo(state.focusedKey, row.key)) collapseReasons.push("Move focus to the stage heading to collapse this group.");
  return collapseReasons.length ? { ...state, collapseReasons } : { ...state, collapsed: [...state.collapsed, row.key], collapseReasons: [] };
}
