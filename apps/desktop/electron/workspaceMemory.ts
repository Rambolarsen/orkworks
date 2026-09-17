import { randomBytes } from "node:crypto";
import {
  closeSync,
  existsSync,
  fsyncSync,
  mkdirSync,
  openSync,
  readFileSync,
  renameSync,
  rmSync,
  writeSync,
} from "node:fs";
import { join } from "node:path";

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
const lockDirectoryName = ".workspace-memory.lock";
const maximumRecentPaths = 20;
const maximumSerializedBytes = 64 * 1024;
const lockRetryCount = 50;
const lockRetryDelayMs = 10;

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
): string[] | null {
  const deduplicated = [
    ...(lastWorkspacePath === null ? [] : [lastWorkspacePath]),
    ...paths,
  ].filter((value, index, values) => values.indexOf(value) === index);
  const bounded = deduplicated.slice(0, maximumRecentPaths);
  const candidate = {
    version: 1 as const,
    revision: 0,
    lastWorkspacePath,
    recentWorkspacePaths: bounded,
  };

  while (bounded.length > 1 && !memoryFits({ ...candidate, recentWorkspacePaths: bounded })) {
    bounded.pop();
  }
  return memoryFits({ ...candidate, recentWorkspacePaths: bounded }) ? bounded : null;
}

function validStoredMemory(value: unknown): value is StoredWorkspaceMemory {
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  const raw = value as Record<string, unknown>;
  return raw.version === 1
    && typeof raw.revision === "number"
    && Number.isSafeInteger(raw.revision)
    && raw.revision >= 0
    && (raw.lastWorkspacePath === null || typeof raw.lastWorkspacePath === "string")
    && Array.isArray(raw.recentWorkspacePaths)
    && raw.recentWorkspacePaths.every((entry) => typeof entry === "string");
}

function readStoredWorkspaceMemory(userDataPath: string): AppWorkspaceMemory {
  const target = workspaceMemoryPath(userDataPath);
  if (!existsSync(target)) return emptyMemory();

  try {
    const parsed: unknown = JSON.parse(readFileSync(target, "utf8"));
    if (!validStoredMemory(parsed)) return withDiagnostic(emptyMemory(), corruptDiagnostic);
    const recentWorkspacePaths = boundedPaths(parsed.lastWorkspacePath, parsed.recentWorkspacePaths);
    if (recentWorkspacePaths === null) return withDiagnostic(emptyMemory(), corruptDiagnostic);
    return {
      version: 1,
      revision: parsed.revision,
      lastWorkspacePath: parsed.lastWorkspacePath,
      recentWorkspacePaths,
      diagnostic: null,
    };
  } catch {
    return withDiagnostic(emptyMemory(), corruptDiagnostic);
  }
}

function sleepForLockRetry(): void {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, lockRetryDelayMs);
}

function acquireHistoryLock(userDataPath: string): string | null {
  const lockPath = join(userDataPath, lockDirectoryName);
  for (let attempt = 0; attempt < lockRetryCount; attempt += 1) {
    try {
      mkdirSync(lockPath);
      return lockPath;
    } catch (error) {
      const code = error && typeof error === "object" && "code" in error
        ? (error as { code?: string }).code
        : undefined;
      if (code !== "EEXIST") return null;
      sleepForLockRetry();
    }
  }
  return null;
}

function releaseHistoryLock(lockPath: string): void {
  rmSync(lockPath, { recursive: true, force: true });
}

function writeAndVerify(
  userDataPath: string,
  current: AppWorkspaceMemory,
  next: StoredWorkspaceMemory,
): AppWorkspaceMemory {
  const target = workspaceMemoryPath(userDataPath);
  const temporary = join(
    userDataPath,
    `.workspace-memory.${process.pid}.${randomBytes(8).toString("hex")}.tmp`,
  );
  let descriptor: number | null = null;
  try {
    descriptor = openSync(temporary, "wx", 0o600);
    const bytes = Buffer.from(serializedMemory(next), "utf8");
    writeSync(descriptor, bytes, 0, bytes.byteLength, 0);
    fsyncSync(descriptor);
    closeSync(descriptor);
    descriptor = null;
    renameSync(temporary, target);

    const observed = readStoredWorkspaceMemory(userDataPath);
    if (
      observed.diagnostic === null
      && JSON.stringify(storedMemory(observed)) === JSON.stringify(next)
    ) {
      return observed;
    }
    return withDiagnostic(current, {
      code: "history_write_failed",
      message: "Workspace history could not be confirmed after replacement.",
    });
  } catch {
    return withDiagnostic(current, {
      code: "history_write_failed",
      message: "Workspace history could not be saved; the ready workspace was kept.",
    });
  } finally {
    if (descriptor !== null) closeSync(descriptor);
    rmSync(temporary, { force: true });
  }
}

function updateWorkspaceMemory(
  userDataPath: string,
  update: (current: AppWorkspaceMemory) => StoredWorkspaceMemory | null,
): AppWorkspaceMemory {
  try {
    mkdirSync(userDataPath, { recursive: true });
  } catch {
    return withDiagnostic(emptyMemory(), {
      code: "history_write_failed",
      message: "Workspace history could not be saved; the ready workspace was kept.",
    });
  }

  const lockPath = acquireHistoryLock(userDataPath);
  if (lockPath === null) {
    return withDiagnostic(readStoredWorkspaceMemory(userDataPath), {
      code: "history_lock_timeout",
      message: "Workspace history was busy and could not be updated.",
    });
  }

  try {
    const current = readStoredWorkspaceMemory(userDataPath);
    if (current.diagnostic !== null) return current;
    const next = update(current);
    if (next === null) return current;
    if (!memoryFits(next)) {
      return withDiagnostic(current, {
        code: "history_write_failed",
        message: "Workspace path is too large to fit in the bounded history file.",
      });
    }
    return writeAndVerify(userDataPath, current, next);
  } finally {
    releaseHistoryLock(lockPath);
  }
}

export function workspaceMemoryPath(userDataPath: string): string {
  return join(userDataPath, fileName);
}

export function readWorkspaceMemory(userDataPath: string): AppWorkspaceMemory {
  return readStoredWorkspaceMemory(userDataPath);
}

export function rememberWorkspacePath(userDataPath: string, workspacePath: string): AppWorkspaceMemory {
  return updateWorkspaceMemory(userDataPath, (current) => {
    const recentWorkspacePaths = boundedPaths(workspacePath, [
      workspacePath,
      ...current.recentWorkspacePaths.filter((path) => path !== workspacePath),
    ]);
    if (recentWorkspacePaths === null) return null;
    return {
      version: 1,
      revision: current.revision + 1,
      lastWorkspacePath: workspacePath,
      recentWorkspacePaths,
    };
  });
}

export function forgetWorkspacePath(userDataPath: string, workspacePath: string): AppWorkspaceMemory {
  return updateWorkspaceMemory(userDataPath, (current) => {
    const recentWorkspacePaths = current.recentWorkspacePaths.filter((path) => path !== workspacePath);
    if (recentWorkspacePaths.length === current.recentWorkspacePaths.length && current.lastWorkspacePath !== workspacePath) {
      return null;
    }
    return {
      version: 1,
      revision: current.revision + 1,
      lastWorkspacePath: current.lastWorkspacePath === workspacePath ? null : current.lastWorkspacePath,
      recentWorkspacePaths,
    };
  });
}
