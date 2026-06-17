import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const fileName = "layout.json";

function layoutMemoryPath(userDataPath: string): string {
  return join(userDataPath, fileName);
}

export function readLayoutMemory(userDataPath: string): string | null {
  const path = layoutMemoryPath(userDataPath);
  if (!existsSync(path)) {
    return null;
  }
  try {
    const raw = readFileSync(path, "utf8");
    JSON.parse(raw);
    return raw;
  } catch {
    console.warn("[layoutMemory] corrupt layout.json, ignoring");
    return null;
  }
}

export function writeLayoutMemory(userDataPath: string, json: string): void {
  if (!json || json.length === 0) return;
  mkdirSync(userDataPath, { recursive: true });
  writeFileSync(layoutMemoryPath(userDataPath), json);
}
