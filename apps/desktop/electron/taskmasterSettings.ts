function validateEvidenceAdmission(value: Record<string, unknown>): void {
  const versions = value.versions as Record<string, unknown> | undefined;
  if (versions?.collector !== 4) return;
  const fail = (): never => { throw Error('Invalid readiness evidence admission'); };
  const manifest = value.inputManifest as {collector?: Record<string, unknown>} | undefined;
  const collector = manifest?.collector;
  if (!collector || collector.collectorVersion !== 4 || collector.purpose !== 'readiness') fail();
  const ids = (value: unknown): number[] => {
    if (!Array.isArray(value) || value.length > 8 || value.some(id=>!Number.isInteger(id)||id<0||id>=512) || new Set(value).size!==value.length) fail();
    return value as number[];
  };
  const preferred = ids(collector!.preferredFileIds ?? []);
  const admitted = ids(collector!.requestedFileIds);
  const omissions = collector!.selectionOmissions ?? [];
  if (!Array.isArray(omissions) || omissions.length > 8) fail();
  const omitted: number[] = [];
  for (const item of omissions as Record<string, unknown>[]) {
    if (!item || typeof item!=='object' || Object.keys(item).sort().join(',')!=='id,reason' || item.reason!=='evidence_limit' || !preferred.includes(item.id as number)) fail();
    omitted.push(item.id as number);
  }
  if (new Set(omitted).size!==omitted.length || JSON.stringify(preferred.filter(id=>!omitted.includes(id)))!==JSON.stringify(admitted)) fail();
}

// Defined independently of renderer types to preserve the process boundary.
export type TaskmasterAssessmentPhase = "discovering" | "reading_evidence" | "assessing";
const assessmentPhases: readonly TaskmasterAssessmentPhase[] = ["discovering", "reading_evidence", "assessing"];

async function boundedResponseText(response: Response, limit: number, assertCurrent: () => void): Promise<string> {
  if (!response.body) return "";
  const reader = response.body.getReader();
  const chunks: Uint8Array[] = [];
  let size = 0;
  let complete = false;
  try {
    while (true) {
      assertCurrent();
      const { done, value } = await reader.read();
      assertCurrent();
      if (done) { complete = true; break; }
      if (value.byteLength > limit - size) throw new Error("Taskmaster response too large");
      size += value.byteLength;
      chunks.push(value);
    }
    return Buffer.concat(chunks, size).toString("utf8");
  } finally {
    if (!complete) await reader.cancel().catch(() => {});
    reader.releaseLock();
  }
}

/** Narrow privileged transport: renderer input never selects a URL or authority. */
export async function taskmasterRequest(
  port: number, token: string, resource: "settings" | "knowledge" | "inference" | "models" | "run-status" | "assess-workflow" | "readiness" | "assess-readiness" | "assess-and-suggest" | "improvement" | "start-improvement" | "cancel-improvement", payload?: unknown,
  fetcher: (url: string, init?: RequestInit) => Promise<Response> = fetch,
  assertCurrent: () => void = () => {},
  requestSignal?: AbortSignal,
): Promise<Record<string, unknown>> {
  assertCurrent();
  if (!Number.isInteger(port) || port < 1 || port > 65535 || !token) throw new Error("Taskmaster backend unavailable");
  if ((resource === "assess-workflow" || resource === "assess-readiness" || resource === "assess-and-suggest" || resource === "readiness" || resource === "improvement" || resource === "start-improvement") && payload !== undefined) throw new Error("Workflow assessment does not accept a request body");
  const body = payload === undefined ? undefined : JSON.stringify(payload);
  const limit = resource === "knowledge" ? 6 * 1024 * 1024 : resource === "inference" ? 8 * 1024 : 64 * 1024;
  if (body && Buffer.byteLength(body) > limit) throw new Error("Taskmaster request too large");
  const path = resource === "settings" ? "/settings/taskmaster"
    : resource === "run-status" ? "/taskmaster/run-status"
      : resource === "assess-workflow" ? "/taskmaster/assess-workflow"
      : resource === "readiness" ? "/taskmaster/readiness"
      : resource === "assess-and-suggest" ? "/taskmaster/assess-and-suggest"
      : resource === "assess-readiness" ? "/taskmaster/assess-readiness"
      : resource === "improvement" ? "/taskmaster/improvement"
      : resource === "start-improvement" ? "/taskmaster/improvement"
      : resource === "cancel-improvement" ? "/taskmaster/improvement/cancel"
      : `/settings/taskmaster/${resource}`;
  const response = await fetcher(`http://127.0.0.1:${port}${path}`, {
    method: resource === "assess-workflow" || resource === "assess-readiness" || resource === "assess-and-suggest" || resource === "start-improvement" || resource === "cancel-improvement" || body !== undefined ? "POST" : "GET",
    headers: { "Content-Type": "application/json", "x-orkworks-open-plan-token": token },
    body, signal: requestSignal ? AbortSignal.any([requestSignal, AbortSignal.timeout(resource === "models" ? 45_000 : 15_000)]) : AbortSignal.timeout(resource === "models" ? 45_000 : 15_000),
  });
  assertCurrent();
  const responseLimit = resource === "readiness" ? 8 * 1024 * 1024 + 128 * 1024
    : resource === "knowledge" ? 2 * 1024 * 1024
    : resource === "models" ? 256 * 1024
      : resource === "run-status" || resource === "improvement" ? 256 * 1024 : 64 * 1024;
  const responseText = await boundedResponseText(response, responseLimit, assertCurrent);
  assertCurrent();
  let result: unknown;
  try { result = JSON.parse(responseText); }
  catch { throw new Error("Invalid Taskmaster response"); }
  if (!result || typeof result !== "object" || Array.isArray(result)) throw new Error("Invalid Taskmaster response");
  const value = result as Record<string, unknown>;
  const error = value.error;
  const message = typeof error === "string" ? error
    : error && typeof error === "object" && !Array.isArray(error) && typeof (error as Record<string, unknown>).message === "string"
      ? (error as Record<string, string>).message
      : `Taskmaster request failed (${response.status})`;
  if (!response.ok) throw new Error(message);
  if (resource === "run-status" && value.assessment && typeof value.assessment === "object") {
    const phase = (value.assessment as Record<string, unknown>).phase;
    if (phase !== undefined && phase !== null
      && (typeof phase !== "string" || !assessmentPhases.includes(phase as TaskmasterAssessmentPhase))) {
      throw new Error("Invalid Taskmaster assessment phase");
    }
  }
  if (resource === "readiness" && Array.isArray(value.snapshots)) {
    for (const snapshot of value.snapshots) {
      if (snapshot && typeof snapshot === "object" && snapshot.data && typeof snapshot.data === "object") validateEvidenceAdmission(snapshot.data);
    }
  }
  return value;
}


// Keep identity and presentation separate; manual IDs never pass through this parser.
export function parseTaskmasterModels(value: Record<string, unknown>): { id: string; displayName: string }[] {
  if (value.modelOptions !== undefined) {
    if (!Array.isArray(value.modelOptions) || value.modelOptions.some((model) =>
      !model || typeof model !== "object" || typeof model.id !== "string" || typeof model.displayName !== "string")) {
      throw new Error("Invalid Taskmaster model response");
    }
    return value.modelOptions.map((model) => ({ id: model.id as string, displayName: model.displayName as string }));
  }
  // Older sidecars expose only IDs.
  if (!Array.isArray(value.models) || value.models.some((model) => typeof model !== "string")) {
    throw new Error("Invalid Taskmaster model response");
  }
  return value.models.map((id: string) => ({ id, displayName: id }));
}
