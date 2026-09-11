/** Privileged custom-inference approval protocol. No renderer-provided URLs or authority. */
export interface InferenceTrustRevision { documentRevision: string; generation: string; digest: string | null }
export interface InferenceTrustRequest { harnessId: string; expectedRevision: InferenceTrustRevision }
export interface InferenceAdapterView {
  id: string; name: string; state: "approved" | "approval_required" | "unavailable";
  definition: { kind: "command"; command: string; args: string[]; input: "stdin" | "file"; output: "result-json-v1"; timeoutSecs: number; reasoningEffortArgs?: string[] };
  resolvedPath: string | null; revision: InferenceTrustRevision;
}
export interface TrustContext {
  port: number; token: string; isCurrent: () => boolean;
  confirm: (detail: string) => Promise<boolean>;
  fetcher?: (url: string, init?: RequestInit) => Promise<Response>;
}
const object = (value: unknown): value is Record<string, unknown> => !!value && typeof value === "object" && !Array.isArray(value);
const digest = (value: unknown): value is string => typeof value === "string" && /^[a-f0-9]{64}$/.test(value);
function requestValue(raw: unknown, approving: boolean): InferenceTrustRequest {
  if (!object(raw) || Object.keys(raw).some((key) => !["harnessId", "expectedRevision"].includes(key))
    || typeof raw.harnessId !== "string" || !/^[a-z0-9-]{1,256}$/.test(raw.harnessId) || !object(raw.expectedRevision)) throw new Error("Invalid inference approval request");
  const rev = raw.expectedRevision;
  if (Object.keys(rev).some((key) => !["documentRevision", "generation", "digest"].includes(key))
    || !digest(rev.documentRevision) || typeof rev.generation !== "string" || !/^(0|[1-9][0-9]{0,19})$/.test(rev.generation)
    || BigInt(rev.generation) > 18446744073709551615n || !(digest(rev.digest) || (!approving && rev.digest === null))) throw new Error("Invalid inference approval revision");
  return { harnessId: raw.harnessId, expectedRevision: { documentRevision: rev.documentRevision, generation: rev.generation, digest: rev.digest as string | null } };
}
function current(context: TrustContext) {
  if (!context.isCurrent()) throw new Error("Sidecar changed. Refresh and review the adapter again.");
  if (!Number.isInteger(context.port) || context.port < 1 || context.port > 65535 || !context.token) throw new Error("Inference approval backend unavailable");
}
async function transport(context: TrustContext, payload?: unknown): Promise<Record<string, unknown>> {
  current(context);
  const response = await (context.fetcher ?? fetch)(`http://127.0.0.1:${context.port}/settings/taskmaster/inference`, {
    method: payload === undefined ? "GET" : "POST", headers: { "Content-Type": "application/json", "x-orkworks-open-plan-token": context.token },
    body: payload === undefined ? undefined : JSON.stringify(payload), signal: AbortSignal.timeout(15_000),
  });
  current(context);
  const result: unknown = await response.json();
  current(context);
  if (!response.ok) throw new Error(object(result) && typeof result.error === "string" ? result.error : "Inference approval request failed");
  if (!object(result)) throw new Error("Invalid inference approval response");
  return result;
}
export async function readInferenceTrust(context: TrustContext): Promise<InferenceAdapterView[]> {
  const result = await transport(context);
  if (!Array.isArray(result.adapters)) throw new Error("Invalid inference approval response");
  return result.adapters.map((raw: unknown) => {
    if (!object(raw) || typeof raw.name !== "string" || !["approved", "approval_required", "unavailable"].includes(String(raw.state))
      || !(typeof raw.resolvedPath === "string" || raw.resolvedPath === null) || !object(raw.definition)) throw new Error("Invalid inference adapter response");
    requestValue({ harnessId: raw.id, expectedRevision: raw.revision }, raw.state !== "unavailable");
    const def = raw.definition;
    if (def.kind !== "command" || typeof def.command !== "string" || !Array.isArray(def.args) || !def.args.every((arg) => typeof arg === "string")
      || !["stdin", "file"].includes(String(def.input)) || def.output !== "result-json-v1" || !Number.isInteger(def.timeoutSecs) || Number(def.timeoutSecs) < 1 || Number(def.timeoutSecs) > 120
      || (def.reasoningEffortArgs !== undefined && (!Array.isArray(def.reasoningEffortArgs) || !def.reasoningEffortArgs.every((arg) => typeof arg === "string")))) throw new Error("Invalid inference adapter definition");
    if (raw.state !== "unavailable" && !raw.resolvedPath) throw new Error("Invalid inference executable path");
    return raw as unknown as InferenceAdapterView;
  });
}
export async function approveInferenceAdapter(raw: unknown, context: TrustContext): Promise<boolean> {
  const request = requestValue(raw, true);
  const adapter = (await readInferenceTrust(context)).find((item) => item.id === request.harnessId);
  const rev = request.expectedRevision;
  if (!adapter || adapter.state === "unavailable" || adapter.revision.documentRevision !== rev.documentRevision
    || adapter.revision.generation !== rev.generation || adapter.revision.digest !== rev.digest) throw new Error("Adapter changed. Refresh and review it again.");
  const def = adapter.definition;
  const detail = [
    `Coding tool: ${adapter.name}`, `Executable: ${def.command}`, `Resolved path: ${adapter.resolvedPath}`,
    `Arguments: ${JSON.stringify(def.args)}`, `Reasoning arguments: ${JSON.stringify(def.reasoningEffortArgs ?? [])}`,
    `Input: ${def.input}; timeout: ${def.timeoutSecs} seconds`, "",
    "This executable receives permitted workspace context and access to your existing CLI credentials. It may run configured hooks or plugins. It is not sandboxed or certified safe by OrkWorks. Administrator policy remains authoritative.",
    "Approval includes updates to the installed executable at this path, its dependencies and configuration. Revocation prevents new calls and discards pending results; a running process may continue until exit or timeout, and side effects cannot be undone.",
    "Custom execution is not enabled in this version. This records executable trust only.",
  ].join("\n");
  if (!await context.confirm(detail)) return false;
  current(context);
  const result = await transport(context, { ...request, action: "approve" });
  if (result.ok !== true) throw new Error("Inference approval was not saved");
  return true;
}
export async function revokeInferenceAdapter(raw: unknown, context: TrustContext): Promise<void> {
  const request = requestValue(raw, false);
  const result = await transport(context, { ...request, action: "revoke" });
  if (result.ok !== true) throw new Error("Inference revocation was not saved");
}
