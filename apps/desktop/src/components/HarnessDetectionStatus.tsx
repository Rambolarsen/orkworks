import { useEffect, useState } from "react";
import type { IntegrationStatusResult } from "../harnessTypes";

interface HarnessDetectionStatusProps {
  harnessId: string;
}

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

  const detected = result?.ok ? result.status.toolDetected : null;

  return (
    <span className="harness-detection-status">
      <span className={`harness-detection-dot${detected ? " harness-detection-dot--ok" : ""}`} />
      <span className="harness-detection-text">
        {detected === null ? "Checking…" : detected ? "Detected" : "Not detected"}
      </span>
    </span>
  );
}
