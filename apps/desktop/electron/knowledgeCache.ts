import { createHash, createPublicKey, randomBytes } from "node:crypto";
import { execFileSync } from "node:child_process";
import { closeSync, openSync } from "node:fs";
import { open, mkdir, rename, rm, stat } from "node:fs/promises";
import { join } from "node:path";
import fsExt from "fs-ext";
import { verifyKnowledgeActivation } from "./knowledgeProof.ts";
import type { KnowledgeIdentity, VerifiedKnowledge } from "./knowledgeProof.ts";

const MAX_BYTES = 6 * 1024 * 1024;
export function knowledgeKeyDirectory(directory: string, publicKey: string): string {
  const key = createPublicKey(publicKey);
  if (key.asymmetricKeyType !== "ed25519") throw new Error("Knowledge key must be Ed25519");
  const fingerprint = createHash("sha256").update(key.export({ type: "spki", format: "der" })).digest("hex");
  return join(directory, "keys", fingerprint);
}
export function sameKnowledgeIdentity(a: KnowledgeIdentity, b: KnowledgeIdentity): boolean {
  return a.bundleSha256 === b.bundleSha256 && a.formatVersion === b.formatVersion
    && a.privacyPolicyVersion === b.privacyPolicyVersion && a.version === b.version
    && a.sequence === b.sequence && a.publicKeySha256 === b.publicKeySha256;
}
export async function readKnowledgeFile(path: string): Promise<Buffer> {
  const file = await open(path, "r");
  try {
    const metadata = await file.stat();
    if (!metadata.isFile() || metadata.size > MAX_BYTES) throw new Error("Knowledge record exceeds 6 MiB or is not a regular file");
    const bytes = Buffer.alloc(Math.min(metadata.size + 1, MAX_BYTES + 1));
    let length = 0;
    while (length < bytes.length) {
      const result = await file.read(bytes, length, bytes.length - length, null);
      if (!result.bytesRead) break;
      length += result.bytesRead;
    }
    if (length > metadata.size || length > MAX_BYTES) throw new Error("Knowledge record changed or exceeds bounds");
    return bytes.subarray(0, length);
  } finally { await file.close(); }
}
/** Unique flushed files; retain the destination until the native atomic replace succeeds. */
export async function writeKnowledgeFile(path: string, content: string, signal?: AbortSignal): Promise<void> {
  const temporary = `${path}.${process.pid}.${randomBytes(16).toString("hex")}.tmp`;
  try {
    const file = await open(temporary, "wx", 0o600);
    try { await file.writeFile(content); await file.sync(); } finally { await file.close(); }
    const exists = await stat(path).then(() => true, (error: NodeJS.ErrnoException) => {
      if (error.code !== "ENOENT") throw error;
      return false;
    });
    // Cancellation cutoff: no await between this check and dispatching native replacement.
    // Once dispatched, atomic publication is allowed to finish and must not be rolled back.
    signal?.throwIfAborted();
    if (process.platform === "win32" && exists) {
      execFileSync("powershell.exe", ["-NoLogo", "-NoProfile", "-NonInteractive", "-Command",
        "$ErrorActionPreference = 'Stop'; [System.IO.File]::Replace($env:ORKWORKS_KNOWLEDGE_TEMPORARY, $env:ORKWORKS_KNOWLEDGE_TARGET, $null, $true)"],
      { env: { ...process.env, ORKWORKS_KNOWLEDGE_TEMPORARY: temporary, ORKWORKS_KNOWLEDGE_TARGET: path }, stdio: "ignore", timeout: 5_000 });
    } else await rename(temporary, path);
  } finally { await rm(temporary, { force: true }); }
}

export class KnowledgeCache {
  readonly directory: string;
  private options: { directory: string; publicKey: string; starterPath: string };
  constructor(options: { directory: string; publicKey: string; starterPath: string }) {
    this.options = options;
    this.directory = knowledgeKeyDirectory(options.directory, options.publicKey);
  }
  private async read(path: string): Promise<VerifiedKnowledge | null> {
    try { return verifyKnowledgeActivation(await readKnowledgeFile(path), this.options.publicKey); }
    catch { return null; }
  }
  async load(): Promise<VerifiedKnowledge | null> {
    const records = await Promise.all([this.read(join(this.directory, "active.json")),
      this.read(join(this.directory, "previous.json")), this.read(this.options.starterPath)]);
    let selected: VerifiedKnowledge | null = null;
    for (const candidate of records) {
      if (candidate && (!selected || candidate.identity.sequence > selected.identity.sequence)) selected = candidate;
    }
    return selected;
  }
  private async lock(signal?: AbortSignal): Promise<number> {
    await mkdir(this.directory, { recursive: true });
    // Never unlink this inode: flock releases ownership on close or process exit.
    const descriptor = openSync(join(this.directory, ".knowledge.lock"), "a+", 0o600);
    for (let attempt = 0; attempt < 50; attempt++) {
      try { signal?.throwIfAborted(); fsExt.flockSync(descriptor, "exnb"); return descriptor; }
      catch (error) {
        if (!["EAGAIN", "EWOULDBLOCK", "EINTR"].includes((error as NodeJS.ErrnoException).code ?? "")) {
          closeSync(descriptor); throw error;
        }
        await new Promise(resolve => setTimeout(resolve, 10));
      }
    }
    closeSync(descriptor);
    throw new Error("Knowledge cache lock timed out");
  }
  async commit(candidate: VerifiedKnowledge, signal?: AbortSignal): Promise<VerifiedKnowledge> {
    // Public structural types are not a trust boundary; rederive identity from proof.
    const verified = verifyKnowledgeActivation(Buffer.from(JSON.stringify(candidate.activation)), this.options.publicKey);
    const descriptor = await this.lock(signal);
    try {
      signal?.throwIfAborted();
      const current = await this.load();
      signal?.throwIfAborted();
      if (current) {
        if (current.identity.sequence > verified.identity.sequence) return current;
        if (current.identity.sequence === verified.identity.sequence) {
          if (!sameKnowledgeIdentity(current.identity, verified.identity)) throw new Error("Knowledge sequence equivocation");
          return current;
        }
        await writeKnowledgeFile(join(this.directory, "previous.json"), JSON.stringify(current.activation), signal);
      }
      try {
        await writeKnowledgeFile(join(this.directory, "active.json"), JSON.stringify(verified.activation), signal);
      } catch (error) {
        const observed = await this.read(join(this.directory, "active.json"));
        if (observed && sameKnowledgeIdentity(observed.identity, verified.identity)) return observed;
        throw error;
      }
      const observed = await this.read(join(this.directory, "active.json"));
      if (!observed || !sameKnowledgeIdentity(observed.identity, verified.identity)) throw new Error("Knowledge cache replacement could not be confirmed");
      return observed;
    } finally { closeSync(descriptor); }
  }
}
