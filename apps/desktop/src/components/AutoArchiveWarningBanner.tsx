import { useCallback } from "react";
import { Clock } from "lucide-react";
import {
  formatAutoArchiveRemaining,
  nextAutoArchiveCountdownRefreshMs,
} from "../autoArchiveWarning";
import { useStableRelativeTimeNow } from "../useStableRelativeTimeNow";

interface AutoArchiveWarningBannerProps {
  scheduledAt: string;
  disabled: boolean;
  variant?: "detail" | "terminal";
  onKeep: () => void;
  onArchiveNow: () => void;
}

/**
 * Non-blocking idle-Active auto-archive warning banner. Mounted only while a
 * warning is active (keyed by `scheduledAt`) so its relative-time clock always
 * starts fresh at warning appearance or deadline renewal instead of reusing a
 * stale mount-time timestamp from an always-mounted shell.
 */
export default function AutoArchiveWarningBanner({
  scheduledAt,
  disabled,
  variant = "detail",
  onKeep,
  onArchiveNow,
}: AutoArchiveWarningBannerProps) {
  const now = useStableRelativeTimeNow(
    useCallback(
      (currentNow: Date) => nextAutoArchiveCountdownRefreshMs(scheduledAt, currentNow),
      [scheduledAt],
    ),
  );
  const countdown = formatAutoArchiveRemaining(scheduledAt, now);
  const isTerminal = variant === "terminal";

  return (
    <div
      className={isTerminal ? "auto-archive-warning-banner auto-archive-warning-banner--terminal" : "auto-archive-warning-banner"}
      role="status"
      {...(isTerminal ? { "data-terminal-auto-archive-warning": "" } : { "data-auto-archive-warning": "" })}
    >
      <div className="auto-archive-warning-copy">
        <Clock size={14} aria-hidden="true" />
        <span>Idle — closes and archives in {countdown}</span>
      </div>
      <div className="auto-archive-warning-actions">
        <button
          type="button"
          data-auto-archive-keep
          disabled={disabled}
          onClick={onKeep}
        >
          Favorite (keep)
        </button>
        <button
          type="button"
          data-auto-archive-now
          disabled={disabled}
          onClick={onArchiveNow}
        >
          Archive now
        </button>
      </div>
    </div>
  );
}
