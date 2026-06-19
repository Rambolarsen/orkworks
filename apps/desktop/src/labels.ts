import type { MemoryState, ResumeStrategy } from "./api";

/** Canonical vocabulary. One word per concept. */
export const VOCAB = {
  workspace: "Workspace",
  openWorkspace: "Open workspace…",
  switchWorkspace: "Switch workspace",
  session: "Session",
  terminal: "Terminal",
  newSession: "New session",
} as const;

/** Plain-language attention label. Pairs with attentionTone() for visual weight. */
export function attentionLabel(status: string): string {
  switch (status) {
    case "waiting_for_input": return "Needs you";
    case "blocked":           return "Blocked";
    case "failed":            return "Failed";
    case "done":              return "Done";
    case "stale":             return "Idle";
    case "idle":              return "Idle";
    case "working":           return "Working";
    case "running":           return "Running";
    case "creating":          return "Starting";
    case "ended":             return "Ended";
    case "killed":            return "Killed";
    case "error":             return "Error";
    default:                  return "Unknown";
  }
}

export type AttentionTone =
  | "needs-you"
  | "blocked"
  | "done"
  | "working"
  | "idle"
  | "neutral";

export function attentionTone(status: string): AttentionTone {
  switch (status) {
    case "waiting_for_input":
    case "failed":
      return "needs-you";
    case "blocked":
      return "blocked";
    case "done":
      return "done";
    case "working":
    case "running":
    case "creating":
      return "working";
    case "stale":
    case "idle":
      return "idle";
    default:
      return "neutral";
  }
}

export function memoryStateLabel(s: MemoryState): string {
  switch (s) {
    case "live":        return "Live";
    case "resumable":   return "Resumable";
    case "remembered":  return "Remembered";
    case "unsupported": return "—";
  }
}

export function resumeActionLabel(strategy: ResumeStrategy): string {
  switch (strategy) {
    case "exact":       return "Resume session";
    case "latest_cwd":  return "Resume latest in folder";
    case "latest_repo": return "Resume latest in repo";
    case "none":        return "Resume unavailable";
  }
}

export function sourceLabel(source: string | undefined): string {
  if (source === "agent") return "Agent";
  if (source === "peon")  return "Peon";
  if (!source)            return "Unknown";
  return source.charAt(0).toUpperCase() + source.slice(1);
}

/** "Agent · 95% confidence" or "Agent" when confidence is unknown. */
export function sourceWithConfidence(
  source: string | undefined,
  confidence: number | undefined,
): string {
  if (confidence === undefined) return sourceLabel(source);
  const c = Math.round(confidence * 100);
  return `${sourceLabel(source)} · ${c}% confidence`;
}

/** Relative-time formatting for "last activity". Local-only; no library. */
export function relativeTime(iso: string | undefined, now: Date = new Date()): string {
  if (!iso) return "";
  const t = new Date(iso).getTime();
  if (Number.isNaN(t)) return "";
  const diffSec = Math.max(0, Math.round((now.getTime() - t) / 1000));
  if (diffSec < 5)     return "just now";
  if (diffSec < 60)    return `${diffSec}s ago`;
  if (diffSec < 3600)  return `${Math.round(diffSec / 60)}m ago`;
  if (diffSec < 86400) return `${Math.round(diffSec / 3600)}h ago`;
  return `${Math.round(diffSec / 86400)}d ago`;
}
