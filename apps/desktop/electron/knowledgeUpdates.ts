import { createHash, verify } from "node:crypto";
import { mkdir, readFile, rename, writeFile } from "node:fs/promises";
import { join } from "node:path";

export interface KnowledgePage {
  id: string; title: string; type: string; status: string; content: string;
  sha256: string; relatedIds: string[];
}
export interface KnowledgeBundle {
  formatVersion: 1; version: string; sequence: number; publishedAt: string;
  pages: KnowledgePage[];
}
export interface KnowledgeStatus {
  version: string | null; lastSuccessfulUpdate: string | null; lastError: string | null;
}
interface Options {
  directory: string; starterPath: string; publicKey: string; feedUrl: string;
  fetcher?: (url: string, init?: RequestInit) => Promise<Response>;
  now?: () => number;
}
const MAX_BYTES = 2 * 1024 * 1024;
const INTERVAL = 6 * 60 * 60 * 1000;
const digest = (text: string) => createHash("sha256").update(text).digest("hex");
const object = (value: unknown): Record<string, unknown> => {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error("Invalid knowledge object");
  return value as Record<string, unknown>;
};
const shortText = (value: unknown, max = 500): value is string => typeof value === "string" && value.length > 0 && value.length <= max;

export function validateKnowledgeBundle(value: unknown): KnowledgeBundle {
  const bundle = object(value);
  if (bundle.formatVersion !== 1 || !shortText(bundle.version, 128)
    || !Number.isSafeInteger(bundle.sequence) || Number(bundle.sequence) < 0
    || typeof bundle.publishedAt !== "string" || !Number.isFinite(Date.parse(bundle.publishedAt))
    || !Array.isArray(bundle.pages) || bundle.pages.length > 256) throw new Error("Invalid knowledge bundle");
  const ids = new Set<string>();
  for (const item of bundle.pages) {
    const page = object(item);
    if (!shortText(page.id, 256) || !/^[a-zA-Z0-9_-]+(?:\/[a-zA-Z0-9_-]+)*\.md$/.test(page.id)
      || ids.has(page.id) || !shortText(page.title) || !shortText(page.type, 64)
      || !shortText(page.status, 64) || !shortText(page.content, 64 * 1024)
      || Buffer.byteLength(page.content) > 64 * 1024
      || page.sha256 !== digest(page.content) || !Array.isArray(page.relatedIds)
      || page.relatedIds.length > 256 || !page.relatedIds.every((id) => shortText(id, 256))) {
      throw new Error("Invalid knowledge page");
    }
    ids.add(page.id);
  }
  for (const item of bundle.pages) {
    if (!(item as KnowledgePage).relatedIds.every((id) => ids.has(id))) throw new Error("Knowledge relationship escapes bundle");
  }
  if (Buffer.byteLength(JSON.stringify(bundle)) > MAX_BYTES) throw new Error("Knowledge bundle too large");
  return bundle as unknown as KnowledgeBundle;
}

/** Verifies immutable reference data. Neither bundle content nor URLs can select tools. */
export class KnowledgeUpdates {
  private options: Options;
  private current: KnowledgeBundle | null = null;
  private lastChecked = 0;
  private info: KnowledgeStatus = { version: null, lastSuccessfulUpdate: null, lastError: null };
  private inFlight: Promise<KnowledgeBundle> | null = null;
  private enabled = true;
  private cancellation = new AbortController();

  constructor(options: Options) { this.options = options; }
  status(): KnowledgeStatus { return { ...this.info }; }
  setEnabled(enabled: boolean): void {
    this.enabled = enabled;
    if (!enabled) this.cancellation.abort();
    else if (this.cancellation.signal.aborted) this.cancellation = new AbortController();
  }
  private now(): number { return (this.options.now ?? Date.now)(); }
  private decode(text: string): unknown {
    if (Buffer.byteLength(text) > MAX_BYTES) throw new Error("Knowledge response too large");
    const envelope = object(JSON.parse(text));
    if (typeof envelope.payload !== "string" || typeof envelope.signature !== "string"
      || !/^[A-Za-z0-9+/]{86}==$/.test(envelope.signature)
      || !this.options.publicKey
      || !verify(null, Buffer.from(envelope.payload), this.options.publicKey, Buffer.from(envelope.signature, "base64"))) {
      throw new Error("Invalid knowledge signature");
    }
    return JSON.parse(envelope.payload);
  }
  private async atomic(name: string, content: string): Promise<void> {
    await mkdir(this.options.directory, { recursive: true });
    const path = join(this.options.directory, name);
    await writeFile(`${path}.tmp`, content, { mode: 0o600 });
    await rename(`${path}.tmp`, path);
  }
  async load(): Promise<KnowledgeBundle> {
    if (this.current) return this.current;
    const starter = validateKnowledgeBundle(JSON.parse(await readFile(this.options.starterPath, "utf8")));
    this.current = starter;
    for (const name of ["active.json", "previous.json"]) {
      try {
        const candidate = validateKnowledgeBundle(this.decode(await readFile(join(this.options.directory, name), "utf8")));
        if (candidate.sequence >= this.current.sequence) this.current = candidate;
      } catch { /* A missing or damaged cache cannot displace the packaged snapshot. */ }
    }
    try {
      const saved = object(JSON.parse(await readFile(join(this.options.directory, "status.json"), "utf8")));
      if (typeof saved.lastChecked === "number" && saved.lastChecked <= this.now()) this.lastChecked = saved.lastChecked;
      if (typeof saved.lastSuccessfulUpdate === "string") this.info.lastSuccessfulUpdate = saved.lastSuccessfulUpdate;
    } catch { /* Status is advisory; cached content is independently verified. */ }
    this.info.version = this.current.version;
    return this.current;
  }
  check(): Promise<KnowledgeBundle> {
    if (!this.inFlight) this.inFlight = this.checkOnce().finally(() => { this.inFlight = null; });
    return this.inFlight;
  }
  private async download(url: string): Promise<string> {
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
    return Buffer.concat(chunks).toString("utf8");
  }
  private async checkOnce(): Promise<KnowledgeBundle> {
    await this.load();
    if (!this.enabled) return this.current!;
    if (this.lastChecked && this.now() - this.lastChecked < INTERVAL) return this.current!;
    this.lastChecked = this.now();
    try {
      const feed = new URL(this.options.feedUrl);
      if (feed.protocol !== "https:") throw new Error("Knowledge feed requires HTTPS");
      const manifest = object(this.decode(await this.download(feed.href)));
      if (manifest.formatVersion !== 1 || !Array.isArray(manifest.bundles) || manifest.bundles.length > 1000) throw new Error("Invalid knowledge manifest");
      const candidates = manifest.bundles.map(object).filter((entry) => entry.formatVersion === 1);
      for (const entry of candidates) {
        if (!Number.isSafeInteger(entry.sequence) || !shortText(entry.version, 128)
          || typeof entry.path !== "string" || !/^bundles\/[a-zA-Z0-9_-]+\.json$/.test(entry.path)
          || typeof entry.sha256 !== "string" || !/^[a-f0-9]{64}$/.test(entry.sha256)) throw new Error("Invalid knowledge manifest entry");
      }
      candidates.sort((a, b) => Number(b.sequence) - Number(a.sequence));
      const next = candidates[0];
      if (next && Number(next.sequence) > this.current!.sequence) {
        const remote = await this.download(new URL(next.path as string, feed).href);
        if (digest(remote) !== next.sha256) throw new Error("Knowledge checksum mismatch");
        const candidate = validateKnowledgeBundle(this.decode(remote));
        if (!this.enabled) throw new Error("Knowledge updates disabled");
        if (candidate.version !== next.version || candidate.sequence !== next.sequence) throw new Error("Knowledge manifest mismatch");
        try {
          const prior = await readFile(join(this.options.directory, "active.json"), "utf8");
          validateKnowledgeBundle(this.decode(prior));
          await this.atomic("previous.json", prior);
        } catch (error) {
          if ((error as NodeJS.ErrnoException).code !== "ENOENT") {
            // Preserve an independently valid previous snapshot if active was damaged.
            this.info.lastError = "Previous active knowledge snapshot was unavailable";
          }
        }
        await this.atomic("active.json", remote);
        this.current = candidate;
        this.info.version = candidate.version;
      }
      this.info.lastSuccessfulUpdate = new Date(this.now()).toISOString();
      this.info.lastError = null;
    } catch (error) {
      this.info.lastError = error instanceof Error ? error.message.slice(0, 300) : "Knowledge update unavailable";
    }
    await this.atomic("status.json", JSON.stringify({ lastChecked: this.lastChecked, lastSuccessfulUpdate: this.info.lastSuccessfulUpdate })).catch(() => {});
    return this.current!;
  }
}
