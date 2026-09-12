/** Editable declaration only: this type never grants execution authority. */
export interface InferenceDefinition {
  kind: "command";
  command: string;
  args: string[];
  input: "stdin" | "file";
  output: "result-json-v1";
  timeoutSecs?: number;
  reasoningEffortArgs?: string[];
}

export function validateInferenceDefinition(value: unknown): { code: string; message: string; path: string }[] {
  if (value === undefined || value === null) return [];
  const errors: { code: string; message: string; path: string }[] = [];
  const fail = (field: string, message: string) => errors.push({ code: "invalid_inference", message, path: `$.inference${field ? `.${field}` : ""}` });
  if (typeof value !== "object" || Array.isArray(value)) {
    fail("", "Inference must be a complete command definition or null.");
    return errors;
  }
  const object = value as Record<string, unknown>;
  const allowed = new Set(["kind", "command", "args", "input", "output", "timeoutSecs", "reasoningEffortArgs"]);
  for (const field of Object.keys(object)) if (!allowed.has(field)) fail(field, "Unknown inference field.");
  if (object.kind !== "command") fail("kind", "Custom inference must use command kind.");
  const bounded = (text: unknown): text is string => typeof text === "string" && new TextEncoder().encode(text).length <= 4096 && !/\p{Cc}/u.test(text);
  if (!bounded(object.command) || !object.command.trim() || /[{}]/.test(object.command)) {
    fail("command", "Expected a nonempty executable name or absolute path, without placeholders or control characters.");
  } else if (/[\\/:]/.test(object.command) && !/^(?:\/|[A-Za-z]:[\\/]|\\\\)/.test(object.command)) {
    fail("command", "Relative executable paths are not supported.");
  }
  if (object.input !== "stdin" && object.input !== "file") fail("input", "Input must be stdin or file.");
  if (object.output !== "result-json-v1") fail("output", "Output must be result-json-v1.");
  if ("timeoutSecs" in object && (!Number.isInteger(object.timeoutSecs) || (object.timeoutSecs as number) < 1 || (object.timeoutSecs as number) > 120)) fail("timeoutSecs", "Timeout must be 1–120 seconds.");
  const checkArgs = (args: unknown, field: string, expected: Record<string, number>): number => {
    if (!Array.isArray(args) || !args.every(bounded)) {
      fail(field, "Expected bounded string arguments without control characters.");
      return 0;
    }
    const found: Record<string, number> = {};
    for (const arg of args) {
      const rest = arg.replace(/\{[^{}]*\}/g, (token: string) => {
        found[token] = (found[token] ?? 0) + 1;
        if (!(token in expected)) fail(field, "Unknown or misplaced inference placeholder.");
        return "";
      });
      if (/[{}]/.test(rest)) fail(field, "Malformed inference placeholder.");
    }
    for (const [token, count] of Object.entries(expected)) if ((found[token] ?? 0) !== count) fail(field, `Expected ${count} occurrence of ${token}.`);
    return args.length;
  };
  const argsCount = checkArgs(object.args, "args", { "{model}": 1, "{promptFile}": object.input === "file" ? 1 : 0 });
  const effortCount = "reasoningEffortArgs" in object ? checkArgs(object.reasoningEffortArgs, "reasoningEffortArgs", { "{effort}": 1 }) : 0;
  if (argsCount + effortCount > 64) fail("args", "At most 64 combined arguments are supported.");
  return errors;
}
