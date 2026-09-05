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
  /**
   * Id of a status readout rendered elsewhere in the DOM (e.g. a collapsed
   * disclosure's expanded subsection) that describes this switch. Ignored
   * when `statusDescription` is given — that case renders its own readout
   * and points `aria-describedby` at it instead.
   */
  describedById?: string;
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

/** Status readout used both inside Toggle and by callers that render it in a separately-disclosed subsection. */
export function ToggleStatusText({
  id,
  description,
  glyph,
}: {
  id?: string;
  description: string;
  glyph?: ToggleStatusGlyph;
}) {
  return (
    <span
      id={id}
      className={`ui-toggle-status${glyph ? ` ui-toggle-status--${glyph}` : ""}`}
    >
      <span
        className={`ui-toggle-status-glyph${glyph === "spinner" ? " ui-toggle-status-glyph--spinner" : ""}`}
        aria-hidden="true"
      >
        {glyphText(glyph)}
      </span>
      <span className="ui-toggle-status-text">{description}</span>
    </span>
  );
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
  describedById,
}: ToggleProps) {
  const statusId = useId();
  const hasStatus = Boolean(statusDescription);
  const button = (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={ariaLabel ?? label ?? undefined}
      aria-describedby={hasStatus ? statusId : describedById}
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
      {hasStatus ? <ToggleStatusText id={statusId} description={statusDescription!} glyph={statusGlyph} /> : null}
    </span>
  );
}
