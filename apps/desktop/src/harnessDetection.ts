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
export async function getHarnessDetectionStatus(harnessId: string, integrationKey?: IntegrationKey): Promise<IntegrationStatusResult>;
export async function getHarnessDetectionStatus(
  harnessOrId: HarnessConfig | string,
  integrationKeyOverride?: IntegrationKey,
): Promise<IntegrationStatusResult> {
  const harnessId = typeof harnessOrId === "string" ? harnessOrId : harnessOrId.id;
  const key = typeof harnessOrId === "string"
    ? integrationKeyOverride
    : integrationKeyOverride ?? integrationKeyForHarness(harnessOrId);
  if (!key) return window.orkworks.getHarnessIntegrationStatus(harnessId);

  const result = await window.orkworks.getGroupedHarnessIntegrationStatus(key.adapterId, key.targetId);
  return result.ok ? { ok: true, status: result.group.status } : result;
}
