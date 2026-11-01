// Intentionally duplicated across the Electron/renderer boundary.
export type SkillScope = "workspace" | "personal";
export type SkillTool = "codex" | "claude" | "copilot" | "opencode";
export interface SkillEntry {
  id: string;
  name: string;
  description: string;
  scope: SkillScope;
  paths: string[];
  resolvedPath: string;
  tools: SkillTool[];
  management: "local" | "apm" | "plugin";
  plugins?: string[];
  packageSource?: string;
  apmIntegrity?: { status: "matches" | "differs" | "unavailable"; reason?: string };
  version?: string;
}
export interface ApmStatus {
  available: boolean;
  compatible: boolean;
  path: string | null;
  version: string | null;
  executableOverride: string | null;
  message: string;
}
export interface SkillInventory {
  workspace: string | null;
  skills: SkillEntry[];
  diagnostics: string[];
  apm?: ApmStatus;
  roots: Array<{ path: string; scope: SkillScope; tools: SkillTool[] }>;
}
export interface SkillAddRequest {
  scope: SkillScope;
  source: string;
  target: "codex";
}
export interface SkillPreview {
  id: string | null;
  scope: SkillScope;
  destination: string;
  source: string;
  target: "codex";
  available: boolean;
  summary: string;
  paths: string[];
  command: string;
  reason?: string;
}
export interface SkillInstallResult {
  ok: boolean;
  message: string;
}
