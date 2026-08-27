const MAX_DIAGNOSTIC_MESSAGE_LENGTH = 200;

const BEARER_TOKEN_PATTERN = /(\bBearer\s+)[^\s,}]+/gi;
const SENSITIVE_ASSIGNMENT_PATTERN =
  /((?:["']?(?:token|password|secret|authorization|api[_-]?key|cookie|prompt|workspace|cwd|path|content|body|headers)["']?\s*[:=]\s*))(?:Bearer\s+)?(?:"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'|[^\s,}]+)/gi;
const URL_PATTERN = /(?:https?|file|data):\/\/[^\s"'<>`]+/gi;
const POSIX_PATH_PATTERN = /(^|[\s("'=])\/(?:[^\s"'<>`/]+\/)+[^\s"'<>`]+/g;
const WINDOWS_PATH_PATTERN = /(^|[\s("'=])(?:[A-Za-z]:\\|\\\\)[^\s"'<>`]+/g;
const SENSITIVE_JSON_KEY_PATTERN = /(?:token|password|secret|authorization|api[_-]?key|cookie|prompt|workspace|cwd|path|content|body|headers)/i;

export interface RendererConsoleDiagnostic {
  type: "console-message";
  level: number;
  origin: string;
  line: number;
}

function redactJsonSecrets(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(redactJsonSecrets);
  if (typeof value === "string") return sanitizePlainText(value);
  if (value === null || typeof value !== "object") return value;

  return Object.fromEntries(
    Object.entries(value).map(([key, child]) => [
      key,
      SENSITIVE_JSON_KEY_PATTERN.test(key) ? "[redacted]" : redactJsonSecrets(child),
    ]),
  );
}

function redactStructuredJson(message: string): string | null {
  try {
    const parsed = JSON.parse(message) as unknown;
    if (parsed !== null && typeof parsed === "object") {
      return JSON.stringify(redactJsonSecrets(parsed));
    }
  } catch {
    // Console messages are often plain text; the regular sanitizer handles those.
  }
  return null;
}

function sanitizePlainText(message: string): string {
  return message
    .replace(SENSITIVE_ASSIGNMENT_PATTERN, "$1[redacted]")
    .replace(BEARER_TOKEN_PATTERN, "$1[redacted]")
    .replace(URL_PATTERN, "[url]")
    .replace(POSIX_PATH_PATTERN, "$1[path]")
    .replace(WINDOWS_PATH_PATTERN, "$1[path]");
}

export function rendererOrigin(url: string): string {
  try {
    return new URL(url).origin;
  } catch {
    return "unknown";
  }
}

export function rendererConsoleLevel(level: "info" | "warning" | "error" | "debug"): number {
  return { debug: 0, info: 1, warning: 2, error: 3 }[level];
}

export function rendererConsoleDiagnostic(
  level: number,
  sourceId: string,
  line: number,
): RendererConsoleDiagnostic {
  return {
    type: "console-message",
    level,
    origin: rendererOrigin(sourceId),
    line,
  };
}

export function sanitizeRendererDiagnosticMessage(message: string): string {
  return (redactStructuredJson(message) ?? sanitizePlainText(message))
    .slice(0, MAX_DIAGNOSTIC_MESSAGE_LENGTH);
}
