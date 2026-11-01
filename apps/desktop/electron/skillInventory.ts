import { open, realpath, readdir, lstat, stat } from "node:fs/promises";
import { constants } from "node:fs";
import { createHash } from "node:crypto";
import { join, resolve, dirname, isAbsolute, relative, sep } from "node:path";
import * as yaml from "js-yaml";
import type {
  SkillEntry,
  SkillInventory,
  SkillScope,
  SkillTool,
} from "./skillTypes.ts";

export const record = (value: unknown): Record<string, any> =>
  value && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, any>)
    : {};
export async function boundedBytes(
  path: string,
  limit = 65536,
): Promise<Buffer> {
  const handle = await open(
    path,
    constants.O_RDONLY | constants.O_NOFOLLOW | constants.O_NONBLOCK,
  );
  try {
    const info = await handle.stat();
    if (!info.isFile() || info.size > limit)
      throw new Error(
        "Expected a bounded regular file (maximum " + limit + " bytes).",
      );
    const buffer = Buffer.alloc(limit + 1);
    const { bytesRead } = await handle.read(buffer, 0, buffer.length, 0);
    if (bytesRead > limit) throw new Error("File exceeds size limit.");
    return buffer.subarray(0, bytesRead);
  } finally {
    await handle.close();
  }
}
export async function boundedText(
  path: string,
  limit = 65536,
): Promise<string> {
  return (await boundedBytes(path, limit)).toString("utf8");
}
// Inventory only needs a bounded prefix; a long instruction body is not invalid metadata.
async function boundedSkillHeader(path: string): Promise<string> {
  const handle = await open(path, constants.O_RDONLY | constants.O_NOFOLLOW | constants.O_NONBLOCK);
  try {
    const info = await handle.stat();
    if (!info.isFile()) throw new Error("Expected a regular skill file.");
    const buffer = Buffer.alloc(Math.min(info.size, 65536));
    let total = 0;
    while (total < buffer.length) {
      const { bytesRead } = await handle.read(buffer, total, buffer.length - total, total);
      if (!bytesRead) break;
      total += bytesRead;
    }
    return buffer.subarray(0, total).toString("utf8");
  } finally {
    await handle.close();
  }
}
export function skillMetadata(text: string): {
  name: string;
  description: string;
} {
  const match = /^---\r?\n([\s\S]*?)\r?\n---(?:\r?\n|$)/.exec(text);
  if (!match) throw new Error("Missing YAML frontmatter.");
  const metadata = record(yaml.load(match[1]));
  if (
    typeof metadata.name !== "string" ||
    !metadata.name.trim() ||
    metadata.name.length > 256 ||
    typeof metadata.description !== "string" ||
    !metadata.description.trim() ||
    metadata.description.length > 8192
  ) {
    throw new Error("Skill needs a bounded name and description.");
  }
  return { name: metadata.name, description: metadata.description };
}
const missing = (error: unknown) => record(error).code === "ENOENT";
// Cached packages are filesystem evidence, not proof of activation or version selection.
async function codexPluginRoots(codexHome: string, diagnostics: string[]) {
  const roots: Array<{ path: string; plugin: string; version?: string }> = [];
  let directoriesLeft = 256;
  const directories = async (path: string) => {
    try {
      const children = (await readdir(path, { withFileTypes: true }))
        .filter(child => child.isDirectory() && !child.name.startsWith("."))
        .sort((a, b) => a.name.localeCompare(b.name));
      if (children.length > directoriesLeft && !diagnostics.includes("Codex plugin cache discovery limit reached."))
        diagnostics.push("Codex plugin cache discovery limit reached.");
      const selected = children.slice(0, directoriesLeft);
      directoriesLeft -= selected.length;
      return selected;
    } catch (error) {
      if (!missing(error)) diagnostics.push(`${path}: Codex plugin cache directory is unreadable.`);
      return [];
    }
  };
  const cache = join(codexHome, "plugins/cache");
  for (const marketplace of await directories(cache)) {
    const marketPath = join(cache, marketplace.name);
    for (const plugin of await directories(marketPath)) {
      const pluginPath = join(marketPath, plugin.name);
      for (const version of await directories(pluginPath)) {
        const packagePath = join(pluginPath, version.name);
        try {
          let manifest: Record<string, any>;
          let portable = true;
          try {
            manifest = record(JSON.parse(await boundedText(join(packagePath, "plugin.json"))));
          } catch (error) {
            if (!missing(error)) throw error;
            portable = false;
            manifest = record(JSON.parse(await boundedText(join(packagePath, ".codex-plugin/plugin.json"))));
          }
          if (manifest.name !== plugin.name) throw new Error("Plugin identity does not match cache directory.");
          const source = portable ? "./skills" : manifest.skills;
          if (source === undefined) continue; // A plugin can contain only tools.
          if (typeof source !== "string" || !source.startsWith("./"))
            throw new Error("Unsupported skill directory; expected a package-relative path.");
          const path = resolve(packagePath, source);
          const contained = (base: string, target: string) => {
            const child = relative(base, target);
            return !isAbsolute(child) && child !== ".." && !child.startsWith(".." + sep);
          };
          if (!contained(packagePath, path)) throw new Error("Skill directory escapes its plugin package.");
          try {
            if (!contained(await realpath(packagePath), await realpath(path)))
              throw new Error("Skill directory resolves outside its plugin package.");
          } catch (error) {
            if (missing(error)) continue;
            throw error;
          }
          roots.push({ path, plugin: `${plugin.name}@${marketplace.name}`, version: version.name });
        } catch (error) {
          diagnostics.push(`${packagePath}: Codex plugin skills could not be inspected (${error instanceof Error ? error.message : "invalid manifest"}).`);
        }
      }
    }
  }
  if (roots.length)
    diagnostics.push("Codex cached plugin skills are found files; enabled state and active version are not verified. Multiple cached versions remain separate.");
  return roots;
}
export async function discoverSkills(options: {
  workspace: string | null;
  home: string;
  env: NodeJS.ProcessEnv;
}): Promise<SkillInventory> {
  const { workspace, home, env } = options;
  const result: SkillInventory = {
    workspace,
    skills: [],
    diagnostics: [],
    roots: [],
  };
  const roots: Array<{
    path: string;
    scope: SkillScope;
    tools: SkillTool[];
    plugin?: string;
    version?: string;
  }> = [];
  const add = (
    path: string,
    scope: SkillScope,
    tools: SkillTool[],
    plugin?: string,
    version?: string,
  ) => {
    if (roots.length >= 128) {
      if (!result.diagnostics.includes("Discovery root limit reached."))
        result.diagnostics.push("Discovery root limit reached.");
      return;
    }
    const existing = roots.find(
      (r) => r.path === path && r.scope === scope && r.plugin === plugin,
    );
    if (existing)
      existing.tools = Array.from(new Set([...existing.tools, ...tools]));
    else roots.push({ path, scope, tools, plugin, version });
  };
  const conventional = (base: string, scope: SkillScope) => {
    add(join(base, ".agents/skills"), scope, ["codex", "copilot", "opencode"]);
    add(join(base, ".claude/skills"), scope, [
      "claude",
      "opencode",
      ...(scope === "workspace" ? ["copilot" as const] : []),
    ]);
    add(
      join(base, scope === "personal" ? ".copilot/skills" : ".github/skills"),
      scope,
      ["copilot"],
    );
    add(
      join(
        base,
        scope === "personal" ? ".config/opencode/skills" : ".opencode/skills",
      ),
      scope,
      ["opencode"],
    );
  };
  conventional(home, "personal");
  // Older/local Codex installs are visible without claiming current native discovery.
  const codexHome = env.CODEX_HOME || join(home, ".codex");
  add(join(codexHome, "skills"), "personal", []);
  // This explicit hidden root is materialized by Codex; other hidden trees stay excluded.
  add(join(codexHome, "skills/.system"), "personal", ["codex"]);
  if (env.CLAUDE_CONFIG_DIR)
    add(join(env.CLAUDE_CONFIG_DIR, "skills"), "personal", ["claude"]);
  if (env.XDG_CONFIG_HOME)
    add(join(env.XDG_CONFIG_HOME, "opencode/skills"), "personal", ["opencode"]);
  if (env.COPILOT_HOME)
    add(join(env.COPILOT_HOME, "skills"), "personal", ["copilot"]);
  for (const path of (env.COPILOT_SKILLS_DIRS || "")
    .split(",")
    .filter(Boolean)) {
    if (isAbsolute(path)) add(path, "personal", ["copilot"]);
    else
      result.diagnostics.push(
        "Relative COPILOT_SKILLS_DIRS is not resolved without a native session context.",
      );
  }
  if (workspace) {
    conventional(workspace, "workspace");
    // Copilot explicitly documents inherited parent .github/skills roots.
    let parent = dirname(workspace);
    for (
      let n = 0;
      n < 16 && parent !== dirname(parent) && parent !== home;
      n++, parent = dirname(parent)
    )
      add(join(parent, ".github/skills"), "workspace", ["copilot"]);
    try {
      const config = record(
        JSON.parse(await boundedText(join(workspace, "opencode.json"))),
      );
      const paths = record(config.skills).paths;
      if (Array.isArray(paths))
        for (const path of paths.slice(0, 32))
          if (typeof path === "string")
            add(resolve(workspace, path), "workspace", ["opencode"]);
    } catch (error) {
      if (!missing(error))
        result.diagnostics.push(
          "OpenCode configuration could not be read as bounded JSON; configured skill coverage is incomplete.",
        );
    }
  }
  try {
    const config = record(
      JSON.parse(
        await boundedText(
          join(
            env.CLAUDE_CONFIG_DIR || join(home, ".claude"),
            "plugins/installed_plugins.json",
          ),
        ),
      ),
    );
    for (const [plugin, registrations] of Object.entries(record(config.plugins)))
      if (Array.isArray(registrations))
        for (const entry of registrations) {
          const registration = record(entry);
          if (
            typeof registration.installPath !== "string" ||
            !isAbsolute(registration.installPath)
          )
            continue;
          if (registration.scope === "user")
            add(
              join(registration.installPath, "skills"),
              "personal",
              ["claude"],
              plugin,
            );
          if (
            workspace &&
            ["project", "local"].includes(registration.scope) &&
            registration.projectPath === workspace
          )
            add(
              join(registration.installPath, "skills"),
              "workspace",
              ["claude"],
              plugin,
            );
        }
  } catch (error) {
    if (!missing(error))
      result.diagnostics.push(
        "Claude plugin registrations could not be inspected.",
      );
  }
  for (const root of await codexPluginRoots(codexHome, result.diagnostics))
    add(root.path, "personal", ["codex"], root.plugin, root.version);
  result.diagnostics.push(
    "Known filesystem roots only. Native loading, precedence, disabled skills, session overrides, enterprise/remote skills and unregistered plugin roots are not verified.",
  );
  result.diagnostics.push(
    "OpenCode JSONC/global custom paths and the Copilot plugin registry are not yet covered. Legacy Codex roots have unknown tool discovery.",
  );
  const entries = new Map<string, SkillEntry>();
  let directoriesLeft = 4000,
    metadataBytes = 0;
  for (const root of roots) {
    if (directoriesLeft <= 0 || metadataBytes >= 16 * 1024 * 1024) {
      result.diagnostics.push(
        "Inventory budget reached; remaining roots were not inspected.",
      );
      break;
    }
    result.roots.push({
      path: root.path,
      scope: root.scope,
      tools: root.tools,
    });
    const seen = new Set<string>();
    let remaining = 2000;
    const walk = async (path: string, depth: number): Promise<void> => {
      if (--directoriesLeft < 0 || --remaining < 0 || depth > 8) {
        if (remaining === -1 || depth > 8)
          result.diagnostics.push(`${root.path}: discovery limit reached.`);
        return;
      }
      let canonical: string;
      try {
        canonical = await realpath(path);
        if (!(await stat(canonical)).isDirectory()) return;
      } catch (error) {
        if (!missing(error))
          result.diagnostics.push(`${path}: directory is unreadable.`);
        return;
      }
      if (seen.has(canonical)) {
        result.diagnostics.push(`${path}: cycle or repeated alias skipped.`);
        return;
      }
      seen.add(canonical);
      try {
        const text = await boundedSkillHeader(join(canonical, "SKILL.md"));
        metadataBytes += Buffer.byteLength(text);
        if (metadataBytes > 16 * 1024 * 1024)
          throw new Error("Inventory metadata budget reached.");
        const metadata = skillMetadata(text);
        const key = root.scope + "\0" + canonical;
        const existing = entries.get(key);
        if (existing) {
          existing.paths = Array.from(new Set([...existing.paths, path]));
          if (root.plugin) {
            existing.plugins = Array.from(
              new Set([...(existing.plugins || []), root.plugin]),
            ).sort();
            existing.management = "plugin";
            if (root.version) existing.version = root.version;
          }
          existing.tools = Array.from(
            new Set([...existing.tools, ...root.tools]),
          );
        } else
          entries.set(key, {
            id: createHash("sha256").update(key).digest("hex"),
            ...metadata,
            scope: root.scope,
            paths: [path],
            resolvedPath: canonical,
            tools: [...root.tools],
            management: root.plugin ? "plugin" : "local",
            ...(root.plugin ? { plugins: [root.plugin] } : {}),
            ...(root.version ? { version: root.version } : {}),
          });
        return;
      } catch (error) {
        if (!missing(error)) {
          result.diagnostics.push(`${path}: invalid or unreadable SKILL.md.`);
          return;
        }
      }
      try {
        const children = await readdir(canonical, { withFileTypes: true });
        for (const child of children.sort((a, b) =>
          a.name.localeCompare(b.name),
        )) {
          if (remaining < 0) break;
          if (
            !child.name.startsWith(".") &&
            (child.isDirectory() || child.isSymbolicLink())
          )
            await walk(join(path, child.name), depth + 1);
        }
      } catch {
        result.diagnostics.push(`${path}: directory entries are unreadable.`);
      }
    };
    await walk(root.path, 0);
  }
  for (const scope of ["workspace", "personal"] as const) {
    const base = scope === "workspace" ? workspace : home;
    if (!base) continue;
    try {
      const lock = record(
        yaml.load(
          await boundedText(
            join(
              base,
              scope === "personal" ? ".apm/apm.lock.yaml" : "apm.lock.yaml",
            ),
            // Deployment hashes make ordinary lock files much larger than skill metadata.
            2 * 1024 * 1024,
          ),
        ),
      );
      for (const dependency of Array.isArray(lock.dependencies)
        ? lock.dependencies
        : []) {
        const entry = record(dependency);
        for (const file of Array.isArray(entry.deployed_files)
          ? entry.deployed_files
          : []) {
          if (typeof file !== "string" || !file.endsWith("/SKILL.md")) continue;
          const path = resolve(base, file);
          const match = [...entries.values()].find(
            (skill) =>
              skill.scope === scope &&
              (join(skill.resolvedPath, "SKILL.md") === path ||
                skill.paths.some(alias => join(alias, "SKILL.md") === path)),
          );
          if (match) {
            match.management = "apm";
            if (typeof entry.local_path === "string")
              match.packageSource = entry.local_path;
            else if (typeof entry.repo_url === "string")
              match.packageSource = entry.repo_url;
            if (typeof entry.version === "string")
              match.version = entry.version;
          }
        }
      }
    } catch (error) {
      if (!missing(error))
        result.diagnostics.push(
          `${scope}: APM lock provenance could not be read.`,
        );
    }
  }
  result.skills = [...entries.values()].sort(
    (a, b) => a.name.localeCompare(b.name) || a.scope.localeCompare(b.scope),
  );
  return result;
}
export async function readSkillDetail(
  inventory: SkillInventory,
  id: unknown,
): Promise<string> {
  const skill = inventory.skills.find((entry) => entry.id === id);
  if (!skill)
    throw new Error("Unknown skill or inventory no longer available.");
  if ((await realpath(skill.resolvedPath)) !== skill.resolvedPath)
    throw new Error("Skill directory changed. Refresh the inventory.");
  const file = join(skill.resolvedPath, "SKILL.md");
  if ((await lstat(file)).isSymbolicLink())
    throw new Error(
      "Skill file changed to a symbolic link. Refresh the inventory.",
    );
  return boundedText(file);
}
