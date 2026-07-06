import type { ReactNode } from "react";

interface SourceBadgeProps {
  /** Raw metadata source: "agent" (lime) or "peon" (info-blue); anything else renders neutral. */
  source: string;
  children: ReactNode;
}

/**
 * Tiny tag marking who reported a session's metadata. `source` takes the raw
 * backend value (it only keys the color); the visible content in `children`
 * must be already-humanized (e.g. "Agent · 95% confidence" via
 * sourceWithConfidence) — never a bare backend enum or naked percentage.
 */
function SourceBadge({ source, children }: SourceBadgeProps) {
  return (
    <span className="source-badge" data-source={source.toLowerCase()}>
      {children}
    </span>
  );
}

export default SourceBadge;
