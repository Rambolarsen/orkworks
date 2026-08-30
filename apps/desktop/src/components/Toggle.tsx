import { useId } from "react";

export type ToggleVisualState =
  | "off"
  | "neutral"
  | "healthy"
  | "needs-you"
  | "error"
  | "in-progress";

export type ToggleStatusGlyph =
  | "neutral"
  | "healthy"
  | "warning"
  | "trust"
  | "offline"
  | "spinner";

interface ToggleProps {
  checked: boolean;
  onChange: () => void;
  label?: string;
  /** Accessible name when the visible label already lives elsewhere (e.g. a sibling row). */
  ariaLabel?: string;
  disabled?: boolean;
  visualState?: ToggleVisualState;
  statusDescription?: string;
  tooltip?: string;
  statusGlyph?: ToggleStatusGlyph;
}

function glyphText(statusGlyph: ToggleStatusGlyph | undefined): string {
  switch (statusGlyph) {
    case "healthy":
      return "✓";
    case "warning":
      return "!";
    case "trust":
      return "↺";
    case "offline":
      return "×";
    case "spinner":
      return "";
    default:
      return "•";
  }
}

/** Pill switch used throughout Settings in place of native checkboxes. */
export default function Toggle({
  checked,
  onChange,
  label,
  ariaLabel,
  disabled,
  visualState = checked ? "neutral" : "off",
  statusDescription,
  tooltip,
  statusGlyph,
}: ToggleProps) {
  const statusId = useId();
  const hasStatus = Boolean(statusDescription);
  const button = (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={ariaLabel ?? label ?? undefined}
      aria-describedby={hasStatus ? statusId : undefined}
      className={`ui-toggle ui-toggle--${visualState}${checked ? " ui-toggle--on" : ""}`}
      onClick={onChange}
      disabled={disabled}
      title={tooltip ?? undefined}
    >
      <span className="ui-toggle-thumb" />
    </button>
  );

  if (!label && !hasStatus) return button;

  return (
    <span className="ui-toggle-row">
      {button}
      {label ? <span className="ui-toggle-label">{label}</span> : null}
      {hasStatus ? (
        <span
          id={statusId}
          className={`ui-toggle-status${statusGlyph ? ` ui-toggle-status--${statusGlyph}` : ""}`}
        >
          <span
            className={`ui-toggle-status-glyph${statusGlyph === "spinner" ? " ui-toggle-status-glyph--spinner" : ""}`}
            aria-hidden="true"
          >
            {glyphText(statusGlyph)}
          </span>
          <span className="ui-toggle-status-text">{statusDescription}</span>
        </span>
      ) : null}
    </span>
  );
}
