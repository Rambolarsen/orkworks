import { execFile } from "node:child_process";
import { createHash, randomUUID } from "node:crypto";
import {
  access,
  lstat,
  readdir,
  realpath,
  mkdtemp,
  writeFile,
  rm,
} from "node:fs/promises";
import { constants } from "node:fs";
import { join, delimiter, isAbsolute, basename, resolve } from "node:path";
import { tmpdir } from "node:os";
import * as yaml from "js-yaml";
import {
  boundedBytes,
  boundedText,
  record,
  skillMetadata,
} from "./skillInventory.ts";
import type {
  SkillAddRequest,
  SkillPreview,
  SkillInstallResult,
} from "./skillTypes.ts";

export function skillProcessEnvironment(
  input: NodeJS.ProcessEnv,
): NodeJS.ProcessEnv {
  return Object.fromEntries(
    Object.entries(input).filter(
      ([key]) =>
        !key.startsWith("ORKWORKS_") &&
        ![
          "APM_POLICY_DISABLE",
          "APM_NO_SCRIPTS",
          "APM_ALLOW_PROTOCOL_FALLBACK",
          "APM_LEGACY_SKILL_PATHS",
        ].includes(key),
    ),
  );
}
export async function findApm(env: NodeJS.ProcessEnv): Promise<string> {
  for (const directory of (env.PATH || "").split(delimiter)) {
    if (!isAbsolute(directory)) continue;
    const candidate = join(
      directory,
      process.platform === "win32" ? "apm.exe" : "apm",
    );
    try {
      await access(candidate, constants.X_OK);
      return await realpath(candidate);
    } catch {
      /* next PATH entry */
    }
  }
  throw new Error(
    "APM is not installed on the desktop PATH. Install APM separately, then Refresh.",
  );
}
export function runApm(
  file: string,
  args: string[],
  cwd: string,
  env: NodeJS.ProcessEnv,
): Promise<string> {
  return new Promise((resolve, reject) =>
    execFile(
      file,
      args,
      { cwd, env, timeout: 60000, maxBuffer: 256 * 1024, windowsHide: true },
      (error, stdout) => {
        if (error)
          reject(
            new Error(
              "APM failed or exceeded its time/output limit. Inspect the destination for partial changes before trying again.",
            ),
          );
        else resolve(stdout);
      },
    ),
  );
}
const absent = (error: unknown) => record(error).code === "ENOENT";
async function optionalText(path: string): Promise<string> {
  try {
    return await boundedText(path);
  } catch (error) {
    if (absent(error)) return "";
    throw error;
  }
}
// Fingerprint bounded trees without following links, including executable identity
// (stat), source bytes and existing deployment/configuration bytes.
async function treeDigest(paths: string[]): Promise<string> {
  const digest = createHash("sha256");
  let count = 0,
    bytes = 0;
  const visit = async (path: string, depth: number): Promise<void> => {
    if (++count > 4000 || depth > 10)
      throw new Error(
        "Review exceeds the file/depth limit. Use APM in a terminal.",
      );
    digest.update(path + "\0");
    let info;
    try {
      info = await lstat(path);
    } catch (error) {
      if (absent(error)) {
        digest.update("absent");
        return;
      }
      throw error;
    }
    if (info.isSymbolicLink())
      throw new Error(
        "A reviewed source or destination uses a symbolic link. Use APM in a terminal.",
      );
    if (info.isDirectory())
      for (const name of (await readdir(path)).sort())
        await visit(join(path, name), depth + 1);
    else {
      if (!info.isFile() || info.nlink > 1)
        throw new Error("Review requires ordinary files without hard links.");
      bytes += info.size;
      if (bytes > 16 * 1024 * 1024)
        throw new Error(
          "Review exceeds the 16 MiB limit. Use APM in a terminal.",
        );
      digest.update(await boundedBytes(path, 1024 * 1024));
    }
  };
  for (const path of paths) await visit(path, 0);
  return digest.digest("hex");
}
async function pureSkill(source: string): Promise<void> {
  if (!isAbsolute(source) || (await realpath(source)) !== source)
    throw new Error("Select a canonical local skill directory.");
  skillMetadata(await boundedText(join(source, "SKILL.md")));
  const allowed = new Set([
    "SKILL.md",
    "README.md",
    "LICENSE",
    "LICENSE.md",
    "scripts",
    "references",
    "assets",
  ]);
  for (const name of await readdir(source))
    if (!allowed.has(name))
      throw new Error(
        "This package contains metadata or primitives beyond the qualified single-skill profile. Use APM in a terminal.",
      );
}
function validateRequest(value: unknown): SkillAddRequest {
  const input = record(value);
  if (
    !["workspace", "personal"].includes(input.scope) ||
    input.target !== "codex" ||
    typeof input.source !== "string" ||
    input.source.length > 2048 ||
    /[\x00-\x1f;`$]/.test(input.source) ||
    input.source.startsWith("-") ||
    /https?:\/\/[^/]*@/.test(input.source)
  )
    throw new Error(
      "Choose a valid scope and a local directory or GitHub package reference without credentials or CLI options.",
    );
  if (
    !isAbsolute(input.source) &&
    !/^(?:https:\/\/github\.com\/)?[A-Za-z0-9_.-]+\/[A-Za-z0-9_.\/-]+(?:#[A-Za-z0-9_.\/-]+)?$/.test(
      input.source,
    )
  )
    throw new Error("Invalid GitHub package reference.");
  return input as SkillAddRequest;
}
const command = (request: SkillAddRequest) =>
  [
    "apm",
    "install",
    request.source,
    "--target",
    "codex",
    "--only",
    "apm",
    "--no-trust-bin",
    ...(request.scope === "personal" ? ["--global"] : []),
  ]
    .map((arg) => "'" + arg.replace(/'/g, "'\"'\"'") + "'")
    .join(" ");
interface Dependencies {
  home: string;
  env: NodeJS.ProcessEnv;
  workspace(): string | null;
  workspaceRevision?(): string;
  executable?(): Promise<string>;
  run?(
    file: string,
    args: string[],
    cwd: string,
    env: NodeJS.ProcessEnv,
  ): Promise<string>;
  confirm(preview: SkillPreview): Promise<boolean>;
}
type Review = {
  view: SkillPreview;
  request: SkillAddRequest;
  executable: string;
  fingerprint: string;
  expires: number;
};
export class SkillInstaller {
  private deps: Dependencies;
  private reviews = new Map<string, Review>();
  private busy = new Set<string>();
  constructor(deps: Dependencies) {
    this.deps = deps;
  }
  private run(file: string, args: string[], cwd: string) {
    return (this.deps.run || runApm)(
      file,
      args,
      cwd,
      skillProcessEnvironment(this.deps.env),
    );
  }
  private async check(
    request: SkillAddRequest,
    destination: string,
    executable: string,
  ): Promise<{ fingerprint: string; manifest: string }> {
    if (request.scope === "workspace" && this.deps.workspace() !== destination)
      throw new Error("Workspace changed. Preview again.");
    if ((await realpath(destination)) !== destination)
      throw new Error(
        "Destination changed or is not canonical. Reopen the workspace.",
      );
    await pureSkill(request.source);
    for (const target of [
      join(destination, ".agents"),
      join(destination, ".agents/skills"),
      join(destination, ".codex"),
      join(destination, ".apm"),
      join(destination, "apm_modules"),
    ]) {
      try {
        if ((await lstat(target)).isSymbolicLink())
          throw new Error(
            "A destination root is a symbolic link. Use APM in a terminal.",
          );
      } catch (error) {
        if (!absent(error)) throw error;
      }
    }
    const metadata =
      request.scope === "personal" ? join(destination, ".apm") : destination;
    const manifest = await optionalText(join(metadata, "apm.yml"));
    const config = record(manifest ? yaml.load(manifest) : {});
    const keys = new Set([
      "name",
      "version",
      "description",
      "author",
      "license",
      "targets",
      "dependencies",
      "includes",
      "scripts",
    ]);
    if (
      (config.includes !== undefined && config.includes !== "auto") ||
      Object.keys(config).some((key) => !keys.has(key)) ||
      Object.keys(record(config.scripts)).length ||
      (config.targets &&
        JSON.stringify(config.targets) !== JSON.stringify(["codex"]))
    )
      throw new Error(
        "Existing manifest has broader configuration or targets. Review it through APM in a terminal.",
      );
    const deps = record(config.dependencies);
    if (
      Object.keys(deps).some((key) => key !== "apm" && key !== "mcp") ||
      (deps.mcp && JSON.stringify(deps.mcp) !== "[]") ||
      (deps.apm && !Array.isArray(deps.apm))
    )
      throw new Error(
        "Existing manifest requires unrelated service/configuration changes.",
      );
    const sources: string[] = [];
    for (const source of deps.apm || []) {
      if (typeof source !== "string" || !isAbsolute(source))
        throw new Error(
          "Existing manifest has unresolved or remote dependencies. Use APM in a terminal.",
        );
      await pureSkill(source);
      sources.push(source);
    }
    if (request.scope === "workspace") {
      try {
        await lstat(join(destination, ".apm"));
        throw new Error(
          "Project-owned .apm primitives require a broader install review.",
        );
      } catch (error) {
        if (!absent(error)) throw error;
      }
    }
    if (request.scope === "personal") {
      for (const primitive of ["skills", "agents", "instructions", "prompts", "hooks", "bin", ".apm"]) {
        try { await lstat(join(metadata, primitive)); throw new Error("Personal APM primitives require a broader terminal review."); }
        catch (error) { if (!absent(error)) throw error; }
      }
    }
    const userManifest = await optionalText(
      join(this.deps.env.APM_HOME || join(this.deps.home, ".apm"), "apm.yml"),
    );
    if (record(userManifest ? yaml.load(userManifest) : {}).lifecycle)
      throw new Error(
        "Personal APM lifecycle commands require a broader terminal review.",
      );
    const policyDirectory =
      process.platform === "win32"
        ? join(this.deps.env.PROGRAMDATA || "C:\\ProgramData", "APM/policy.d")
        : "/etc/apm/policy.d";
    try {
      if (
        (await readdir(policyDirectory)).some((name) => name.endsWith(".json"))
      )
        throw new Error(
          "Administrator APM lifecycle policy requires terminal review.",
        );
    } catch (error) {
      if (!absent(error)) throw error;
    }
    // Every target ancestor is checked too: a symlinked .agents must not redirect writes.
    const fingerprint = await treeDigest([
      ...new Set([
        request.source,
        ...sources,
        join(metadata, "apm.yml"),
        join(metadata, "apm.lock.yaml"),
        join(destination, ".agents"),
        join(destination, ".codex/hooks.json"),
        join(destination, ".codex/config.toml"),
        join(destination, ".gitignore"),
      ]),
    ]);
    const binary = await lstat(executable);
    return {
      manifest,
      fingerprint: createHash("sha256")
        .update(fingerprint)
        .update(userManifest)
        .update(JSON.stringify(this.deps.env))
        .update(
          request.scope === "workspace"
            ? this.deps.workspaceRevision?.() || destination
            : "personal",
        )
        .update(
          `${executable}:${binary.dev}:${binary.ino}:${binary.size}:${binary.mtimeMs}`,
        )
        .digest("hex"),
    };
  }
  async preview(value: unknown): Promise<SkillPreview> {
    const request = validateRequest(value);
    const destination =
      request.scope === "personal" ? this.deps.home : this.deps.workspace();
    const view: SkillPreview = {
      id: null,
      scope: request.scope,
      destination: destination || "Open a workspace first",
      source: request.source,
      target: "codex",
      available: false,
      summary: "",
      paths: [],
      command: command(request),
    };
    try {
      if (!destination) throw new Error("Open a workspace first.");
      if (!isAbsolute(request.source))
        throw new Error(
          "Remote dependency resolution cannot yet be completely previewed. Select a local single-skill directory or use the terminal command.",
        );
      if (process.platform === "win32")
        throw new Error(
          "Windows APM installation is not yet qualified; use the terminal command.",
        );
      if (this.busy.has(destination))
        throw new Error(
          "An installation is already in progress for this destination.",
        );
      const executable = await (this.deps.executable
        ? this.deps.executable()
        : findApm(this.deps.env));
      const version = await this.run(executable, ["--version"], tmpdir());
      if (!/CLI version 0\.33\.0\b/.test(version))
        throw new Error(
          "This APM version is not yet qualified for reviewed installation (qualified: 0.33.0). Browsing remains available.",
        );
      const before = await this.check(request, destination, executable);
      const stage = await mkdtemp(join(tmpdir(), "orkworks-skill-preview-"));
      try {
        if (before.manifest)
          await writeFile(join(stage, "apm.yml"), before.manifest);
        await this.run(
          executable,
          [
            "install",
            request.source,
            "--target",
            "codex",
            "--only",
            "apm",
            "--no-trust-bin",
            "--dry-run",
          ],
          stage,
        );
      } finally {
        await rm(stage, { recursive: true, force: true });
      }
      const after = await this.check(request, destination, executable);
      if (after.fingerprint !== before.fingerprint)
        throw new Error("Inputs changed during preview. Preview again.");
      const metadata =
        request.scope === "personal" ? join(destination, ".apm") : destination;
      view.paths = [
        join(metadata, "apm.yml"),
        join(metadata, "apm.lock.yaml"),
        join(metadata, "apm_modules"),
        join(destination, ".agents/skills", basename(request.source)),
        join(destination, ".gitignore"),
      ];
      view.summary =
        "Add this local skill with APM 0.33.0 for Codex. Other tools may also discover the shared .agents/skills directory. APM may reconcile existing declared local skills and ownership metadata. This is a bounded operation review, not an exact file diff. No session is restarted.";
      view.available = true;
      view.id = randomUUID();
      // Only a bounded set of short-lived, single-use reviews is retained.
      for (const [id, review] of this.reviews)
        if (review.expires < Date.now()) this.reviews.delete(id);
      if (this.reviews.size >= 16)
        this.reviews.delete(this.reviews.keys().next().value!);
      this.reviews.set(view.id, {
        view,
        request,
        executable,
        fingerprint: after.fingerprint,
        expires: Date.now() + 5 * 60_000,
      });
    } catch (error) {
      view.reason =
        error instanceof Error ? error.message : "Preview unavailable.";
    }
    return view;
  }
  async install(id: unknown): Promise<SkillInstallResult> {
    const review = typeof id === "string" ? this.reviews.get(id) : undefined;
    if (!review)
      return {
        ok: false,
        message: "Review expired or was already used. Preview again.",
      };
    this.reviews.delete(id as string);
    const destination = review.view.destination;
    if (this.busy.has(destination))
      return { ok: false, message: "An installation is already in progress." };
    this.busy.add(destination);
    let started = false;
    try {
      const valid = async () => {
        if (
          review.expires < Date.now() ||
          (await this.check(review.request, destination, review.executable))
            .fingerprint !== review.fingerprint
        )
          throw new Error(
            "Source, destination or configuration changed. Preview again.",
          );
      };
      await valid();
      if (!(await this.deps.confirm(review.view)))
        return { ok: false, message: "Installation cancelled." };
      await valid();
      started = true;
      await this.run(
        review.executable,
        [
          "install",
          review.request.source,
          "--target",
          "codex",
          "--only",
          "apm",
          "--no-trust-bin",
          ...(review.request.scope === "personal" ? ["--global"] : []),
        ],
        destination,
      );
      return {
        ok: true,
        message:
          "Skill installed. Refresh or start a new coding-tool session if it does not appear; installation does not prove loading or use.",
      };
    } catch (error) {
      return {
        ok: false,
        message:
          (error instanceof Error ? error.message : "Installation failed.") +
          (started
            ? " Partial changes may remain; inspect the destination before retrying."
            : ""),
      };
    } finally {
      this.busy.delete(destination);
    }
  }
}
