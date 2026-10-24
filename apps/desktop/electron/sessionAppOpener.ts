import { isAbsolute, win32 } from "node:path";

// Deliberately duplicated in the renderer: neither side imports across the boundary.
export type SessionAppAvailability = { label: string; available: boolean; reason?: string };
export interface SessionAppDependencies {
  platform: string;
  arch: string;
  signal: AbortSignal;
  readSessions(): Promise<unknown>;
  readHarnesses(): Promise<unknown>;
  applicationForProtocol(url: string): Promise<string>;
  checkDirectory(path: string): Promise<void>;
  run(command: string, args: string[], cwd: string): Promise<string>;
  openExternal(url: string): Promise<void>;
}

type RecordValue = Record<string, unknown>;
const record = (value: unknown): RecordValue => value && typeof value === "object" && !Array.isArray(value) ? value as RecordValue : {};
const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

async function resolveSessionApp(sessionId: unknown, deps: SessionAppDependencies): Promise<{ availability: SessionAppAvailability; nativeId?: string; harnessId?: string; command?: string; cwd?: string }> {
  if (typeof sessionId !== "string" || !sessionId || sessionId.length > 256) throw new Error("Invalid session ID.");
  deps.signal.throwIfAborted();
  const sessions = await deps.readSessions();
  if (!Array.isArray(sessions)) throw new Error("Couldn't read the current sessions.");
  const session = record(sessions.find(value => record(value).id === sessionId));
  if (!session.id) throw new Error("This session is no longer available.");
  const harnessId = session.harnessId;
  const label = harnessId === "codex" ? "ChatGPT" : harnessId === "claude-code" ? "Claude" : "Native application";
  const unavailable = (reason: string) => ({ availability: { label, available: false, reason } });
  if (harnessId !== "codex" && harnessId !== "claude-code") return unavailable("This coding tool has no supported desktop destination.");
  const nativeId = record(session.resume).harnessSessionId;
  if (typeof nativeId !== "string" || !uuid.test(nativeId)) return unavailable("The exact native session ID has not been captured.");
  const harnesses = record(await deps.readHarnesses()).harnesses;
  const entry = record(Array.isArray(harnesses) ? harnesses.find(value => record(record(value).definition).id === harnessId) : undefined);
  const harness = record(entry.definition);
  // Custom/overridden tools need their own native-opening qualification.
  if (entry.origin !== "builtin") return unavailable("Desktop opening is not verified for this custom coding-tool configuration.");
  if (harnessId === "claude-code") {
    if (deps.platform !== "darwin" && !(deps.platform === "win32" && deps.arch === "x64")) return unavailable("Claude desktop handoff requires macOS or x64 Windows.");
    if (session.lifecycle !== "dead") return unavailable("Close this session in OrkWorks first to open it in Claude.");
  }
  const scheme = harnessId === "codex" ? "codex" : "claude";
  const application = await deps.applicationForProtocol(`${scheme}://`).catch(() => "");
  if (!application) return unavailable(`${label} is not installed or has no registered application link handler.`);
  let command: string | undefined;
  let cwd: string | undefined;
  if (harnessId === "claude-code") {
    const launch = record(harness.launch);
    if (launch.kind !== "command-template" || typeof launch.command !== "string" || !launch.command || /[\x00-\x1f]/.test(launch.command)) return unavailable("Claude's executable is unavailable.");
    command = launch.command;
    if (typeof session.cwd !== "string" || !isAbsolute(session.cwd) || session.cwd.includes("\0")) return unavailable("The session directory is unavailable.");
    cwd = session.cwd;
    await deps.checkDirectory(cwd);
    const help = await deps.run(command, ["--help"], cwd);
    if (!/--desktop\b/.test(help) || !/--resume\b/.test(help)) return unavailable("Update Claude Code to a version supporting desktop session handoff (2.1.285+).");
  }
  deps.signal.throwIfAborted();
  return { availability: { label, available: true }, nativeId, harnessId, command, cwd };
}

export async function inspectSessionApp(sessionId: unknown, deps: SessionAppDependencies): Promise<SessionAppAvailability> {
  return (await resolveSessionApp(sessionId, deps)).availability;
}

export async function openSessionApp(sessionId: unknown, deps: SessionAppDependencies): Promise<void> {
  const target = await resolveSessionApp(sessionId, deps);
  if (!target.availability.available || !target.nativeId) throw new Error(target.availability.reason ?? "Desktop opening is unavailable.");
  // Capability probes yield: Resume, native-ID replacement, or deletion may have
  // happened meanwhile. Read the authoritative session again before dispatch.
  const sessions = await deps.readSessions();
  const current = record(Array.isArray(sessions) ? sessions.find(value => record(value).id === sessionId) : undefined);
  if (current.harnessId !== target.harnessId || record(current.resume).harnessSessionId !== target.nativeId
    || (target.command && (current.lifecycle !== "dead" || current.cwd !== target.cwd))) {
    throw new Error("The session changed while checking its desktop destination. Close any running Claude session and try again.");
  }
  deps.signal.throwIfAborted();
  if (target.command && target.cwd) {
    await deps.run(target.command, ["--desktop", "--resume", target.nativeId], target.cwd);
  } else {
    await deps.openExternal(`codex://threads/${target.nativeId}`);
  }
}

/** Only these two fixed Claude operations may reach a Windows command interpreter.
 * No path, prompt, custom flag or renderer-provided text is placed in its command.
 */
export function sessionAppInvocation(platform: string, command: string, args: string[], interpreter = "C:\\Windows\\System32\\cmd.exe"): { file: string; args: string[]; env?: Record<string, string> } {
  const help = args.length === 1 && args[0] === "--help";
  const handoff = args.length === 3 && args[0] === "--desktop" && args[1] === "--resume" && uuid.test(args[2]);
  if (!help && !handoff) throw new Error("Unsupported desktop application command.");
  if (platform !== "win32") return { file: command, args };
  if (command !== "claude" || !win32.isAbsolute(interpreter) || win32.extname(interpreter).toLowerCase() !== ".exe") {
    throw new Error("This Windows Claude command is not supported for desktop handoff.");
  }
  // Node cannot execFile npm .cmd/.bat shims. /d disables AutoRun; /v:off
  // disables delayed expansion. Every word is fixed except the validated UUID.
  return { file: interpreter, env: { NoDefaultCurrentDirectoryInExePath: "1" }, args: ["/d", "/v:off", "/s", "/c", help ? "claude --help" : `claude --desktop --resume ${args[2]}`] };
}
