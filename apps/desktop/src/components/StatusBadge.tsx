import type { ReactNode } from "react";

export type StatusBadgeTone = "ok" | "warn" | "error" | "info" | "neutral";

interface StatusBadgeProps {
  tone: StatusBadgeTone;
  children: ReactNode;
}

/** Pill status label — used for coding-tool connection state and provider capacity state. */
export default function StatusBadge({ tone, children }: StatusBadgeProps) {
  return <span className={`ui-status-badge ui-status-badge--${tone}`}>{children}</span>;
}
