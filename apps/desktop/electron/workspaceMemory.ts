import { randomBytes } from "node:crypto";
import { execFileSync } from "node:child_process";
import {
  closeSync,
  existsSync,
  fsyncSync,
  mkdirSync,
  openSync,
  readFileSync,
  realpathSync,
  renameSync,
  rmSync,
  statSync,
  writeSync,
} from "node:fs";
import { join } from "node:path";
import { TextDecoder } from "node:util";
import fsExt from "fs-ext";

export interface WorkspaceMemoryDiagnostic {
  code: "corrupt_history" | "history_lock_timeout" | "history_write_failed" | "pin_limit_reached";
  message: string;
}

export interface AppWorkspaceMemory {
  version: 2;
  revision: number;
  lastWorkspacePath: string | null;
  recentWorkspacePaths: string[];
  pinnedWorkspacePaths: string[];
  diagnostic: WorkspaceMemoryDiagnostic | null;
}

interface StoredWorkspaceMemory {
  version: 2;
  revision: number;
  lastWorkspacePath: string | null;
  recentWorkspacePaths: string[];
  pinnedWorkspacePaths: string[];
}

// The pre-this-feature (ADR 0060) on-disk shape. Read-only: never written by
// this module, only migrated forward. See ADR 0061.
interface V1StoredWorkspaceMemory {
  version: 1;
  revision: number;
  lastWorkspacePath: string | null;
  recentWorkspacePaths: string[];
}

const fileName = "workspace-memory.json";
const lockFileName = ".workspace-memory.lock";
const maximumRecentPaths = 20;
const maximumPinnedPaths = 50;
const maximumSerializedBytes = 64 * 1024;
// The pre-#569 writer's actual cap (see validLegacyStoredMemory) — never 20.
const maximumLegacyRecentPaths = 10;
// Pre-#569 files predate any byte bound: the old writer capped at 10 entries
// with no size limit, so long (e.g. Windows extended-length) paths could
// legitimately exceed maximumSerializedBytes. Gate parsing at a sanity
// ceiling derived from the old writer's actual worst case instead of
// rejecting such files before they're even inspected: up to
// maximumLegacyRecentPaths entries plus one duplicate of lastWorkspacePath,
// each up to the Windows extended-length maximum of 32,767 UTF-16 code
// units (~98,301 bytes worst-case UTF-8) — roughly 1.05 MiB including JSON
// overhead. 2 MiB leaves comfortable headroom. migratedLegacyMemory still
// trims the result to fit maximumSerializedBytes via boundedPaths.
const maximumLegacySourceBytes = 2 * 1024 * 1024;
const lockRetryCount = 50;
const lockRetryDelayMs = 10;
const utf8Decoder = new TextDecoder("utf-8", { fatal: true });

export type WorkspaceHistoryReplacer = (
  temporary: string,
  target: string,
  targetExists: boolean,
) => void;

const corruptDiagnostic: WorkspaceMemoryDiagnostic = {
  code: "corrupt_history",
  message: "Workspace history is corrupt or unreadable and was left unchanged.",
};

const emptyMemory = (): AppWorkspaceMemory => ({
  version: 2,
  revision: 0,
  lastWorkspacePath: null,
  recentWorkspacePaths: [],
  pinnedWorkspacePaths: [],
  diagnostic: null,
});

function withDiagnostic(
  memory: AppWorkspaceMemory,
  diagnostic: WorkspaceMemoryDiagnostic,
): AppWorkspaceMemory {
  return { ...memory, diagnostic };
}

function storedMemory(memory: AppWorkspaceMemory): StoredWorkspaceMemory {
  return {
    version: 2,
    revision: memory.revision,
    lastWorkspacePath: memory.lastWorkspacePath,
    recentWorkspacePaths: memory.recentWorkspacePaths,
    pinnedWorkspacePaths: memory.pinnedWorkspacePaths,
  };
}

function serializedMemory(memory: StoredWorkspaceMemory): string {
  return `${JSON.stringify(memory, null, 2)}\n`;
}

function memoryFits(memory: StoredWorkspaceMemory): boolean {
  return Buffer.byteLength(serializedMemory(memory), "utf8") <= maximumSerializedBytes;
}

// Builds the recentWorkspacePaths list for a write: puts `lastWorkspacePath`
// first (unless it is pinned — a pinned path never lives in both lists),
// dedupes against `paths`, bounds to maximumRecentPaths entries, then trims
// further until the candidate record (including the caller's current
// pinnedWorkspacePaths, which count toward the same byte budget) fits.
function boundedPaths(
  lastWorkspacePath: string | null,
  paths: readonly string[],
  pinnedWorkspacePaths: readonly string[],
  revision: number,
): string[] | null {
  const deduplicated = [
    ...(lastWorkspacePath === null || pinnedWorkspacePaths.includes(lastWorkspacePath) ? [] : [lastWorkspacePath]),
    ...paths,
  ].filter((value, index, values) => values.indexOf(value) === index);
  const bounded = deduplicated.slice(0, maximumRecentPaths);
  const candidate = (recentWorkspacePaths: string[]): StoredWorkspaceMemory => ({
    version: 2,
    revision,
    lastWorkspacePath,
    recentWorkspacePaths,
    pinnedWorkspacePaths: [...pinnedWorkspacePaths],
  });

  while (bounded.length > 1 && !memoryFits(candidate(bounded))) {
    bounded.pop();
  }
  return memoryFits(candidate(bounded)) ? bounded : null;
}

interface LegacyStoredWorkspaceMemory {
  lastWorkspacePath: string | null;
  recentWorkspacePaths: string[];
}

// Pre-#569 files predate the version/revision fields and the invariants
// validV1StoredMemory enforces (deduplication, lastWorkspacePath inclusion).
// Accept the looser shape the old writer actually produced and normalize it
// through boundedPaths rather than rejecting installs' existing history as
// corrupt.
function validLegacyStoredMemory(value: unknown): value is LegacyStoredWorkspaceMemory {
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  const raw = value as Record<string, unknown>;
  const keys = Object.keys(raw).sort();
  if (JSON.stringify(keys) !== JSON.stringify([
    "lastWorkspacePath",
    "recentWorkspacePaths",
  ])) return false;
  return (raw.lastWorkspacePath === null || typeof raw.lastWorkspacePath === "string")
    && Array.isArray(raw.recentWorkspacePaths)
    && raw.recentWorkspacePaths.length <= maximumLegacyRecentPaths
    && raw.recentWorkspacePaths.every((entry) => typeof entry === "string");
}

// The pre-#569 writer never canonicalized paths (it stored the raw dialog
// selection), while specs/multi-workspace.md requires persisting canonical
// paths only. realpathSync.native is also a synchronous, potentially slow
// syscall (a disconnected UNC share or other unreachable network mount can
// block for the OS network timeout), and readWorkspaceMemory runs on
// Electron's main thread before the window is created — resolving every
// recentWorkspacePaths entry there risks the app appearing hung on launch.
//
// Rather than choosing between an unbounded synchronous cost (canonicalize
// everything) and a spec-violating one (persist raw aliases), secondary
// entries are dropped instead of carried forward: they're low-stakes
// convenience shortcuts (removing one never touches project files or
// session data) that naturally repopulate, correctly canonicalized, as the
// user reopens workspaces going forward. Only lastWorkspacePath — the one
// entry guaranteed to matter immediately, since it's what gets
// auto-restored at startup — is resolved, bounding the worst-case startup
// stall to a single call.
function canonicalizedLegacyPath(path: string): string {
  return canonicalWorkspacePath(path) ?? path;
}

function migratedLegacyMemory(legacy: LegacyStoredWorkspaceMemory): AppWorkspaceMemory {
  const lastWorkspacePath = legacy.lastWorkspacePath === null
    ? null
    : canonicalizedLegacyPath(legacy.lastWorkspacePath);
  // boundedPaths prepends lastWorkspacePath itself, so an empty paths list
  // is enough to produce the single-entry (or empty) result. No pinned
  // paths exist yet at this migration tier.
  const bounded = boundedPaths(lastWorkspacePath, [], [], 0);
  // boundedPaths only returns null when even a single entry can't fit under
  // the serialized-size bound. Silently dropping recentWorkspacePaths would
  // produce a lastWorkspacePath not present in either list, violating the
  // same invariant validStoredMemory enforces for the current format —
  // surface a diagnostic instead, matching how every other "doesn't fit"
  // case here fails loud rather than discarding data quietly.
  if (bounded === null) return withDiagnostic(emptyMemory(), corruptDiagnostic);
  return {
    version: 2,
    revision: 0,
    lastWorkspacePath,
    recentWorkspacePaths: bounded,
    pinnedWorkspacePaths: [],
    diagnostic: null,
  };
}

// ADR 0060's original shape, unchanged: version 1, no pinnedWorkspacePaths,
// lastWorkspacePath must appear in recentWorkspacePaths. Never written by
// this module; only migrated forward to v2. See ADR 0061.
function validV1StoredMemory(value: unknown): value is V1StoredWorkspaceMemory {
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  const raw = value as Record<string, unknown>;
  const keys = Object.keys(raw).sort();
  if (JSON.stringify(keys) !== JSON.stringify([
    "lastWorkspacePath",
    "recentWorkspacePaths",
    "revision",
    "version",
  ])) return false;
  return raw.version === 1
    && typeof raw.revision === "number"
    && Number.isSafeInteger(raw.revision)
    && raw.revision >= 0
    && (raw.lastWorkspacePath === null || typeof raw.lastWorkspacePath === "string")
    && Array.isArray(raw.recentWorkspacePaths)
    && raw.recentWorkspacePaths.length <= maximumRecentPaths
    && raw.recentWorkspacePaths.every((entry) => typeof entry === "string")
    && new Set(raw.recentWorkspacePaths).size === raw.recentWorkspacePaths.length
    && (raw.lastWorkspacePath === null || raw.recentWorkspacePaths.includes(raw.lastWorkspacePath));
}

function migratedV1Memory(v1: V1StoredWorkspaceMemory): AppWorkspaceMemory {
  return {
    version: 2,
    revision: v1.revision,
    lastWorkspacePath: v1.lastWorkspacePath,
    recentWorkspacePaths: [...v1.recentWorkspacePaths],
    pinnedWorkspacePaths: [],
    diagnostic: null,
  };
}

function validStoredMemory(value: unknown): value is StoredWorkspaceMemory {
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  const raw = value as Record<string, unknown>;
  const keys = Object.keys(raw).sort();
  if (JSON.stringify(keys) !== JSON.stringify([
    "lastWorkspacePath",
    "pinnedWorkspacePaths",
    "recentWorkspacePaths",
    "revision",
    "version",
  ])) return false;
  if (
    raw.version !== 2
    || typeof raw.revision !== "number"
    || !Number.isSafeInteger(raw.revision)
    || raw.revision < 0
    || (raw.lastWorkspacePath !== null && typeof raw.lastWorkspacePath !== "string")
    || !Array.isArray(raw.recentWorkspacePaths)
    || !Array.isArray(raw.pinnedWorkspacePaths)
    || raw.recentWorkspacePaths.length > maximumRecentPaths
    || raw.pinnedWorkspacePaths.length > maximumPinnedPaths
    || !raw.recentWorkspacePaths.every((entry) => typeof entry === "string")
    || !raw.pinnedWorkspacePaths.every((entry) => typeof entry === "string")
  ) return false;

  const recentSet = new Set(raw.recentWorkspacePaths);
  const pinnedSet = new Set(raw.pinnedWorkspacePaths);
  if (recentSet.size !== raw.recentWorkspacePaths.length) return false;
  if (pinnedSet.size !== raw.pinnedWorkspacePaths.length) return false;
  for (const path of raw.recentWorkspacePaths) {
    if (pinnedSet.has(path)) return false; // a path must live in at most one list
  }
  if (raw.lastWorkspacePath === null) return true;
  return recentSet.has(raw.lastWorkspacePath) || pinnedSet.has(raw.lastWorkspacePath);
}

function readStoredWorkspaceMemory(userDataPath: string): AppWorkspaceMemory {
  const target = workspaceMemoryPath(userDataPath);
  if (!existsSync(target)) return emptyMemory();

  try {
    const source = readFileSync(target);
    if (source.byteLength > maximumLegacySourceBytes) return withDiagnostic(emptyMemory(), corruptDiagnostic);
    const parsed: unknown = JSON.parse(utf8Decoder.decode(source));
    if (validStoredMemory(parsed)) {
      // The v2 writer never produces a file over maximumSerializedBytes, so
      // enforce that tighter bound here now that the shape is confirmed
      // current-format rather than legacy/v1.
      if (source.byteLength > maximumSerializedBytes || !memoryFits(parsed)) {
        return withDiagnostic(emptyMemory(), corruptDiagnostic);
      }
      return {
        version: 2,
        revision: parsed.revision,
        lastWorkspacePath: parsed.lastWorkspacePath,
        recentWorkspacePaths: [...parsed.recentWorkspacePaths],
        pinnedWorkspacePaths: [...parsed.pinnedWorkspacePaths],
        diagnostic: null,
      };
    }
    if (validV1StoredMemory(parsed)) return migratedV1Memory(parsed);
    if (validLegacyStoredMemory(parsed)) return migratedLegacyMemory(parsed);
    return withDiagnostic(emptyMemory(), corruptDiagnostic);
  } catch {
    return withDiagnostic(emptyMemory(), corruptDiagnostic);
  }
}

function sleepForLockRetry(): void {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, lockRetryDelayMs);
}

function acquireHistoryLock(userDataPath: string): number | null {
  let descriptor: number;
  try {
    descriptor = openSync(join(userDataPath, lockFileName), "a+", 0o600);
  } catch {
    return null;
  }
  // Keep this inode permanently. Unlinking or renaming it could let another
  // process lock a different inode and enter the critical section concurrently.
  // flock/LockFileEx releases ownership on close or process exit, never by age.
  for (let attempt = 0; attempt < lockRetryCount; attempt += 1) {
    try {
      fsExt.flockSync(descriptor, "exnb");
      return descriptor;
    } catch (error) {
      const code = (error as NodeJS.ErrnoException).code;
      if (code !== "EAGAIN" && code !== "EWOULDBLOCK" && code !== "EINTR") break;
      sleepForLockRetry();
    }
  }
  closeSync(descriptor);
  return null;
}

function replaceExistingWorkspaceHistoryOnWindows(temporary: string, target: string): void {
  // PowerShell's [System.IO.File]::Replace delegates to ReplaceFileW, which
  // atomically replaces an existing file on Windows. Passing paths as
  // environment values keeps arbitrary workspace paths out of the command
  // parser and preserves the error if the native operation fails.
  execFileSync("powershell.exe", [
    "-NoLogo",
    "-NoProfile",
    "-NonInteractive",
    "-Command",
    "$ErrorActionPreference = 'Stop'; [System.IO.File]::Replace($env:ORKWORKS_HISTORY_TEMPORARY, $env:ORKWORKS_HISTORY_TARGET, $null, $true)",
  ], {
    env: {
      ...process.env,
      ORKWORKS_HISTORY_TEMPORARY: temporary,
      ORKWORKS_HISTORY_TARGET: target,
    },
    stdio: "ignore",
  });
}

const replaceWorkspaceHistoryFile: WorkspaceHistoryReplacer = (temporary, target, targetExists) => {
  if (process.platform === "win32" && targetExists) {
    replaceExistingWorkspaceHistoryOnWindows(temporary, target);
    return;
  }
  // POSIX rename and the new-target Windows path both publish the fully
  // flushed temporary file in one filesystem operation.
  renameSync(temporary, target);
};

function writeAndVerify(
  userDataPath: string,
  current: AppWorkspaceMemory,
  next: StoredWorkspaceMemory,
  replaceFile: WorkspaceHistoryReplacer,
): AppWorkspaceMemory {
  const target = workspaceMemoryPath(userDataPath);
  const temporary = join(
    userDataPath,
    `.workspace-memory.${process.pid}.${randomBytes(8).toString("hex")}.tmp`,
  );
  let descriptor: number | null = null;
  let replacementCompleted = false;
  try {
    descriptor = openSync(temporary, "wx", 0o600);
    const bytes = Buffer.from(serializedMemory(next), "utf8");
    let offset = 0;
    while (offset < bytes.byteLength) {
      const written = writeSync(descriptor, bytes, offset, bytes.byteLength - offset, null);
      if (written <= 0) throw new Error("Workspace history write made no progress.");
      offset += written;
    }
    fsyncSync(descriptor);
    closeSync(descriptor);
    descriptor = null;
    replaceFile(temporary, target, existsSync(target));
    replacementCompleted = true;
  } catch {
    return withDiagnostic(current, {
      code: "history_write_failed",
      message: "Workspace history could not be saved; the ready workspace was kept.",
    });
  } finally {
    if (descriptor !== null) closeSync(descriptor);
    rmSync(temporary, { force: true });
  }

  if (replacementCompleted) {
    const observed = readStoredWorkspaceMemory(userDataPath);
    if (
      observed.diagnostic === null
      && JSON.stringify(storedMemory(observed)) === JSON.stringify(next)
    ) return observed;
  }
  return withDiagnostic(current, {
    code: "history_write_failed",
    message: "Workspace history could not be confirmed after replacement.",
  });
}

const noChange = Symbol("no workspace history change");
const tooLarge = Symbol("workspace history entry too large");
const pinLimitReached = Symbol("workspace history pin limit reached");
type UpdateResult = StoredWorkspaceMemory | typeof noChange | typeof tooLarge | typeof pinLimitReached;

function updateWorkspaceMemory(
  userDataPath: string,
  update: (current: AppWorkspaceMemory) => UpdateResult,
  replaceFile: WorkspaceHistoryReplacer = replaceWorkspaceHistoryFile,
): AppWorkspaceMemory {
  try {
    mkdirSync(userDataPath, { recursive: true });
  } catch {
    return withDiagnostic(emptyMemory(), {
      code: "history_write_failed",
      message: "Workspace history could not be saved; the ready workspace was kept.",
    });
  }

  const lock = acquireHistoryLock(userDataPath);
  if (lock === null) {
    return withDiagnostic(readStoredWorkspaceMemory(userDataPath), {
      code: "history_lock_timeout",
      message: "Workspace history was busy and could not be updated.",
    });
  }

  try {
    const current = readStoredWorkspaceMemory(userDataPath);
    if (current.diagnostic !== null) return current;
    const next = update(current);
    if (next === noChange) return current;
    if (current.revision === Number.MAX_SAFE_INTEGER) {
      return withDiagnostic(current, {
        code: "history_write_failed",
        message: "Workspace history revision is exhausted; the file was left unchanged.",
      });
    }
    if (next === pinLimitReached) {
      return withDiagnostic(current, {
        code: "pin_limit_reached",
        message: `Only ${maximumPinnedPaths} workspaces can be pinned at a time.`,
      });
    }
    if (next === tooLarge) {
      return withDiagnostic(current, {
        code: "history_write_failed",
        message: "Workspace path is too large to fit in the bounded history file.",
      });
    }
    if (!memoryFits(next)) {
      return withDiagnostic(current, {
        code: "history_write_failed",
        message: "Workspace path is too large to fit in the bounded history file.",
      });
    }
    return writeAndVerify(userDataPath, current, next, replaceFile);
  } finally {
    closeSync(lock);
  }
}

export function workspaceMemoryPath(userDataPath: string): string {
  return join(userDataPath, fileName);
}

export function readWorkspaceMemory(userDataPath: string): AppWorkspaceMemory {
  return readStoredWorkspaceMemory(userDataPath);
}

export function canonicalWorkspacePath(workspacePath: string): string | null {
  try {
    return realpathSync.native(workspacePath);
  } catch {
    return null;
  }
}

export function accessibleWorkspaceDirectoryPath(workspacePath: string): string | null {
  try {
    if (!statSync(workspacePath).isDirectory()) return null;
    return realpathSync.native(workspacePath);
  } catch {
    return null;
  }
}

export function rememberWorkspacePath(
  userDataPath: string,
  workspacePath: string,
  replaceFile: WorkspaceHistoryReplacer = replaceWorkspaceHistoryFile,
): AppWorkspaceMemory {
  return updateWorkspaceMemory(userDataPath, (current) => {
    if (current.pinnedWorkspacePaths.includes(workspacePath)) {
      // Opening an already-pinned workspace only updates lastWorkspacePath —
      // it must never be duplicated into recentWorkspacePaths (a path lives
      // in at most one list; see ADR 0061).
      if (current.lastWorkspacePath === workspacePath) return noChange;
      return {
        version: 2,
        revision: current.revision + 1,
        lastWorkspacePath: workspacePath,
        recentWorkspacePaths: current.recentWorkspacePaths,
        pinnedWorkspacePaths: current.pinnedWorkspacePaths,
      };
    }
    const recentWorkspacePaths = boundedPaths(workspacePath, [
      workspacePath,
      ...current.recentWorkspacePaths.filter((path) => path !== workspacePath),
    ], current.pinnedWorkspacePaths, current.revision + 1);
    if (recentWorkspacePaths === null) return tooLarge;
    return {
      version: 2,
      revision: current.revision + 1,
      lastWorkspacePath: workspacePath,
      recentWorkspacePaths,
      pinnedWorkspacePaths: current.pinnedWorkspacePaths,
    };
  }, replaceFile);
}

export function forgetWorkspacePath(
  userDataPath: string,
  workspacePath: string,
  replaceFile: WorkspaceHistoryReplacer = replaceWorkspaceHistoryFile,
): AppWorkspaceMemory {
  return updateWorkspaceMemory(userDataPath, (current) => {
    const recentWorkspacePaths = current.recentWorkspacePaths.filter((path) => path !== workspacePath);
    const pinnedWorkspacePaths = current.pinnedWorkspacePaths.filter((path) => path !== workspacePath);
    const changed = recentWorkspacePaths.length !== current.recentWorkspacePaths.length
      || pinnedWorkspacePaths.length !== current.pinnedWorkspacePaths.length
      || current.lastWorkspacePath === workspacePath;
    if (!changed) return noChange;
    return {
      version: 2,
      revision: current.revision + 1,
      lastWorkspacePath: current.lastWorkspacePath === workspacePath ? null : current.lastWorkspacePath,
      recentWorkspacePaths,
      pinnedWorkspacePaths,
    };
  }, replaceFile);
}

export function pinWorkspacePath(
  userDataPath: string,
  workspacePath: string,
  replaceFile: WorkspaceHistoryReplacer = replaceWorkspaceHistoryFile,
): AppWorkspaceMemory {
  return updateWorkspaceMemory(userDataPath, (current) => {
    if (current.pinnedWorkspacePaths.includes(workspacePath)) return noChange;
    // Count overflow is distinct from, and checked before, the generic
    // byte-size check below: 50 short paths fit easily in 64 KiB, so the
    // byte check alone would never catch a 51st pin. See ADR 0061.
    if (current.pinnedWorkspacePaths.length >= maximumPinnedPaths) return pinLimitReached;
    const pinnedWorkspacePaths = [workspacePath, ...current.pinnedWorkspacePaths];
    const recentWorkspacePaths = current.recentWorkspacePaths.filter((path) => path !== workspacePath);
    return {
      version: 2,
      revision: current.revision + 1,
      lastWorkspacePath: current.lastWorkspacePath,
      recentWorkspacePaths,
      pinnedWorkspacePaths,
    };
  }, replaceFile);
}

export function unpinWorkspacePath(
  userDataPath: string,
  workspacePath: string,
  replaceFile: WorkspaceHistoryReplacer = replaceWorkspaceHistoryFile,
): AppWorkspaceMemory {
  return updateWorkspaceMemory(userDataPath, (current) => {
    if (!current.pinnedWorkspacePaths.includes(workspacePath)) return noChange;
    const pinnedWorkspacePaths = current.pinnedWorkspacePaths.filter((path) => path !== workspacePath);
    // Re-enters recentWorkspacePaths at the front, as if just opened, rather
    // than being lost.
    const recentWorkspacePaths = boundedPaths(
      workspacePath,
      current.recentWorkspacePaths,
      pinnedWorkspacePaths,
      current.revision + 1,
    );
    if (recentWorkspacePaths === null) return tooLarge;
    return {
      version: 2,
      revision: current.revision + 1,
      lastWorkspacePath: current.lastWorkspacePath,
      recentWorkspacePaths,
      pinnedWorkspacePaths,
    };
  }, replaceFile);
}
