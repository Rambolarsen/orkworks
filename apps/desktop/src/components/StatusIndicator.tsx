import { Check, Circle, CircleX, LoaderCircle, TriangleAlert } from "lucide-react";
import type { ComponentType } from "react";
import type { AttentionTone } from "../labels";

interface StatusIndicatorProps {
  tone: AttentionTone;
  label: string;
  variant?: "status" | "unread";
}

/** Shapes carry the same meaning as the tone color, so status stays legible without it. */
const TONE_ICON: Partial<Record<AttentionTone, ComponentType<{ size?: number; className?: string }>>> = {
  working: LoaderCircle,
  blocked: TriangleAlert,
  done: Check,
  failed: CircleX,
  idle: Circle,
};

function StatusIndicator({ tone, label, variant = "status" }: StatusIndicatorProps) {
  if (tone === "neutral") return null; // no signal to show — matches the design contract
  if (variant === "unread" && tone !== "working") {
    return (
      <span
        className="status-indicator status-indicator-unread"
        data-attention={tone}
        role="img"
        aria-label={`Unread: ${label}`}
      />
    );
  }
  const Icon = TONE_ICON[tone];
  if (!Icon) {
    return (
      <span
        className="status-indicator status-indicator-dot"
        data-attention={tone}
        role="img"
        aria-label={label}
      />
    );
  }
  return (
    <span className="status-indicator" data-attention={tone} role="img" aria-label={label}>
      <Icon size={13} className={tone === "working" ? "status-indicator-spin" : undefined} />
    </span>
  );
}

export default StatusIndicator;
