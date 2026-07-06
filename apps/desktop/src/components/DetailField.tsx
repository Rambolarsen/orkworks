import type { ReactNode } from "react";

interface DetailFieldProps {
  label: string;
  children: ReactNode;
  /** Extra class on the wrapper, e.g. "detail-fact" inside the facts grid. */
  className?: string;
  /** Tooltip for the value, e.g. the full path behind a shortened directory. */
  valueTitle?: string;
}

/**
 * A labelled field in the session Detail panel: a tiny uppercase eyebrow
 * label over a value. The detail panel's facts are a stack of these.
 */
function DetailField({ label, children, className, valueTitle }: DetailFieldProps) {
  return (
    <div className={className}>
      <div className="session-detail-label">{label}</div>
      <div className="session-detail-value" title={valueTitle}>{children}</div>
    </div>
  );
}

export default DetailField;
