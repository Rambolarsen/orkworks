import type { MemoryState, ResumeOption, ResumeStrategy, SessionInfo } from "./api";

/** Canonical vocabulary. One word per concept. */
export const VOCAB = {
  workspace: "Workspace",
  openWorkspace: "Open workspace…",
  switchWorkspace: "Switch workspace",
  session: "Session",
  terminal: "Terminal",
  newSession: "New session",
  resumeChooserTitle: "Ways to resume",
  resumeBestTag: "Best",
  resumeNoteRemembered: "Not live · OrkWorks kept the memory, not the process.",
  cueReplyInTerminal: "Reply in the terminal to continue.",
  reviewDiffAction: "Review diff",
  markHandledAction: "Mark handled",
  diffReviewComingSoon: "Diff review isn't wired up yet — tracked as a follow-up.",
  markHandledComingSoon: "Marking sessions handled isn't wired up yet — tracked as a follow-up.",
} as const;

/** Plain-language attention label. Pairs with attentionTone() for visual weight. */
export function attentionLabel(status: string): string {
  switch (status) {
    case "waiting_for_input": return "Needs you";
    case "checking_capacity": return "Checking capacity";
    case "capped":            return "Capped";
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
  | "failed"
  | "done"
  | "working"
  | "idle"
  | "neutral";

export function attentionTone(status: string): AttentionTone {
  switch (status) {
    case "waiting_for_input":
      return "needs-you";
    case "failed":
      return "failed";
    case "checking_capacity":
    case "capped":
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

/** Distilled "what's going on" sentence for the Detail panel's situation hero. */
export function situationHeadline(session: SessionInfo): string {
  return (
    session.detectedQuestion ||
    session.blockerDescription ||
    session.summary ||
    session.nextAction ||
    "No additional detail recorded."
  );
}

/**
 * Raw terminal-excerpt quote for the situation hero — the peon's detected options for a live
 * question, or the raw failing command/test. Gated on tone, not just field presence: the backend
 * keeps suggestedOptions/failedTest/failedCommand sticky across peon updates (each merges via
 * `.or(previous)`), so a session that answered its question and later failed would otherwise still
 * show the stale prompt options instead of the fresh failure.
 */
export function situationTail(session: SessionInfo, tone: AttentionTone): string | undefined {
  if (tone === "failed") return session.failedTest || session.failedCommand;
  if (tone === "needs-you" && session.suggestedOptions?.length) return session.suggestedOptions.join("  ·  ");
  return undefined;
}

/**
 * One row in the Detail panel's resume chooser. Deliberately has no `strategy`
 * field yet — the resume API only takes a session id, not a strategy, so every
 * clickable row resumes the same way regardless of which one is picked. See #97.
 */
export interface ResumeChoice {
  label: string;
  sub: string;
  recommended?: boolean;
  unavailable?: boolean;
}

/** Plain-language sub-text for an available resume strategy. */
function resumeStrategySub(strategy: ResumeStrategy, session: SessionInfo): string {
  const folder = session.cwd.split("/").pop() || session.cwd;
  switch (strategy) {
    case "exact":       return "Reattach to this exact session";
    case "latest_cwd":  return `Newest session in ${folder}`;
    case "latest_repo": return "Newest session across the repo";
    case "none":        return "";
  }
}

/** Maps the backend's resumeOptions (or a synthesized fallback) into chooser rows. */
export function resumeChoices(session: SessionInfo): ResumeChoice[] {
  const options: ResumeOption[] = session.resumeOptions?.length
    ? session.resumeOptions
    : session.resumeStrategy !== "none"
      ? [{ strategy: session.resumeStrategy, label: resumeActionLabel(session.resumeStrategy), available: true, preferred: true }]
      : [];

  return options.map((o) => ({
    label: o.label,
    sub: o.available ? resumeStrategySub(o.strategy, session) : (o.reason ?? "Not available"),
    recommended: o.available && o.preferred,
    unavailable: !o.available,
  }));
}

/** The Detail panel's single "action zone" surface — at most one move per session. */
export type DetailActionZone =
  | { kind: "none" }
  | { kind: "cue"; text: string }
  | { kind: "buttons" }
  | { kind: "resume"; options: ResumeChoice[]; note?: string };

export function detailActionZone(session: SessionInfo, tone: AttentionTone): DetailActionZone {
  if (session.memoryState === "resumable" || session.memoryState === "remembered") {
    const options = resumeChoices(session);
    if (options.length === 0) return { kind: "none" };
    return {
      kind: "resume",
      options,
      note: session.memoryState === "remembered" ? VOCAB.resumeNoteRemembered : undefined,
    };
  }
  if (tone === "needs-you") return { kind: "cue", text: VOCAB.cueReplyInTerminal };
  if (tone === "done") return { kind: "buttons" };
  return { kind: "none" };
}
