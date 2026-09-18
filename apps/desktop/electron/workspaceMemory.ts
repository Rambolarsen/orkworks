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
  code: "corrupt_history" | "history_lock_timeout" | "history_write_failed";
  message: string;
}

export interface AppWorkspaceMemory {
  version: 1;
  revision: number;
  lastWorkspacePath: string | null;
  recentWorkspacePaths: string[];
  diagnostic: WorkspaceMemoryDiagnostic | null;
}

interface StoredWorkspaceMemory {
  version: 1;
  revision: number;
  lastWorkspacePath: string | null;
  recentWorkspacePaths: string[];
}

const fileName = "workspace-memory.json";
const lockFileName = ".workspace-memory.lock";
const maximumRecentPaths = 20;
const maximumSerializedBytes = 64 * 1024;
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
  version: 1,
  revision: 0,
  lastWorkspacePath: null,
  recentWorkspacePaths: [],
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
    version: 1,
    revision: memory.revision,
    lastWorkspacePath: memory.lastWorkspacePath,
    recentWorkspacePaths: memory.recentWorkspacePaths,
  };
}

function serializedMemory(memory: StoredWorkspaceMemory): string {
  return `${JSON.stringify(memory, null, 2)}\n`;
}

function memoryFits(memory: StoredWorkspaceMemory): boolean {
  return Buffer.byteLength(serializedMemory(memory), "utf8") <= maximumSerializedBytes;
}

function boundedPaths(
  lastWorkspacePath: string | null,
  paths: readonly string[],
  revision: number,
): string[] | null {
  const deduplicated = [
    ...(lastWorkspacePath === null ? [] : [lastWorkspacePath]),
    ...paths,
  ].filter((value, index, values) => values.indexOf(value) === index);
  const bounded = deduplicated.slice(0, maximumRecentPaths);
  const candidate = {
    version: 1 as const,
    revision,
    lastWorkspacePath,
    recentWorkspacePaths: bounded,
  };

  while (bounded.length > 1 && !memoryFits({ ...candidate, recentWorkspacePaths: bounded })) {
    bounded.pop();
  }
  return memoryFits({ ...candidate, recentWorkspacePaths: bounded }) ? bounded : null;
}

interface LegacyStoredWorkspaceMemory {
  lastWorkspacePath: string | null;
  recentWorkspacePaths: string[];
}

// Pre-#569 files predate the version/revision fields and the invariants
// validStoredMemory enforces (deduplication, lastWorkspacePath inclusion).
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
    && raw.recentWorkspacePaths.every((entry) => typeof entry === "string");
}

function migratedLegacyMemory(legacy: LegacyStoredWorkspaceMemory): AppWorkspaceMemory {
  const bounded = boundedPaths(legacy.lastWorkspacePath, legacy.recentWorkspacePaths, 0);
  // boundedPaths only returns null when even a single entry can't fit under
  // the serialized-size bound. Silently dropping recentWorkspacePaths would
  // produce a lastWorkspacePath not present in the list, violating the same
  // invariant validStoredMemory enforces for the current format — surface a
  // diagnostic instead, matching how every other "doesn't fit" case here
  // fails loud rather than discarding data quietly.
  if (bounded === null) return withDiagnostic(emptyMemory(), corruptDiagnostic);
  return {
    version: 1,
    revision: 0,
    lastWorkspacePath: legacy.lastWorkspacePath,
    recentWorkspacePaths: bounded,
    diagnostic: null,
  };
}

function validStoredMemory(value: unknown): value is StoredWorkspaceMemory {
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

function readStoredWorkspaceMemory(userDataPath: string): AppWorkspaceMemory {
  const target = workspaceMemoryPath(userDataPath);
  if (!existsSync(target)) return emptyMemory();

  try {
    const source = readFileSync(target);
    if (source.byteLength > maximumSerializedBytes) return withDiagnostic(emptyMemory(), corruptDiagnostic);
    const parsed: unknown = JSON.parse(utf8Decoder.decode(source));
    if (!validStoredMemory(parsed)) {
      if (validLegacyStoredMemory(parsed)) return migratedLegacyMemory(parsed);
      return withDiagnostic(emptyMemory(), corruptDiagnostic);
    }
    if (!memoryFits(parsed)) return withDiagnostic(emptyMemory(), corruptDiagnostic);
    return {
      version: 1,
      revision: parsed.revision,
      lastWorkspacePath: parsed.lastWorkspacePath,
      recentWorkspacePaths: [...parsed.recentWorkspacePaths],
      diagnostic: null,
    };
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
type UpdateResult = StoredWorkspaceMemory | typeof noChange | typeof tooLarge;

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
    const recentWorkspacePaths = boundedPaths(workspacePath, [
      workspacePath,
      ...current.recentWorkspacePaths.filter((path) => path !== workspacePath),
    ], current.revision + 1);
    if (recentWorkspacePaths === null) return tooLarge;
    return {
      version: 1,
      revision: current.revision + 1,
      lastWorkspacePath: workspacePath,
      recentWorkspacePaths,
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
    if (recentWorkspacePaths.length === current.recentWorkspacePaths.length && current.lastWorkspacePath !== workspacePath) {
      return noChange;
    }
    return {
      version: 1,
      revision: current.revision + 1,
      lastWorkspacePath: current.lastWorkspacePath === workspacePath ? null : current.lastWorkspacePath,
      recentWorkspacePaths,
    };
  }, replaceFile);
}
