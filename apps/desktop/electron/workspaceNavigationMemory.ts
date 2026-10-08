import { isAbsolute, normalize, join } from "node:path";
import { RevisionedShellMemory, type ShellFileReplacer, type ShellMemoryDiagnostic, type ShellMemoryResult } from "./shellLayoutMemory.ts";
import { canonicalWorkspacePath, forgetWorkspacePath, readWorkspaceMemory, type AppWorkspaceMemory } from "./workspaceMemory.ts";

export type LastCentralSurface = "terminal" | "review";
export type WorkspaceNavigationEntry = { workspaceIdentity: string; lastCentralSurface: LastCentralSurface };
export type WorkspaceNavigationSnapshot = { entries: WorkspaceNavigationEntry[]; revision: number; diagnostic: ShellMemoryDiagnostic | null };
type NavigationPayload = { entries: WorkspaceNavigationEntry[] };

function exactKeys(value: unknown, keys: string[]): value is Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value)
    && JSON.stringify(Object.keys(value).sort()) === JSON.stringify([...keys].sort());
}

function validCanonicalKey(value: unknown): value is string {
  return typeof value === "string" && value.length > 0 && !value.includes("\0")
    && isAbsolute(value) && normalize(value) === value;
}

function validEntry(value: unknown): value is WorkspaceNavigationEntry {
  return exactKeys(value, ["workspaceIdentity", "lastCentralSurface"])
    && validCanonicalKey(value.workspaceIdentity)
    && (value.lastCentralSurface === "terminal" || value.lastCentralSurface === "review");
}

function validPayload(value: unknown): value is NavigationPayload {
  if (!exactKeys(value, ["entries"]) || !Array.isArray(value.entries) || value.entries.length > 20
    || !value.entries.every(validEntry)) return false;
  return new Set(value.entries.map((entry: WorkspaceNavigationEntry) => entry.workspaceIdentity)).size === value.entries.length;
}

export function workspaceNavigationMemoryPath(directory: string): string { return join(directory, "workspace-navigation.json"); }

export function createWorkspaceNavigationMemory(directory: string, replacer?: ShellFileReplacer, restoreReplacer?: ShellFileReplacer) {
  const memory = new RevisionedShellMemory(directory, "workspace-navigation.json", 64 * 1024,
    (): NavigationPayload => ({ entries: [] }), validPayload, replacer, restoreReplacer);
  return {
    read: (): WorkspaceNavigationSnapshot => {
      const loaded = memory.readRecord();
      return { entries: loaded.record?.payload.entries.map((entry) => ({ ...entry })) ?? [],
        revision: loaded.record?.revision ?? 0, diagnostic: loaded.diagnostic };
    },
    complete: (workspaceIdentity: string, generation: number, stillCurrent: () => boolean, surface: LastCentralSurface): Promise<ShellMemoryResult> => {
      if (!validCanonicalKey(workspaceIdentity) || !Number.isSafeInteger(generation) || generation < 0
        || (surface !== "terminal" && surface !== "review")) return Promise.resolve({ ok: false, diagnostic: "invalid_input" });
      return memory.enqueue(workspaceIdentity, stillCurrent, (payload, revision) => {
        const entries = [{ workspaceIdentity, lastCentralSurface: surface },
          ...payload.entries.filter((entry) => entry.workspaceIdentity !== workspaceIdentity)];
        while (entries.length > 20 || Buffer.byteLength(JSON.stringify({ version: 1, epoch: "0".repeat(32), revision,
          payload: { entries } }) + "\n", "utf8") > 64 * 1024) {
          if (entries.length === 1) return null;
          entries.pop();
        }
        return { entries };
      });
    },
    delete: (workspaceIdentity: string, generation: number, stillCurrent: () => boolean): Promise<ShellMemoryResult> => {
      if (!validCanonicalKey(workspaceIdentity) || !Number.isSafeInteger(generation) || generation < 0)
        return Promise.resolve({ ok: false, diagnostic: "invalid_input" });
      return memory.enqueue(workspaceIdentity, stillCurrent,
        (payload) => ({ entries: payload.entries.filter((entry) => entry.workspaceIdentity !== workspaceIdentity) }), true);
    },
    rebuild: (confirmed: true): Promise<ShellMemoryResult> => memory.rebuild(confirmed),
  };
}

function historyContains(memory: AppWorkspaceMemory, identity: string): boolean {
  return memory.lastWorkspacePath === identity
    || memory.recentWorkspacePaths.includes(identity)
    || memory.pinnedWorkspacePaths.includes(identity);
}

export async function forgetRememberedWorkspaceWithNavigation(
  directory: string,
  identity: string,
  navigation: ReturnType<typeof createWorkspaceNavigationMemory>,
): Promise<{ history: AppWorkspaceMemory; navigation: ShellMemoryResult | null }> {
  const before = readWorkspaceMemory(directory);
  // Existing shortcuts are the authority for a path that has disappeared.
  // An existing alias must resolve to itself; renderer text alone is never a
  // workspace identity and must not delete an unrelated navigation entry.
  if (before.diagnostic || !historyContains(before, identity) || !validCanonicalKey(identity)) {
    return { history: before, navigation: null };
  }
  const canonical = canonicalWorkspacePath(identity);
  if (canonical !== null && canonical !== identity) return { history: before, navigation: null };

  const history = forgetWorkspacePath(directory, identity);
  if (history.diagnostic || historyContains(history, identity)) return { history, navigation: null };
  return { history, navigation: await navigation.delete(identity, 0, () => true) };
}
