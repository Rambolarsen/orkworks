import type { HarnessConfig, IntegrationStatusResult } from "./harnessTypes.ts";
import type { IntegrationKey } from "./harnessIntegrationPresentation.ts";

export function integrationKeyForHarness(harness: HarnessConfig): IntegrationKey | null {
  if (!harness.integration || typeof harness.integration !== "object") return null;
  const kind = (harness.integration as { kind?: unknown }).kind;
  return typeof kind === "string" && kind.length > 0
    ? { adapterId: kind, targetId: "workspace" }
    : null;
}

export function isHarnessDetected(result: IntegrationStatusResult | undefined): boolean {
  return result?.ok === true && result.status.toolDetected;
}

export async function getHarnessDetectionStatus(harness: HarnessConfig): Promise<IntegrationStatusResult>;
export async function getHarnessDetectionStatus(harnessId: string): Promise<IntegrationStatusResult>;
export async function getHarnessDetectionStatus(
  harnessOrId: HarnessConfig | string,
): Promise<IntegrationStatusResult> {
  const harnessId = typeof harnessOrId === "string" ? harnessOrId : harnessOrId.id;
  return window.orkworks.getHarnessIntegrationStatus(harnessId);
}
