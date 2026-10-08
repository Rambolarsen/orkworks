import { randomBytes } from "node:crypto";
import { execFileSync } from "node:child_process";
import { closeSync, constants, existsSync, fstatSync, fsyncSync, linkSync, lstatSync, mkdirSync, openSync, readSync, renameSync, rmSync, statSync, writeSync } from "node:fs";
import { join } from "node:path";
import { TextDecoder } from "node:util";
import fsExt from "fs-ext";

export type ShellMemoryDiagnostic = "corrupt_record" | "unsupported_version" | "lock_timeout" | "write_failed" | "restore_failed" | "stale_revision" | "stale_workspace" | "invalid_input";
export type ShellMemoryResult = { ok: true } | { ok: false; diagnostic: ShellMemoryDiagnostic };
export type ShellDensity = "low" | "high";
export type ShellPreferences = { sessionsWidth: number; inspectorWidth: number; sessionsVisible: boolean; density: ShellDensity };
export const defaultShellPreferences: ShellPreferences = { sessionsWidth: 240, inspectorWidth: 320, sessionsVisible: true, density: "low" };
export type ShellLayoutSnapshot = { preferences: ShellPreferences; revision: number; diagnostic: ShellMemoryDiagnostic | null };

type StoredRecord<P> = { version: 1; epoch: string; revision: number; payload: P };
type Loaded<P> = { record: StoredRecord<P> | null; diagnostic: ShellMemoryDiagnostic | null };
type Pending<P> = {
  base: { epoch: string; revision: number };
  subject: string;
  barrier: boolean;
  valid: () => boolean;
  update: (payload: P, revision: number) => P | null;
  resolve: (result: ShellMemoryResult) => void;
};
type RetainedPrior = { backup: string; dev: number; ino: number; size: number; kind: "regular" | "fifo" };
type Prior = Buffer | RetainedPrior | null;
export type ShellFileReplacer = (temporary: string, target: string, targetExists: boolean) => void;
const decoder = new TextDecoder("utf-8", { fatal: true });
const lockFileName = ".shell-memory.lock";

function exactKeys(value: unknown, keys: string[]): value is Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value)
    && JSON.stringify(Object.keys(value).sort()) === JSON.stringify([...keys].sort());
}

export function validShellPreferences(value: unknown): value is ShellPreferences {
  if (!exactKeys(value, ["sessionsWidth", "inspectorWidth", "sessionsVisible", "density"])) return false;
  return typeof value.sessionsWidth === "number" && Number.isFinite(value.sessionsWidth)
    && value.sessionsWidth >= 200 && value.sessionsWidth <= 320
    && typeof value.inspectorWidth === "number" && Number.isFinite(value.inspectorWidth)
    && value.inspectorWidth >= 280 && value.inspectorWidth <= 420
    && typeof value.sessionsVisible === "boolean"
    && (value.density === "low" || value.density === "high");
}

function newEpoch(): string { return randomBytes(16).toString("hex"); }
function lock(directory: string): number | null {
  mkdirSync(directory, { recursive: true });
  const descriptor = openSync(join(directory, lockFileName), "a+", 0o600);
  for (let attempt = 0; attempt < 50; attempt += 1) {
    try { fsExt.flockSync(descriptor, "exnb"); return descriptor; }
    catch (error) {
      const code = (error as NodeJS.ErrnoException).code;
      if (code !== "EAGAIN" && code !== "EWOULDBLOCK" && code !== "EINTR") break;
      Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 10);
    }
  }
  closeSync(descriptor);
  return null;
}

const replaceFile: ShellFileReplacer = (temporary, target, targetExists) => {
  if (process.platform === "win32" && targetExists) {
    execFileSync("powershell.exe", ["-NoLogo", "-NoProfile", "-NonInteractive", "-Command",
      "$ErrorActionPreference = 'Stop'; [System.IO.File]::Replace($env:ORKWORKS_SHELL_TEMPORARY, $env:ORKWORKS_SHELL_TARGET, $null, $true)"], {
      env: { ...process.env, ORKWORKS_SHELL_TEMPORARY: temporary, ORKWORKS_SHELL_TARGET: target }, stdio: "ignore",
    });
  } else renameSync(temporary, target);
};

export class RevisionedShellMemory<P> {
  private directory: string;
  private fileName: string;
  private limit: number;
  private initial: () => P;
  private validPayload: (value: unknown) => value is P;
  private replacer: ShellFileReplacer;
  private restoreReplacer: ShellFileReplacer;
  private snapshot: StoredRecord<P> | null = null;
  private pending: Pending<P>[] = [];
  private scheduled = false;
  private proof: Array<{ from: number; to: number; epoch: string }> = [];
  private needsRefresh = false;

  constructor(directory: string, fileName: string, limit: number,
    initial: () => P, validPayload: (value: unknown) => value is P,
    replacer: ShellFileReplacer = replaceFile,
    restoreReplacer: ShellFileReplacer = replaceFile) {
    this.directory = directory;
    this.fileName = fileName;
    this.limit = limit;
    this.initial = initial;
    this.validPayload = validPayload;
    this.replacer = replacer;
    this.restoreReplacer = restoreReplacer;
  }

  private path(): string { return join(this.directory, this.fileName); }
  private readTarget(): Buffer | "oversize" | "non_regular" | null {
    if (!existsSync(this.path())) return null;
    // O_NONBLOCK prevents a replaced FIFO from freezing Electron before fstat
    // can reject it. It is ignored for ordinary files on supported platforms.
    const descriptor = openSync(this.path(), constants.O_RDONLY | constants.O_NONBLOCK);
    try {
      const info = fstatSync(descriptor);
      if (!info.isFile()) return "non_regular";
      if (info.size > this.limit) return "oversize";
      // A file can grow after fstat. The extra byte detects that race without
      // ever allocating or reading more than the record bound plus one.
      const bytes = Buffer.allocUnsafe(this.limit + 1);
      let length = 0;
      while (length < bytes.byteLength) {
        const count = readSync(descriptor, bytes, length, bytes.byteLength - length, null);
        if (count === 0) break;
        length += count;
      }
      return length > this.limit ? "oversize" : bytes.subarray(0, length);
    } finally { closeSync(descriptor); }
  }

  private load(): Loaded<P> {
    try {
      const bytes = this.readTarget();
      if (bytes === null) return { record: null, diagnostic: null };
      if (bytes === "oversize" || bytes === "non_regular") return { record: null, diagnostic: "corrupt_record" };
      const value: unknown = JSON.parse(decoder.decode(bytes));
      if (value !== null && typeof value === "object" && !Array.isArray(value)
        && "version" in value && value.version !== 1) return { record: null, diagnostic: "unsupported_version" };
      if (!exactKeys(value, ["version", "epoch", "revision", "payload"])
        || value.version !== 1 || typeof value.epoch !== "string" || !/^[0-9a-f]{32}$/.test(value.epoch)
        || typeof value.revision !== "number" || !Number.isSafeInteger(value.revision) || value.revision < 0
        || !this.validPayload(value.payload)) return { record: null, diagnostic: "corrupt_record" };
      return { record: value as StoredRecord<P>, diagnostic: null };
    } catch { return { record: null, diagnostic: "corrupt_record" }; }
  }

  private publish(bytes: Buffer, replacer: ShellFileReplacer = this.replacer): boolean {
    const temporary = join(this.directory, `.${this.fileName}.${process.pid}.${randomBytes(8).toString("hex")}.tmp`);
    let descriptor: number | null = null;
    try {
      descriptor = openSync(temporary, "wx", 0o600);
      let offset = 0;
      while (offset < bytes.byteLength) {
        const written = writeSync(descriptor, bytes, offset, bytes.byteLength - offset, null);
        if (written <= 0) throw new Error("Shell record write made no progress");
        offset += written;
      }
      fsyncSync(descriptor);
      closeSync(descriptor); descriptor = null;
      replacer(temporary, this.path(), existsSync(this.path()));
      // Candidate and restore payloads fit the record bound. A misbehaving
      // replacer or external writer can still publish a larger target.
      const observed = this.readTarget();
      return Buffer.isBuffer(observed) && observed.equals(bytes);
    }
    finally { if (descriptor !== null) closeSync(descriptor); rmSync(temporary, { force: true }); }
  }

  private capturePrior(allowFifo = false): Prior {
    const previous = this.readTarget();
    if (previous === "non_regular") {
      const target = this.path();
      const info = lstatSync(target);
      if (!allowFifo || !info.isFIFO()) throw new Error("Prior shell target is not a rebuildable file");
      const backup = join(this.directory, `.${this.fileName}.${process.pid}.${randomBytes(8).toString("hex")}.bak`);
      try {
        linkSync(target, backup);
        const retained = lstatSync(backup);
        if (!retained.isFIFO() || retained.dev !== info.dev || retained.ino !== info.ino)
          throw new Error("FIFO backup did not preserve the prior inode");
        return { backup, dev: retained.dev, ino: retained.ino, size: retained.size, kind: "fifo" };
      } catch (error) { rmSync(backup, { force: true }); throw error; }
    }
    if (previous !== "oversize") return previous;
    // A hardlink preserves an oversized prior inode for rollback without
    // buffering it or copying attacker-sized bytes. If unsupported, decline
    // the rebuild before publication.
    const target = this.path();
    if (!lstatSync(target).isFile()) throw new Error("Oversized prior target is not a regular file");
    const backup = join(this.directory, `.${this.fileName}.${process.pid}.${randomBytes(8).toString("hex")}.bak`);
    try {
      linkSync(target, backup);
      const info = statSync(backup);
      return { backup, dev: info.dev, ino: info.ino, size: info.size, kind: "regular" };
    } catch (error) { rmSync(backup, { force: true }); throw error; }
  }

  private priorInPlace(previous: RetainedPrior): boolean {
    try {
      const current = lstatSync(this.path());
      return current.dev === previous.dev && current.ino === previous.ino && current.size === previous.size
        && (previous.kind === "fifo" ? current.isFIFO() : current.isFile());
    } catch { return false; }
  }

  private failedWrite(previous: Prior, attempted: Buffer): ShellMemoryResult {
    // A compliant peer cannot write while our retained lock is held. Still,
    // never roll back a valid but unexpected record: it may be a newer write
    // from a noncompliant external actor. The exact attempted bytes identify
    // our own publication even if the replacer threw after publishing it.
    if (previous !== null && !Buffer.isBuffer(previous) && this.priorInPlace(previous)) {
      return { ok: false, diagnostic: "write_failed" };
    }
    let currentBytes: Buffer | "oversize" | "non_regular" | null;
    try { currentBytes = this.readTarget(); }
    catch { return { ok: false, diagnostic: "restore_failed" }; }
    if (currentBytes === "oversize" || currentBytes === "non_regular") return { ok: false, diagnostic: "write_failed" };
    if (currentBytes !== null && !currentBytes.equals(attempted)) {
      const observed = this.load();
      if (observed.record) return { ok: false, diagnostic: "write_failed" };
      try {
        const parsed: unknown = JSON.parse(decoder.decode(currentBytes));
        if (parsed !== null && typeof parsed === "object" && !Array.isArray(parsed)
          && ("version" in parsed || "epoch" in parsed || "revision" in parsed)) {
          return { ok: false, diagnostic: "write_failed" };
        }
      } catch { /* Malformed JSON or UTF-8 may be restored below. */ }
    }
    if (previous === null) {
      // Initialization had no prior record. After excluding a recognizable
      // competing record above, restore the prior absence under the same lock.
      try {
        rmSync(this.path(), { force: true });
        if (!existsSync(this.path())) return { ok: false, diagnostic: "write_failed" };
      } catch { /* Absence could not be confirmed. */ }
      return { ok: false, diagnostic: "restore_failed" };
    }
    if (!Buffer.isBuffer(previous)) {
      try {
        this.restoreReplacer(previous.backup, this.path(), existsSync(this.path()));
        if (this.priorInPlace(previous)) return { ok: false, diagnostic: "write_failed" };
      } catch { /* The prior inode could not be confirmed at the target. */ }
      return { ok: false, diagnostic: "restore_failed" };
    }
    try {
      if (currentBytes !== null && Buffer.isBuffer(currentBytes) && currentBytes.equals(previous)) {
        return { ok: false, diagnostic: "write_failed" };
      }
      if (this.publish(previous, this.restoreReplacer)) return { ok: false, diagnostic: "write_failed" };
    } catch { /* The caller must know preservation could not be verified. */ }
    return { ok: false, diagnostic: "restore_failed" };
  }

  private write(record: StoredRecord<P>, allowFifoRebuild = false): ShellMemoryResult {
    const bytes = Buffer.from(`${JSON.stringify(record)}\n`, "utf8");
    if (bytes.byteLength > this.limit) return { ok: false, diagnostic: "invalid_input" };
    // Every caller holds the shared lock. Keep the exact prior bytes so a
    // failed read-back can restore them without rewriting a normalized record.
    let previous: Prior;
    try { previous = this.capturePrior(allowFifoRebuild); }
    catch { return { ok: false, diagnostic: "write_failed" }; }
    try {
      if (!this.publish(bytes)) return this.failedWrite(previous, bytes);
      this.snapshot = record;
      return { ok: true };
    } catch { return this.failedWrite(previous, bytes); }
    finally { if (previous !== null && !Buffer.isBuffer(previous)) rmSync(previous.backup, { force: true }); }
  }

  readRecord(): Loaded<P> {
    let descriptor: number | null = null;
    try {
      descriptor = lock(this.directory);
      if (descriptor === null) return { record: null, diagnostic: "lock_timeout" };
      let loaded = this.load();
      if (loaded.diagnostic === null && loaded.record === null) {
        const first: StoredRecord<P> = { version: 1, epoch: newEpoch(), revision: 0, payload: this.initial() };
        const written = this.write(first);
        if (!written.ok) return { record: null, diagnostic: written.diagnostic };
        loaded = this.load();
      }
      this.snapshot = loaded.record;
      this.needsRefresh = false;
      return loaded;
    } catch { return { record: null, diagnostic: "write_failed" }; }
    finally { if (descriptor !== null) closeSync(descriptor); }
  }

  enqueue(subject: string, valid: () => boolean, update: Pending<P>["update"], barrier = false): Promise<ShellMemoryResult> {
    if (!this.snapshot || this.needsRefresh) {
      const read = this.readRecord();
      if (!read.record) return Promise.resolve({ ok: false, diagnostic: read.diagnostic ?? "corrupt_record" });
    }
    const retained: Pending<P>[] = [];
    for (const pending of this.pending) {
      if (!pending.barrier && pending.subject === subject) pending.resolve({ ok: false, diagnostic: "stale_revision" });
      else retained.push(pending);
    }
    this.pending = retained;
    const base = { epoch: this.snapshot!.epoch, revision: this.snapshot!.revision };
    return new Promise((resolve) => {
      this.pending.push({ base, subject, barrier, valid, update, resolve });
      this.schedule();
    });
  }

  private schedule(): void {
    if (this.scheduled) return;
    this.scheduled = true;
    queueMicrotask(() => {
      this.scheduled = false;
      const intent = this.pending.shift();
      if (!intent) { this.proof = []; return; }
      const result = this.commit(intent);
      intent.resolve(result);
      if (!result.ok) {
        for (const dependent of this.pending) dependent.resolve({ ok: false, diagnostic: "stale_revision" });
        this.pending = []; this.proof = []; this.needsRefresh = true;
      } else if (intent.barrier) this.proof = [];
      if (this.pending.length) this.schedule(); else this.proof = [];
    });
  }

  private commit(intent: Pending<P>): ShellMemoryResult {
    if (!intent.valid()) return { ok: false, diagnostic: "stale_workspace" };
    let expectedRevision = intent.base.revision;
    for (const step of this.proof) {
      if (step.epoch === intent.base.epoch && step.from === expectedRevision) expectedRevision = step.to;
    }
    let descriptor: number | null = null;
    try {
      descriptor = lock(this.directory);
      if (descriptor === null) return { ok: false, diagnostic: "lock_timeout" };
      const loaded = this.load();
      if (!loaded.record) return { ok: false, diagnostic: loaded.diagnostic ?? "stale_revision" };
      if (loaded.record.epoch !== intent.base.epoch || loaded.record.revision !== expectedRevision)
        return { ok: false, diagnostic: "stale_revision" };
      if (!intent.valid()) return { ok: false, diagnostic: "stale_workspace" };
      if (loaded.record.revision === Number.MAX_SAFE_INTEGER) return { ok: false, diagnostic: "write_failed" };
      const payload = intent.update(loaded.record.payload, loaded.record.revision + 1);
      if (payload === null || !this.validPayload(payload)) return { ok: false, diagnostic: "invalid_input" };
      const next: StoredRecord<P> = { ...loaded.record, revision: loaded.record.revision + 1, payload };
      const result = this.write(next);
      if (result.ok && !intent.barrier) this.proof.push({ epoch: next.epoch, from: loaded.record.revision, to: next.revision });
      return result;
    } catch { return { ok: false, diagnostic: "write_failed" }; }
    finally { if (descriptor !== null) closeSync(descriptor); }
  }

  rebuild(confirmed: true): Promise<ShellMemoryResult> {
    if (confirmed !== true) return Promise.resolve({ ok: false, diagnostic: "invalid_input" });
    for (const pending of this.pending) pending.resolve({ ok: false, diagnostic: "stale_revision" });
    this.pending = []; this.proof = [];
    let descriptor: number | null = null;
    try {
      descriptor = lock(this.directory);
      if (descriptor === null) return Promise.resolve({ ok: false, diagnostic: "lock_timeout" });
      const result = this.write({ version: 1, epoch: newEpoch(), revision: 0, payload: this.initial() }, true);
      this.needsRefresh = !result.ok;
      return Promise.resolve(result);
    } catch { this.needsRefresh = true; return Promise.resolve({ ok: false, diagnostic: "write_failed" }); }
    finally { if (descriptor !== null) closeSync(descriptor); }
  }
}

export function shellLayoutMemoryPath(directory: string): string { return join(directory, "shell-layout.json"); }
export function createShellLayoutMemory(directory: string, replacer?: ShellFileReplacer, restoreReplacer?: ShellFileReplacer) {
  const memory = new RevisionedShellMemory(directory, "shell-layout.json", 16 * 1024,
    () => ({ ...defaultShellPreferences }), validShellPreferences, replacer, restoreReplacer);
  return {
    read: (): ShellLayoutSnapshot => {
      const loaded = memory.readRecord();
      return { preferences: loaded.record?.payload ?? { ...defaultShellPreferences }, revision: loaded.record?.revision ?? 0, diagnostic: loaded.diagnostic };
    },
    save: (preferences: unknown): Promise<ShellMemoryResult> => validShellPreferences(preferences)
      ? memory.enqueue("layout", () => true, () => ({ ...preferences }))
      : Promise.resolve({ ok: false, diagnostic: "invalid_input" }),
    reset: (): Promise<ShellMemoryResult> => memory.enqueue("layout", () => true, () => ({ ...defaultShellPreferences }), true),
    rebuild: (confirmed: true): Promise<ShellMemoryResult> => memory.rebuild(confirmed),
  };
}
