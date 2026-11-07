export const STARTUP_FAILURE_MESSAGES = {
  codex_help_failed: "Could not check Codex session isolation support. Check the configured executable and retry.",
  codex_help_timeout: "The Codex session isolation check timed out. Check that the configured executable responds to --help, then retry.",
  codex_help_oversized: "The Codex session isolation check returned too much output. Check the configured executable and retry.",
  pty_open_failed: "Could not open a terminal for this session. Check available system resources and retry.",
  process_spawn_failed: "Could not start the coding tool. Check its configured executable and working directory, then retry.",
  terminal_io_failed: "Could not connect the session terminal. Check available system resources and retry.",
  instruction_setup_failed: "Could not prepare session instructions. Check workspace access and retry.",
  native_startup_failed: "Could not prepare the coding-tool integration. Check its integration status in Settings and retry.",
  startup_internal_failed: "Session startup failed. Check sidecar diagnostics before retrying.",
} as const;

export type StartupFailureCode = keyof typeof STARTUP_FAILURE_MESSAGES;
export interface StartupFailure {
  code: StartupFailureCode;
  operation: "launch" | "resume";
  occurredAt: string;
}

export const STARTUP_FAILURE_FALLBACK = STARTUP_FAILURE_MESSAGES.startup_internal_failed;

export function validateStartupFailure(value: unknown): StartupFailure | undefined {
  if (!value || typeof value !== "object" || Array.isArray(value)) return undefined;
  let serialized: string;
  try {
    serialized = JSON.stringify(value);
  } catch {
    return undefined;
  }
  if (new TextEncoder().encode(serialized).byteLength > 256) return undefined;
  const record = value as Record<string, unknown>;
  if (typeof record.code !== "string" || !Object.hasOwn(STARTUP_FAILURE_MESSAGES, record.code)) return undefined;
  if (record.operation !== "launch" && record.operation !== "resume") return undefined;
  if (typeof record.occurredAt !== "string" ||
    !/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/.test(record.occurredAt) ||
    !Number.isFinite(Date.parse(record.occurredAt)) ||
    new Date(record.occurredAt).toISOString() !== record.occurredAt) return undefined;
  return { code: record.code as StartupFailureCode, operation: record.operation, occurredAt: record.occurredAt };
}

export function startupFailureMessage(value: unknown): string | undefined {
  if (value === undefined || value === null) return undefined;
  const failure = validateStartupFailure(value);
  return failure ? STARTUP_FAILURE_MESSAGES[failure.code] : STARTUP_FAILURE_FALLBACK;
}
