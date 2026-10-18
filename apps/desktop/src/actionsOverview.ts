import { isHistoricalSession } from "./sessionRuntimeState.ts";
import type { SessionInfo, WorkflowRecommendation } from "./api.ts";
import { sessionAttentionStatus } from "./sessionSort.ts";
import { filterPanelRecommendations } from "./taskmaster.ts";

export type ActionsSourceStatus = "loading" | "ready" | "partial" | "unavailable";
export interface ActionRow {
  key: string;
  source: "session" | "recommendation";
  id: string;
  group: string;
  title: string;
  summary: string;
  revision: string;
  unavailable: boolean;
}

const GROUPS: Record<string, string> = {
  needs_you: "Needs You", waiting_for_input: "Needs You", blocked: "Blocked", failed: "Failed",
  done: "Done", stale: "Stale", working: "Working", idle: "Idle",
};
const ORDER = ["Needs You", "Blocked", "Failed", "Done", "Stale", "Working", "Idle"];

function countLabel(label: string, count: number, status: ActionsSourceStatus): string {
  const value = status === "ready" ? count : status === "partial" ? `${count} known (partial)` : status;
  return `${label}: ${value}`;
}

export function buildActionsOverview(
  sessions: readonly SessionInfo[], sessionsStatus: ActionsSourceStatus,
  recommendations: WorkflowRecommendation[], recommendationsStatus: ActionsSourceStatus,
) {
  const seen = new Set<string>();
  const sessionRows: ActionRow[] = [];
  for (const session of sessions) {
    // Remembered attention is historical, and capacity has its own destination.
    if (isHistoricalSession(session) || seen.has(session.id)) continue;
    seen.add(session.id);
    const group = GROUPS[sessionAttentionStatus(session)];
    if (!group) continue;
    const summary = session.detectedQuestion || session.blockerDescription || session.summary || session.nextAction || "";
    sessionRows.push({
      key: `session:${session.id}`, source: "session", id: session.id, group, title: session.label, summary,
      revision: JSON.stringify([session.lifecycle, session.attention, session.status, session.label, summary, session.lastActivityAt]),
      unavailable: sessionsStatus === "loading" || sessionsStatus === "unavailable",
    });
  }
  // Stable sorting preserves the existing session order within each attention group.
  sessionRows.sort((a, b) => ORDER.indexOf(a.group) - ORDER.indexOf(b.group));
  seen.clear();
  const recommendationRows: ActionRow[] = [];
  for (const recommendation of filterPanelRecommendations(recommendations, "all")) {
    if (seen.has(recommendation.id)) continue;
    seen.add(recommendation.id);
    recommendationRows.push({
      key: `recommendation:${recommendation.id}`, source: "recommendation", id: recommendation.id,
      group: "Recommendations", title: recommendation.title, summary: recommendation.summary,
      revision: JSON.stringify([recommendation.updatedAt, recommendation.status, recommendation.title, recommendation.summary]),
      unavailable: recommendationsStatus === "loading" || recommendationsStatus === "unavailable",
    });
  }
  const attentionCount = sessionRows.filter(row => ["Needs You", "Blocked", "Failed"].includes(row.group)).length;
  return {
    rows: [...sessionRows, ...recommendationRows],
    sessionCount: sessionsStatus === "ready" ? attentionCount : null,
    recommendationCount: recommendationsStatus === "ready" ? recommendationRows.length : null,
    sessionCountLabel: countLabel("Session attention", attentionCount, sessionsStatus),
    recommendationCountLabel: countLabel("Recommendations", recommendationRows.length, recommendationsStatus),
  };
}
export type ActionsOverviewModel = ReturnType<typeof buildActionsOverview>;
