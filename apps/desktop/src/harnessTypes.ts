/** Mirrors crates/orkworksd/src/harness/definition.rs LaunchCapability. */
export type HarnessLaunch =
  | { kind: "command-template"; command: string; args: string[]; modelPrefix: string | null }
  | { kind: "platform-shell"; login: boolean };

export interface HarnessVoiceCapability {
  nativeVoice: boolean;
  requiresMicrophonePermission: boolean;
  orkworksDictation: boolean;
  orkworksVoiceCommands: boolean;
}

/** Mirrors crates/orkworksd/src/harness/definition.rs HarnessDefinition (v2, resolved-registry shape). */
export interface HarnessConfig {
  id: string;
  name: string;
  retired: boolean;
  launch: HarnessLaunch;
  defaultModel: string | null;
  resume: unknown;
  models: unknown;
  peon: unknown;
  capacity: unknown;
  sessionSignals: unknown;
  integration: unknown;
  voice: HarnessVoiceCapability | null;
}

export type HarnessOrigin = "builtin" | "override" | "custom";

export interface HarnessCompatibilityMetadata {
  profile: string | null;
  sessionSignals: unknown;
  integration: unknown;
}

export interface HarnessConfigEntry extends HarnessConfig {
  origin: HarnessOrigin;
  profile: string | null;
  compatibility: HarnessCompatibilityMetadata;
  storedOverride?: unknown;
  documentRevision?: string | null;
}

export interface HarnessListResponse {
  documentRevision: string | null;
  harnesses: HarnessConfigEntry[];
}

export type HarnessEditorMode = "create" | "custom" | "override";

export interface HarnessEditorMetadata {
  entry?: HarnessConfigEntry;
  documentRevision: string | null;
  duplicateSourceId?: string;
}

export interface HarnessValidationDiagnostic {
  code: string;
  message: string;
  path?: string;
  line?: number;
  column?: number;
}

export interface HarnessDraftParseResult {
  value: Record<string, unknown> | null;
  diagnostics: HarnessValidationDiagnostic[];
}

const MAX_HARNESS_DEFINITION_BYTES = 256 * 1024;
const DERIVED_HARNESS_FIELDS = new Set([
  "integration",
  "sessionSignals",
  "compatibilityProfile",
  "compatibilityProfiles",
  "profile",
  "compatibility",
  "origin",
  "storedOverride",
  "documentRevision",
]);
const COMPLETE_FIELDS = new Set([
  "id",
  "name",
  "retired",
  "launch",
  "defaultModel",
  "resume",
  "models",
  "peon",
  "capacity",
  "voice",
  "minVersion",
  "labelResetCommands",
]);
const PATCH_FIELDS = new Set([
  "name",
  "launch",
  "defaultModel",
  "resume",
  "models",
  "peon",
  "capacity",
  "voice",
  "minVersion",
  "labelResetCommands",
]);
const PLACEHOLDERS = new Set(["{model}", "{cwd}", "{repoRoot}", "{harnessSessionId}"]);

function isRecord(value: unknown): value is Record<string, unknown> {
  return !!value && typeof value === "object" && !Array.isArray(value);
}

export function parseHarnessDraft(text: string, mode: HarnessEditorMode): HarnessDraftParseResult {
  if (new TextEncoder().encode(text).byteLength > MAX_HARNESS_DEFINITION_BYTES) {
    return {
      value: null,
      diagnostics: [{
        code: "document_too_large",
        message: "Harness definition exceeds the 256 KiB limit.",
        path: "$",
      }],
    };
  }
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch (error) {
    const message = error instanceof Error ? error.message : "Invalid JSON.";
    const position = message.match(/position\s+(\d+)/i)?.[1];
    const lineColumn = message.match(/line\s+(\d+)\s+column\s+(\d+)/i);
    const offset = position === undefined ? text.length : Number(position);
    const before = text.slice(0, Number.isFinite(offset) ? offset : text.length);
    return {
      value: null,
      diagnostics: [{
        code: "invalid_json",
        message,
        line: lineColumn ? Number(lineColumn[1]) : before.split("\n").length,
        column: lineColumn ? Number(lineColumn[2]) : before.length - before.lastIndexOf("\n"),
      }],
    };
  }
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return {
      value: null,
      diagnostics: [{ code: "invalid_schema", message: "Configuration must be a JSON object.", path: "$" }],
    };
  }

  const object = value as Record<string, unknown>;
  const diagnostics: HarnessValidationDiagnostic[] = [];
  const duplicate = findDuplicateJsonKey(text);
  if (duplicate) {
    const location = lineAndColumn(text, duplicate.offset);
    diagnostics.push({
      code: "duplicate_key",
      message: `Duplicate object key ${duplicate.key}.`,
      path: "$",
      line: location.line,
      column: location.column,
    });
  }
  const allowedFields = mode === "override" ? PATCH_FIELDS : COMPLETE_FIELDS;
  for (const field of Object.keys(object)) {
    if (field in object) {
      if (DERIVED_HARNESS_FIELDS.has(field)) {
        diagnostics.push({
          code: "custom_authority_binding",
          message: `${field} is derived by OrkWorks and cannot be edited here.`,
          path: `$.${field}`,
        });
      } else if (!allowedFields.has(field)) {
        diagnostics.push({
          code: "unknown_field",
          message: `Unknown ${mode === "override" ? "override" : "custom definition"} field ${field}.`,
          path: `$.${field}`,
        });
      }
    }
  }

  if (mode === "override") validatePatch(object, diagnostics);
  else validateCompleteDefinition(object, diagnostics);

  return { value: object, diagnostics };
}

function validateCompleteDefinition(
  object: Record<string, unknown>,
  diagnostics: HarnessValidationDiagnostic[],
) {
  requireField(object, "id", diagnostics);
  requireField(object, "name", diagnostics);
  requireField(object, "launch", diagnostics);
  validateId(object.id, "$.id", diagnostics);
  validateString(object.name, "$.name", diagnostics, "invalid_schema");
  if (typeof object.name === "string" && !object.name.trim()) {
    diagnostics.push({ code: "invalid_name", message: "Harness name is required.", path: "$.name" });
  }
  validateBooleanIfPresent(object.retired, "$.retired", diagnostics);
  validateLaunch(object.launch, "$.launch", true, diagnostics);
  validateNullableStringIfPresent(object.defaultModel, "$.defaultModel", diagnostics);
  validateResume(object.resume, "$.resume", true, diagnostics);
  validateModels(object.models, "$.models", true, diagnostics);
  validatePeon(object.peon, "$.peon", true, diagnostics);
  validateCapacity(object.capacity, "$.capacity", true, diagnostics);
  validateVoice(object.voice, "$.voice", true, diagnostics);
  validateMinVersion(object.minVersion, "$.minVersion", true, diagnostics);
  validateStringArrayIfPresent(object.labelResetCommands, "$.labelResetCommands", diagnostics);

  if (isRecord(object.peon)) {
    const commandOverride = object.peon.commandOverride;
    const launchIsPlatformShell = isRecord(object.launch) && object.launch.kind === "platform-shell";
    if ((typeof commandOverride === "string" && !commandOverride.trim())
      || (commandOverride === undefined && launchIsPlatformShell)) {
      diagnostics.push({
        code: "invalid_peon_command",
        message: "Peon requires a non-empty command: set peon.commandOverride or use a command-template launch.",
        path: "$.peon.commandOverride",
      });
    }
  }
}

function validatePatch(
  object: Record<string, unknown>,
  diagnostics: HarnessValidationDiagnostic[],
) {
  if ("name" in object) validateString(object.name, "$.name", diagnostics, "invalid_schema");
  if ("launch" in object) validateLaunch(object.launch, "$.launch", false, diagnostics);
  if ("defaultModel" in object) validateNullableStringIfPresent(object.defaultModel, "$.defaultModel", diagnostics);
  if ("resume" in object) validateResume(object.resume, "$.resume", true, diagnostics);
  if ("models" in object) validateModels(object.models, "$.models", true, diagnostics);
  if ("peon" in object) validatePeon(object.peon, "$.peon", false, diagnostics);
  if ("capacity" in object) validateCapacity(object.capacity, "$.capacity", true, diagnostics);
  if ("voice" in object) validateVoice(object.voice, "$.voice", false, diagnostics);
  if ("minVersion" in object) validateMinVersion(object.minVersion, "$.minVersion", true, diagnostics);
  if ("labelResetCommands" in object) {
    if (object.labelResetCommands !== null) {
      validateStringArrayIfPresent(object.labelResetCommands, "$.labelResetCommands", diagnostics);
    }
  }
}

function validateLaunch(
  value: unknown,
  path: string,
  complete: boolean,
  diagnostics: HarnessValidationDiagnostic[],
) {
  const object = expectObject(value, path, diagnostics);
  if (!object) return;
  rejectUnknown(object, path, new Set(["kind", "command", "args", "modelPrefix", "login"]), diagnostics);
  if (!complete && object.kind === undefined) {
    if ("command" in object) {
      validateString(object.command, `${path}.command`, diagnostics, "invalid_schema");
      if (typeof object.command === "string") {
        if (!object.command.trim()) diagnostics.push({ code: "invalid_command", message: "Launch command is required.", path: `${path}.command` });
        validateTemplate(object.command, `${path}.command`, diagnostics);
      }
    }
    if ("args" in object) validateStringArrayIfPresent(object.args, `${path}.args`, diagnostics);
    validateNullableStringIfPresent(object.modelPrefix, `${path}.modelPrefix`, diagnostics);
    if ("login" in object) validateBoolean(object.login, `${path}.login`, diagnostics);
    return;
  }
  if (typeof object.kind !== "string") {
    diagnostics.push({ code: "invalid_schema", message: "Expected a string.", path: `${path}.kind` });
    return;
  }
  if (object.kind !== "command-template" && object.kind !== "platform-shell") {
    diagnostics.push({ code: "invalid_schema", message: "Launch kind must be command-template or platform-shell.", path: `${path}.kind` });
    return;
  }
  if (object.kind === "command-template") {
    if (complete) {
      requireNestedField(object, "command", path, diagnostics);
      requireNestedField(object, "args", path, diagnostics);
    }
    if ("command" in object) {
      validateString(object.command, `${path}.command`, diagnostics, "invalid_schema");
      if (typeof object.command === "string") {
        if (!object.command.trim()) diagnostics.push({ code: "invalid_command", message: "Launch command is required.", path: `${path}.command` });
        validateTemplate(object.command, `${path}.command`, diagnostics);
      }
    }
    if ("args" in object) validateStringArrayIfPresent(object.args, `${path}.args`, diagnostics);
    validateNullableStringIfPresent(object.modelPrefix, `${path}.modelPrefix`, diagnostics);
    if ("login" in object) diagnostics.push({ code: "invalid_capability_combination", message: "Field login is not valid for command-template launch.", path: `${path}.login` });
  } else {
    if (complete) requireNestedField(object, "login", path, diagnostics);
    if ("login" in object) validateBoolean(object.login, `${path}.login`, diagnostics);
    for (const field of ["command", "args", "modelPrefix"]) {
      if (field in object) diagnostics.push({ code: "invalid_capability_combination", message: `Field ${field} is not valid for platform-shell launch.`, path: `${path}.${field}` });
    }
  }
}

function validateResume(
  value: unknown,
  path: string,
  complete: boolean,
  diagnostics: HarnessValidationDiagnostic[],
) {
  if (value === undefined || value === null) return;
  const object = expectObject(value, path, diagnostics);
  if (!object) return;
  rejectUnknown(object, path, new Set(["exact", "latestCwd", "latestRepo"]), diagnostics);
  for (const field of ["exact", "latestCwd", "latestRepo"]) {
    if (field in object && object[field] !== null) validateCommandTemplate(object[field], `${path}.${field}`, complete, diagnostics);
  }
}

function validateCommandTemplate(value: unknown, path: string, complete: boolean, diagnostics: HarnessValidationDiagnostic[]) {
  const object = expectObject(value, path, diagnostics);
  if (!object) return;
  rejectUnknown(object, path, new Set(["command", "args"]), diagnostics);
  if (complete) {
    requireNestedField(object, "command", path, diagnostics);
    requireNestedField(object, "args", path, diagnostics);
  }
  if ("command" in object) {
    validateString(object.command, `${path}.command`, diagnostics, "invalid_schema");
    if (typeof object.command === "string") {
      if (!object.command.trim()) diagnostics.push({ code: "invalid_resume_command", message: "Resume command is required.", path: `${path}.command` });
      validateTemplate(object.command, `${path}.command`, diagnostics);
    }
  }
  if ("args" in object) validateStringArrayIfPresent(object.args, `${path}.args`, diagnostics);
}

function validateModels(value: unknown, path: string, complete: boolean, diagnostics: HarnessValidationDiagnostic[]) {
  if (value === undefined || value === null) return;
  const object = expectObject(value, path, diagnostics);
  if (!object) return;
  rejectUnknown(object, path, new Set(["kind", "models", "command", "args"]), diagnostics);
  if (typeof object.kind !== "string") {
    diagnostics.push({ code: "invalid_schema", message: "Expected a string.", path: `${path}.kind` });
    return;
  }
  if (!["static", "command", "http"].includes(object.kind)) {
    diagnostics.push({ code: "invalid_schema", message: "Model kind must be static, command, or http.", path: `${path}.kind` });
    return;
  }
  if (object.kind === "static") {
    if (complete || "models" in object) {
      if (complete) requireNestedField(object, "models", path, diagnostics);
      if ("models" in object) validateStringArrayIfPresent(object.models, `${path}.models`, diagnostics);
    }
    for (const field of ["command", "args"]) if (field in object) diagnostics.push({ code: "invalid_capability_combination", message: `Field ${field} is not valid for static models.`, path: `${path}.${field}` });
  } else if (object.kind === "command") {
    if (complete) {
      requireNestedField(object, "command", path, diagnostics);
      requireNestedField(object, "args", path, diagnostics);
    }
    if ("command" in object) {
      validateString(object.command, `${path}.command`, diagnostics, "invalid_schema");
      if (typeof object.command === "string") {
        if (!object.command.trim()) diagnostics.push({ code: "invalid_model_command", message: "Model command is required.", path: `${path}.command` });
        validateTemplate(object.command, `${path}.command`, diagnostics);
      }
    }
    if ("args" in object) validateStringArrayIfPresent(object.args, `${path}.args`, diagnostics);
    if ("models" in object) diagnostics.push({ code: "invalid_capability_combination", message: "Field models is not valid for command models.", path: `${path}.models` });
  } else {
    for (const field of ["models", "command", "args"]) if (field in object) diagnostics.push({ code: "invalid_capability_combination", message: `Field ${field} is not valid for http models.`, path: `${path}.${field}` });
  }
}

function validatePeon(value: unknown, path: string, complete: boolean, diagnostics: HarnessValidationDiagnostic[]) {
  if (value === undefined || value === null) return;
  const object = expectObject(value, path, diagnostics);
  if (!object) return;
  rejectUnknown(object, path, new Set(["commandOverride", "args", "modelArgTemplate", "supportsModel", "timeoutSecs", "promptTransport"]), diagnostics);
  for (const field of ["commandOverride", "modelArgTemplate"]) {
    if (field in object) validateNullableStringIfPresent(object[field], `${path}.${field}`, diagnostics);
  }
  if (complete) {
    requireNestedField(object, "args", path, diagnostics);
    requireNestedField(object, "supportsModel", path, diagnostics);
    requireNestedField(object, "timeoutSecs", path, diagnostics);
  }
  if ("args" in object) validateStringArrayIfPresent(object.args, `${path}.args`, diagnostics);
  if ("supportsModel" in object) validateBoolean(object.supportsModel, `${path}.supportsModel`, diagnostics);
  if ("timeoutSecs" in object) validateNonNegativeInteger(object.timeoutSecs, `${path}.timeoutSecs`, diagnostics);
  if ("promptTransport" in object && object.promptTransport !== "stdin" && object.promptTransport !== "argument") {
    diagnostics.push({ code: "invalid_schema", message: "promptTransport must be stdin or argument.", path: `${path}.promptTransport` });
  }
  if (typeof object.modelArgTemplate === "string") validateTemplate(object.modelArgTemplate, `${path}.modelArgTemplate`, diagnostics);
}

function validateCapacity(value: unknown, path: string, complete: boolean, diagnostics: HarnessValidationDiagnostic[]) {
  if (value === undefined || value === null) return;
  const object = expectObject(value, path, diagnostics);
  if (!object) return;
  rejectUnknown(object, path, new Set(["kind", "limitPatterns"]), diagnostics);
  if (typeof object.kind !== "string") {
    diagnostics.push({ code: "invalid_schema", message: "Expected a string.", path: `${path}.kind` });
  } else if (object.kind !== "terminal-patterns") {
    diagnostics.push({ code: "invalid_schema", message: "Capacity kind must be terminal-patterns.", path: `${path}.kind` });
  }
  if (complete || "limitPatterns" in object) {
    if (complete) requireNestedField(object, "limitPatterns", path, diagnostics);
    if ("limitPatterns" in object) validateStringArrayIfPresent(object.limitPatterns, `${path}.limitPatterns`, diagnostics);
  }
}

function validateVoice(value: unknown, path: string, complete: boolean, diagnostics: HarnessValidationDiagnostic[]) {
  if (value === undefined || value === null) return;
  const object = expectObject(value, path, diagnostics);
  if (!object) return;
  const fields = ["nativeVoice", "requiresMicrophonePermission", "orkworksDictation", "orkworksVoiceCommands"];
  rejectUnknown(object, path, new Set(fields), diagnostics);
  for (const field of fields) {
    if (complete && !(field in object)) requireNestedField(object, field, path, diagnostics);
    if (field in object) validateBoolean(object[field], `${path}.${field}`, diagnostics);
  }
}

function validateMinVersion(value: unknown, path: string, complete: boolean, diagnostics: HarnessValidationDiagnostic[]) {
  if (value === undefined || value === null) return;
  const object = expectObject(value, path, diagnostics);
  if (!object) return;
  rejectUnknown(object, path, new Set(["min"]), diagnostics);
  if (complete || "min" in object) {
    if (complete) requireNestedField(object, "min", path, diagnostics);
    if ("min" in object) {
      const min = object.min;
      if (!Array.isArray(min) || min.length !== 3 || !min.every((part) => Number.isInteger(part) && (part as number) >= 0)) {
        diagnostics.push({ code: "invalid_schema", message: "min must be an array of three non-negative integers.", path: `${path}.min` });
      }
    }
  }
}

function validateTemplate(value: string, path: string, diagnostics: HarnessValidationDiagnostic[]) {
  let cursor = 0;
  while (cursor < value.length) {
    const open = value.indexOf("{", cursor);
    const close = value.indexOf("}", cursor);
    if (close !== -1 && (open === -1 || close < open)) {
      diagnostics.push({ code: "invalid_placeholder", message: "Command templates use only {model}, {cwd}, {repoRoot}, or {harnessSessionId}.", path });
      return;
    }
    if (open === -1) return;
    const end = value.indexOf("}", open + 1);
    if (end === -1 || !PLACEHOLDERS.has(value.slice(open, end + 1))) {
      diagnostics.push({ code: "invalid_placeholder", message: "Command templates use only {model}, {cwd}, {repoRoot}, or {harnessSessionId}.", path });
      return;
    }
    cursor = end + 1;
  }
}

function validateId(value: unknown, path: string, diagnostics: HarnessValidationDiagnostic[]) {
  if (typeof value !== "string" || !/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(value)) {
    diagnostics.push({ code: "invalid_id", message: "Harness ID must be lowercase kebab-case.", path });
  }
}

function validateString(value: unknown, path: string, diagnostics: HarnessValidationDiagnostic[], code: string) {
  if (typeof value !== "string") diagnostics.push({ code, message: "Expected a string.", path });
}

function validateBoolean(value: unknown, path: string, diagnostics: HarnessValidationDiagnostic[]) {
  if (typeof value !== "boolean") diagnostics.push({ code: "invalid_schema", message: "Expected a boolean.", path });
}

function validateBooleanIfPresent(value: unknown, path: string, diagnostics: HarnessValidationDiagnostic[]) {
  if (value !== undefined) validateBoolean(value, path, diagnostics);
}

function validateNullableStringIfPresent(value: unknown, path: string, diagnostics: HarnessValidationDiagnostic[]) {
  if (value !== undefined && value !== null && typeof value !== "string") diagnostics.push({ code: "invalid_schema", message: "Expected a string or null.", path });
}

function validateStringArrayIfPresent(value: unknown, path: string, diagnostics: HarnessValidationDiagnostic[]) {
  if (!Array.isArray(value) || !value.every((item) => typeof item === "string")) diagnostics.push({ code: "invalid_schema", message: "Expected an array of strings.", path });
}

function validateNonNegativeInteger(value: unknown, path: string, diagnostics: HarnessValidationDiagnostic[]) {
  if (!Number.isInteger(value) || (value as number) < 0) diagnostics.push({ code: "invalid_schema", message: "Expected a non-negative integer.", path });
}

function expectObject(value: unknown, path: string, diagnostics: HarnessValidationDiagnostic[]): Record<string, unknown> | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    diagnostics.push({ code: "invalid_schema", message: "Expected a JSON object.", path });
    return null;
  }
  return value as Record<string, unknown>;
}

function requireField(object: Record<string, unknown>, field: string, diagnostics: HarnessValidationDiagnostic[]) {
  if (!(field in object)) diagnostics.push({ code: "missing_field", message: `Required field ${field} is missing.`, path: `$.${field}` });
}

function requireNestedField(object: Record<string, unknown>, field: string, path: string, diagnostics: HarnessValidationDiagnostic[]) {
  if (!(field in object)) diagnostics.push({ code: "missing_field", message: `Required field ${field} is missing.`, path: `${path}.${field}` });
}

function rejectUnknown(object: Record<string, unknown>, path: string, allowed: Set<string>, diagnostics: HarnessValidationDiagnostic[]) {
  for (const field of Object.keys(object)) {
    if (!allowed.has(field)) diagnostics.push({ code: "unknown_field", message: `Unknown custom definition field ${field}.`, path: `${path}.${field}` });
  }
}

function lineAndColumn(text: string, offset: number): { line: number; column: number } {
  const before = text.slice(0, offset);
  return { line: before.split("\n").length, column: before.length - before.lastIndexOf("\n") };
}

function findDuplicateJsonKey(text: string): { key: string; offset: number } | null {
  let cursor = 0;
  const whitespace = () => { while (/\s/.test(text[cursor] ?? "")) cursor += 1; };
  const stringEnd = (): number => {
    if (text[cursor] !== '"') return -1;
    cursor += 1;
    while (cursor < text.length) {
      const code = text.charCodeAt(cursor);
      if (code === 34) return ++cursor;
      if (code === 92) cursor += 2;
      else {
        if (code < 32) return -1;
        cursor += 1;
      }
    }
    return -1;
  };
  const value = (): { key: string; offset: number } | null => {
    whitespace();
    if (text[cursor] === "{") {
      cursor += 1;
      const keys = new Set<string>();
      whitespace();
      if (text[cursor] === "}") { cursor += 1; return null; }
      while (cursor < text.length) {
        whitespace();
        const offset = cursor;
        const end = stringEnd();
        if (end < 0) return null;
        let key: string;
        try { key = JSON.parse(text.slice(offset, end)); } catch { return null; }
        if (keys.has(key)) return { key, offset };
        keys.add(key);
        whitespace();
        if (text[cursor++] !== ":") return null;
        const nested = value();
        if (nested) return nested;
        whitespace();
        if (text[cursor] === "}") { cursor += 1; return null; }
        if (text[cursor++] !== ",") return null;
      }
      return null;
    }
    if (text[cursor] === "[") {
      cursor += 1;
      whitespace();
      if (text[cursor] === "]") { cursor += 1; return null; }
      while (cursor < text.length) {
        const nested = value();
        if (nested) return nested;
        whitespace();
        if (text[cursor] === "]") { cursor += 1; return null; }
        if (text[cursor++] !== ",") return null;
      }
      return null;
    }
    if (text[cursor] === '"') { stringEnd(); return null; }
    while (cursor < text.length && !/[\s,\]}]/.test(text[cursor] ?? "")) cursor += 1;
    return null;
  };
  return value();
}

export interface CreateSessionOptions {
  harnessId?: string;
  model?: string;
  initialPrompt?: string;
}

export type IntegrationRegistration = "unsupported" | "absent" | "installed" | "drifted" | "error";
export type IntegrationOwnership = "none" | "ork_works" | "ambiguous";
export type IntegrationActivation = "active" | "needs_trust" | "disabled" | "unknown" | "not_applicable";
export type IntegrationCoverage = "full" | "limited" | "none";

export interface IntegrationDiagnostic {
  code: string;
  message: string;
  action?: string;
}

export interface IntegrationConfirmation {
  toolName: string;
  workspaceLabel: string;
  coverageSummary: string;
  relativePaths: string[];
  executableCodeWarning: boolean;
}

export interface IntegrationStatus {
  harnessId: string;
  enabled: boolean;
  toolDetected: boolean;
  registration: IntegrationRegistration;
  ownership: IntegrationOwnership;
  activation: IntegrationActivation;
  coverage: IntegrationCoverage;
  diagnostics: IntegrationDiagnostic[];
  confirmation: IntegrationConfirmation | null;
  /** Number of active harness rows currently consuming this shared adapter. */
  activeConsumerCount?: number;
}

export type IntegrationStatusResult =
  | { ok: true; status: IntegrationStatus }
  | { ok: false; error: string };
