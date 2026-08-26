const MAX_DIAGNOSTIC_MESSAGE_LENGTH = 200;

const BEARER_TOKEN_PATTERN = /(\bBearer\s+)[^\s,}]+/gi;
const SENSITIVE_ASSIGNMENT_PATTERN =
  /((?:["']?(?:token|password|secret|authorization|api[_-]?key|cookie|prompt|workspace|cwd|path|content|body|headers)["']?\s*[:=]\s*))(?:Bearer\s+)?(?:"[^"]*"|'[^']*'|[^\s,}]+)/gi;
const URL_PATTERN = /(?:https?|file|data):\/\/[^\s"'<>`]+/gi;
const POSIX_PATH_PATTERN = /(^|[\s("'=])\/(?:Users|private|tmp|var|home)\/[^\s"'<>`]+/g;
const WINDOWS_PATH_PATTERN = /(^|[\s("'=])(?:[A-Za-z]:\\|\\\\)[^\s"'<>`]+/g;

export function rendererOrigin(url: string): string {
  try {
    return new URL(url).origin;
  } catch {
    return "unknown";
  }
}

export function sanitizeRendererDiagnosticMessage(message: string): string {
  return message
    .replace(SENSITIVE_ASSIGNMENT_PATTERN, "$1[redacted]")
    .replace(BEARER_TOKEN_PATTERN, "$1[redacted]")
    .replace(URL_PATTERN, "[url]")
    .replace(POSIX_PATH_PATTERN, "$1[path]")
    .replace(WINDOWS_PATH_PATTERN, "$1[path]")
    .slice(0, MAX_DIAGNOSTIC_MESSAGE_LENGTH);
}
