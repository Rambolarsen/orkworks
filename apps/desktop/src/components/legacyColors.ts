/**
 * Temporary color helpers. Inline-style call sites in SessionListPanel and
 * SessionDetailPanel still reach for these; phases 4 and 5 of the UI
 * substrate redesign rewrite those sites to class-driven CSS and delete
 * this file. Do not add new call sites.
 */

export function statusDotColor(status: string): string {
  if (status === "waiting_for_input" || status === "failed") return "#cc4444";
  if (status === "blocked") return "#d4d44e";
  if (status === "done") return "#4ec94e";
  if (status === "stale" || status === "idle") return "#666";
  if (status === "working" || status === "running" || status === "creating") return "#4ec94e";
  return "#666";
}

export function attentionBorderColor(status: string): string {
  if (status === "waiting_for_input" || status === "failed") return "#cc4444";
  if (status === "blocked") return "#d4d44e";
  if (status === "done") return "#4ec94e";
  if (status === "stale" || status === "idle") return "#4a4a4a";
  return "#3c3c3c";
}

export function sourceColor(source: string | undefined): string {
  if (source === "agent") return "#4ec94e";
  if (source === "peon") return "#57c7ff";
  return "#858585";
}
