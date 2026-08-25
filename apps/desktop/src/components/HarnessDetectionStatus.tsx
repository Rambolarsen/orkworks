import { useEffect, useState } from "react";
import type { IntegrationStatusResult } from "../harnessTypes";

interface HarnessDetectionStatusProps {
  harnessId: string;
}

type DetectionState = "loading" | "detected" | "not-detected" | "unknown";

/**
 * Always-on "Detected"/"Not detected" indicator for a coding-tool row.
 * Independent of whether the tool is enabled — HarnessIntegrationSection
 * only mounts (and re-fetches the same status) once a tool is toggled on,
 * but the row header shows this regardless so the list is scannable at a
 * glance, matching the design handoff.
 */
export default function HarnessDetectionStatus({ harnessId }: HarnessDetectionStatusProps) {
  const [result, setResult] = useState<IntegrationStatusResult | null>(null);

  useEffect(() => {
    let cancelled = false;
    setResult(null);
    window.orkworks.getHarnessIntegrationStatus(harnessId).then((r) => {
      if (!cancelled) setResult(r);
    });
    return () => {
      cancelled = true;
    };
  }, [harnessId]);

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
    <span className="harness-detection-status">
      <span className={`harness-detection-dot${state === "detected" ? " harness-detection-dot--ok" : ""}`} />
      <span className="harness-detection-text">{text}</span>
    </span>
  );
}
