import { useEffect, useState } from "react";
import type { IntegrationStatusResult } from "../harnessTypes";
import { getHarnessDetectionStatus } from "../harnessDetection";

interface HarnessDetectionStatusProps {
  harnessId: string;
  onResult?: (harnessId: string, result: IntegrationStatusResult | null) => void;
  refreshGeneration?: number;
}

type DetectionState = "loading" | "detected" | "not-detected" | "unknown";

/**
 * Always-on "Detected"/"Not detected" indicator for a coding-tool row.
 * Independent of whether the tool is enabled. The row header shows this
 * regardless so the list is scannable at a glance, matching the design handoff.
 */
export default function HarnessDetectionStatus({ harnessId, onResult, refreshGeneration = 0 }: HarnessDetectionStatusProps) {
  const [result, setResult] = useState<IntegrationStatusResult | null>(null);

  useEffect(() => {
    let cancelled = false;
    setResult(null);
    onResult?.(harnessId, null);
    const request = getHarnessDetectionStatus(harnessId);
    request.then((r) => {
      if (!cancelled) {
        setResult(r);
        onResult?.(harnessId, r);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [harnessId, onResult, refreshGeneration]);

  const state: DetectionState =
    result === null
      ? "loading"
      : !result.ok
        ? "unknown"
        : result.status.toolDetected
          ? "detected"
          : "not-detected";

  const text =
    state === "loading" ? "Checking…" : state === "unknown" ? "Unknown" : state === "detected" ? "Detected" : "Not detected";

  return (
    <span
      className="harness-detection-status"
      role="status"
      aria-live="polite"
      aria-label={`Coding tool detection status: ${text}`}
    >
      <span className={`harness-detection-dot${state === "detected" ? " harness-detection-dot--ok" : ""}`} />
      <span className="harness-detection-text">{text}</span>
    </span>
  );
}
