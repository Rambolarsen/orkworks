import { sidecarSend, errorFromBody, type SidecarFetchImpl } from "./sidecarClient.ts";
export interface ImageCreation {
  harnessId: string; model?: string; reasoningEffort?: string; initialPrompt?: string; expectedWorkspacePath: string;
  initialImages: Array<{name: string; mimeType: string; dataBase64: string}>;
}
export function normalizeImageCreation(value: unknown): ImageCreation {
  if (!value || typeof value !== "object") throw new Error("Invalid image session request.");
  const input = value as Record<string,unknown>;
  if (typeof input.harnessId !== "string" || !input.harnessId || input.harnessId.length > 256 || typeof input.imageWorkspacePath !== "string" || !input.imageWorkspacePath || Buffer.byteLength(input.imageWorkspacePath) > 32768) throw new Error("Open a workspace and select a coding tool before attaching images.");
  if (input.model !== undefined && (typeof input.model !== "string" || input.model.length > 1024)) throw new Error("Invalid model.");
  if (input.reasoningEffort !== undefined && (typeof input.reasoningEffort !== "string" || input.reasoningEffort.length > 64)) throw new Error("Invalid reasoning effort.");
  if (input.initialPrompt !== undefined && (typeof input.initialPrompt !== "string" || Buffer.byteLength(input.initialPrompt) > 65536)) throw new Error("Image session prompts allow at most 64 KiB.");
  if (!Array.isArray(input.initialImages) || input.initialImages.length < 1 || input.initialImages.length > 8) throw new Error("Attach between one and eight images.");
  let total = 0;
  const initialImages = input.initialImages.map((entry: unknown) => {
    if (!entry || typeof entry !== "object") throw new Error("Invalid image.");
    const {name,mimeType,dataBase64} = entry as Record<string,unknown>;
    if (typeof name !== "string" || Buffer.byteLength(name) > 255 || typeof dataBase64 !== "string" || dataBase64.length > 6990508 || (dataBase64.length % 4 !== 0 || !/^[A-Za-z0-9+/]*={0,2}$/.test(dataBase64))) throw new Error("Invalid image name or encoding.");
    if (mimeType !== "image/png" && mimeType !== "image/jpeg" && mimeType !== "image/webp") throw new Error("Use PNG, JPEG, or WebP images.");
    const bytes = Buffer.from(dataBase64, "base64"); total += bytes.length;
    if (!bytes.length || bytes.length > 5 * 1024 * 1024 || total > 20 * 1024 * 1024) throw new Error("Images allow 5 MiB each and 20 MiB total.");
    const matches = mimeType === "image/png" ? bytes.subarray(0,8).equals(Buffer.from([137,80,78,71,13,10,26,10])) : mimeType === "image/jpeg" ? bytes[0] === 255 && bytes[1] === 216 && bytes[2] === 255 : bytes.toString("ascii",0,4) === "RIFF" && bytes.toString("ascii",8,12) === "WEBP";
    if (!matches) throw new Error("Image type does not match its contents.");
    return {name,mimeType,dataBase64};
  });
  return {harnessId:input.harnessId, reasoningEffort:input.reasoningEffort as string | undefined, model:input.model as string | undefined, initialPrompt:input.initialPrompt as string | undefined, expectedWorkspacePath:input.imageWorkspacePath, initialImages};
}
export async function submitImageSession(port: number, token: string, request: ImageCreation, signal?: AbortSignal, fetchImpl?: SidecarFetchImpl): Promise<unknown> {
  const response = await sidecarSend(port,"session-image-context",{signal,timeoutMs:10000,fetchImpl});
  if (!response.ok) throw new Error("This sidecar does not support image context. Restart or update OrkWorks.");
  const capability = await response.json() as {version?:unknown; supportedHarnesses?:unknown};
  if (capability.version !== 1 || !Array.isArray(capability.supportedHarnesses) || !capability.supportedHarnesses.includes(request.harnessId)) throw new Error("Image context is not verified for this coding tool/version/platform.");
  signal?.throwIfAborted();
  const created = await sidecarSend(port,"sessions",{method:"POST",jsonBody:request,token,signal,fetchImpl});
  if (!created.ok) {
    const fallback = `Could not start the image session (${created.status}).`;
    const text = await created.text().catch(() => "");
    let message = text.trim() || fallback;
    try { message = errorFromBody(JSON.parse(text), fallback); } catch { /* Sidecar BadRequest uses plain text. */ }
    throw new Error(message);
  }
  return created.json();
}
