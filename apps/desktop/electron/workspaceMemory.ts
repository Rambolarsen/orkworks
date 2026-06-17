import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

export interface AppWorkspaceMemory {
  lastWorkspacePath: string | null;
  recentWorkspacePaths: string[];
}

const fileName = "workspace-memory.json";

export function workspaceMemoryPath(userDataPath: string): string {
  return join(userDataPath, fileName);
}

export function readWorkspaceMemory(userDataPath: string): AppWorkspaceMemory {
  const path = workspaceMemoryPath(userDataPath);
  if (!existsSync(path)) {
    return { lastWorkspacePath: null, recentWorkspacePaths: [] };
  }
  try {
    const parsed = JSON.parse(readFileSync(path, "utf8")) as Partial<AppWorkspaceMemory>;
    return {
      lastWorkspacePath: typeof parsed.lastWorkspacePath === "string" ? parsed.lastWorkspacePath : null,
      recentWorkspacePaths: Array.isArray(parsed.recentWorkspacePaths)
        ? parsed.recentWorkspacePaths.filter((item): item is string => typeof item === "string")
        : [],
    };
  } catch {
    return { lastWorkspacePath: null, recentWorkspacePaths: [] };
  }
}

export function writeWorkspaceMemory(userDataPath: string, memory: AppWorkspaceMemory): void {
  mkdirSync(userDataPath, { recursive: true });
  writeFileSync(workspaceMemoryPath(userDataPath), JSON.stringify(memory, null, 2));
}

export function rememberWorkspacePath(userDataPath: string, workspacePath: string): AppWorkspaceMemory {
  const current = readWorkspaceMemory(userDataPath);
  const recentWorkspacePaths = [
    workspacePath,
    ...current.recentWorkspacePaths.filter((path) => path !== workspacePath),
  ].slice(0, 10);
  const next = { lastWorkspacePath: workspacePath, recentWorkspacePaths };
  writeWorkspaceMemory(userDataPath, next);
  return next;
}
