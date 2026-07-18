import type { MemoryState, ResumeOption, ResumeStrategy, SessionInfo } from "./api";

const SECOND_MS = 1000;
const MINUTE_MS = 60 * SECOND_MS;
const HOUR_MS = 60 * MINUTE_MS;
export const DAY_MS = 24 * HOUR_MS;

/** Clamp a scheduled wake-up to always be in the future (min 1ms). Shared by both refresh schedulers. */
export function delayUntil(targetMs: number, nowMs: number): number {
  return Math.max(1, targetMs - nowMs);
}

/** Smaller of two nullable scheduled delays, treating null as "no constraint". Shared by both refresh schedulers. */
export function minDelay(current: number | null, candidate: number | null): number | null {
  if (candidate === null) return current;
  if (current === null) return candidate;
  return Math.min(current, candidate);
}

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
    case "needs_you":         return "Needs you";
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
    case "neutral":           return "";
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

/**
 * Tones that interrupt: only these spell the attention word out on the
 * session row. For working/done/idle the indicator glyph already says it —
 * repeating the word was clutter.
 */
export function isLoudTone(tone: AttentionTone): boolean {
  return tone === "needs-you" || tone === "failed" || tone === "blocked";
}

export function attentionTone(status: string): AttentionTone {
  switch (status) {
    case "needs_you":
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

export function workPhaseLabel(phase: SessionInfo["workPhase"]): string {
  switch (phase) {
    case "ideation":       return "Ideation";
    case "implementation": return "Implementation";
    case "review":         return "Review";
    case "debugging":      return "Debugging";
    case "unknown":
    case undefined:
      return "Unknown";
  }
}

export function lifecyclePhaseLabel(phase: SessionInfo["lifecyclePhase"]): string {
  switch (phase) {
    case "creating": return "Creating";
    case "active":   return "Active";
    case "ending":   return "Ending";
    case "ended":    return "Ended";
    case undefined:  return "Unknown";
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
  if (diffSec < 10)    return "just now";
  if (diffSec < 60)    return "<1m ago";
  if (diffSec < 3600)  return `${Math.round(diffSec / 60)}m ago`;
  if (diffSec < 86400) return `${Math.round(diffSec / 3600)}h ago`;
  return `${Math.round(diffSec / 86400)}d ago`;
}

/** Next moment relativeTime()'s output for this timestamp can change, matching its bucket edges. */
export function nextRelativeTimeRefreshMs(
  iso: string | undefined,
  now: Date = new Date(),
): number | null {
  if (!iso) return null;
  const t = new Date(iso).getTime();
  if (Number.isNaN(t)) return null;

  const nowMs = now.getTime();
  const elapsedSeconds = Math.max(0, Math.round((nowMs - t) / SECOND_MS));

  let nextDisplaySecond: number;
  if (elapsedSeconds < 10) {
    nextDisplaySecond = 10;
  } else if (elapsedSeconds < 60) {
    nextDisplaySecond = 60;
  } else if (elapsedSeconds < 3600) {
    nextDisplaySecond = Math.min(Math.round(elapsedSeconds / 60) * 60 + 30, 3600);
  } else if (elapsedSeconds < 86400) {
    nextDisplaySecond = Math.min(Math.round(elapsedSeconds / 3600) * 3600 + 1800, 86400);
  } else {
    nextDisplaySecond = Math.round(elapsedSeconds / 86400) * 86400 + 43200;
  }

  return delayUntil(t + (nextDisplaySecond - 0.5) * SECOND_MS, nowMs);
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
  if (session.memoryState === "resumable" || session.memoryState === "remembered" || session.memoryState === "unsupported") {
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
