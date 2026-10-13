import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import { KnowledgeCache, readKnowledgeFile, writeKnowledgeFile, sameKnowledgeIdentity } from "./knowledgeCache.ts";
import { createKnowledgeActivation, selectKnowledgeEntry, verifyKnowledgeActivation, verifyKnowledgeManifest } from "./knowledgeProof.ts";
import type { KnowledgeActivation, VerifiedKnowledge } from "./knowledgeProof.ts";
export interface KnowledgeStatus {
  version: string | null; lastSuccessfulUpdate: string | null; lastError: string | null;
}
/** Main-owned synchronization, guarded at each asynchronous workspace boundary. */
export async function synchronizeKnowledge(
  updates: Pick<KnowledgeUpdates, "load" | "check">,
  activate: (activation: KnowledgeActivation) => Promise<unknown>,
  isCurrent: () => boolean,
): Promise<void> {
  const cached = await updates.load();
  if (!isCurrent()) return;
  if (cached) {
    try { await activate(cached.activation); }
    catch (error) {
      if (!(error instanceof Error) || error.message !== "knowledge bundle sequence is older than the active bundle") throw error;
    }
  }
  if (!isCurrent()) return;
  const updated = await updates.check();
  if (isCurrent() && updated && (!cached || !sameKnowledgeIdentity(updated.identity, cached.identity))) await activate(updated.activation);
}
interface Options {
  directory: string; starterPath: string; publicKey: string; feedUrl: string;
  fetcher?: (url: string, init?: RequestInit) => Promise<Response>;
  now?: () => number;
}
const MAX_BYTES = 2 * 1024 * 1024;
const INTERVAL = 6 * 60 * 60 * 1000;
export class KnowledgeUpdates {
  private options: Options;
  private cache: KnowledgeCache | null = null;
  private current: VerifiedKnowledge | null = null;
  private statusLoaded = false;
  private lastChecked = 0;
  private info: KnowledgeStatus = { version: null, lastSuccessfulUpdate: null, lastError: null };
  private inFlight: Promise<VerifiedKnowledge | null> | null = null;
  private enabled = true;
  private cancellation = new AbortController();
  constructor(options: Options) {
    this.options = options;
    try { this.cache = new KnowledgeCache(options); }
    catch { this.info.lastError = "Packaged knowledge public key is unavailable or invalid"; }
  }
  status(): KnowledgeStatus { return { ...this.info }; }
  setEnabled(enabled: boolean): void {
    this.enabled = enabled;
    if (!enabled) this.cancellation.abort();
    else if (this.cancellation.signal.aborted) this.cancellation = new AbortController();
  }
  private now(): number { return (this.options.now ?? Date.now)(); }
  async load(): Promise<VerifiedKnowledge | null> {
    if (!this.cache) return null;
    this.current = await this.cache.load();
    if (!this.statusLoaded) {
      this.statusLoaded = true;
      try {
        const saved = JSON.parse((await readKnowledgeFile(join(this.cache.directory, "status.json"))).toString("utf8"));
        if (Number.isSafeInteger(saved.lastChecked) && saved.lastChecked >= 0 && saved.lastChecked <= this.now()) this.lastChecked = saved.lastChecked;
        if (typeof saved.lastSuccessfulUpdate === "string") this.info.lastSuccessfulUpdate = saved.lastSuccessfulUpdate;
      } catch { /* Advisory status never establishes proof eligibility. */ }
    }
    this.info.version = this.current?.bundle.version ?? null;
    return this.current;
  }
  check(): Promise<VerifiedKnowledge | null> {
    if (!this.inFlight) this.inFlight = this.checkOnce().finally(() => { this.inFlight = null; });
    return this.inFlight;
  }
  private async download(url: string): Promise<Buffer> {
    if (!this.enabled) throw new Error("Knowledge updates disabled");
    const response = await (this.options.fetcher ?? fetch)(url, { signal: AbortSignal.any([AbortSignal.timeout(15_000), this.cancellation.signal]), redirect: "error" });
    if (!response.ok) throw new Error(`Knowledge update HTTP ${response.status}`);
    if (Number(response.headers.get("content-length")) > MAX_BYTES) throw new Error("Knowledge response too large");
    if (!response.body) throw new Error("Empty knowledge response");
    const reader = response.body.getReader();
    const chunks: Uint8Array[] = [];
    let length = 0;
    try {
      for (;;) {
        const { done, value } = await reader.read();
        if (done) break;
        length += value.byteLength;
        if (length > MAX_BYTES) throw new Error("Knowledge response too large");
        chunks.push(value);
      }
    } finally { await reader.cancel().catch(() => {}); }
    return Buffer.concat(chunks);
  }
  private async checkOnce(): Promise<VerifiedKnowledge | null> {
    await this.load();
    if (!this.cache) return null;
    if (!this.enabled || (this.lastChecked && this.now() - this.lastChecked < INTERVAL)) return this.current;
    this.lastChecked = this.now();
    try {
      const feed = new URL(this.options.feedUrl);
      if (feed.protocol !== "https:" || feed.username || feed.password) throw new Error("Knowledge feed requires HTTPS without credentials");
      const manifestBytes = await this.download(feed.href);
      const next = selectKnowledgeEntry(verifyKnowledgeManifest(manifestBytes, this.options.publicKey));
      const identity = this.current?.identity;
      // Equal sequence is an identity comparison, never a successful newer-only shortcut.
      if (identity && next.sequence === identity.sequence && (next.formatVersion !== identity.formatVersion
        || next.privacyPolicyVersion !== identity.privacyPolicyVersion || next.version !== identity.version || next.sha256 !== identity.bundleSha256)) {
        throw new Error("Knowledge sequence equivocation");
      }
      if (!identity || next.sequence > identity.sequence) {
        const remote = await this.download(new URL(next.path, feed).href);
        const activation = createKnowledgeActivation(remote, manifestBytes);
        const candidate = verifyKnowledgeActivation(Buffer.from(JSON.stringify(activation)), this.options.publicKey);
        if (candidate.identity.sequence !== next.sequence || candidate.identity.version !== next.version
          || candidate.identity.bundleSha256 !== next.sha256) throw new Error("Knowledge manifest mismatch");
        if (!this.enabled) throw new Error("Knowledge updates disabled");
        this.current = await this.cache.commit(candidate);
        this.info.version = this.current.bundle.version;
      }
      this.info.lastSuccessfulUpdate = new Date(this.now()).toISOString();
      this.info.lastError = null;
    } catch (error) {
      this.info.lastError = error instanceof Error ? error.message.slice(0, 300) : "Knowledge update unavailable";
    }
    try {
      await mkdir(this.cache.directory, { recursive: true });
      await writeKnowledgeFile(join(this.cache.directory, "status.json"), JSON.stringify({ lastChecked: this.lastChecked, lastSuccessfulUpdate: this.info.lastSuccessfulUpdate }));
    } catch { /* Status is best-effort and separate from verified proof. */ }
    return this.current;
  }
}
