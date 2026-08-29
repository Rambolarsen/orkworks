export interface ElectronHarnessConfig {
  id: string;
  name: string;
  retired: boolean;
  launch:
    | { kind: "command-template"; command: string; args: string[]; modelPrefix: string | null }
    | { kind: "platform-shell"; login: boolean };
  integration: unknown;
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
}

export type IntegrationStatusResult =
  | { ok: true; status: IntegrationStatus }
  | { ok: false; error: string };

export interface ActiveHarnessIntegrationResult {
  operation: "install" | "repair" | "uninstall" | "skipped";
  outcome: "succeeded" | "failed" | "unsupported" | "stale_workspace";
  registration: IntegrationRegistration;
  activation: IntegrationActivation;
  coverage: IntegrationCoverage;
  diagnosticCode?: string;
  message?: string;
}

export interface ActiveHarnessSaveResult {
  activeHarnesses: {
    outcome: "persisted" | "failed" | "stale_workspace";
    message?: string;
  };
  integrations: Record<string, ActiveHarnessIntegrationResult>;
}

export interface WorkspaceGuardSnapshot {
  workspacePath: string | null;
  generation: number;
}

export interface ActiveHarnessIntegrationDeps {
  captureWorkspaceGuard(): WorkspaceGuardSnapshot;
  persistActiveHarnesses(ids: string[]): Promise<{ ok: true } | { ok: false; error: string }>;
  listHarnesses(): Promise<ElectronHarnessConfig[]>;
  getIntegrationStatus(harnessId: string): Promise<IntegrationStatusResult>;
  installIntegration(harnessId: string): Promise<IntegrationStatusResult>;
  uninstallIntegration(harnessId: string): Promise<IntegrationStatusResult>;
}

const STALE_WORKSPACE_MESSAGE = "Workspace changed while saving coding tools. Reload the current workspace and retry.";
const STATUS_UNAVAILABLE_CODE = "status_unavailable";
const MUTATION_FAILED_CODE = "mutation_failed";
const OWNERSHIP_AMBIGUOUS_CODE = "ownership_ambiguous";
const STALE_WORKSPACE_CODE = "stale_workspace";

interface PlannedMutation {
  operation: ActiveHarnessIntegrationResult["operation"];
  mutate: boolean;
}

function selectableHarnesses(harnesses: ElectronHarnessConfig[]): ElectronHarnessConfig[] {
  return harnesses.filter((harness) => !harness.retired);
}

function integrationResultFromStatus(
  operation: ActiveHarnessIntegrationResult["operation"],
  outcome: ActiveHarnessIntegrationResult["outcome"],
  status: IntegrationStatus,
  overrides: Partial<Pick<ActiveHarnessIntegrationResult, "diagnosticCode" | "message">> = {},
): ActiveHarnessIntegrationResult {
  const firstDiagnostic = status.diagnostics[0];
  const diagnosticCode = overrides.diagnosticCode ?? firstDiagnostic?.code;
  const message = overrides.message ?? firstDiagnostic?.message;
  return {
    operation,
    outcome,
    registration: status.registration,
    activation: status.activation,
    coverage: status.coverage,
    ...(diagnosticCode ? { diagnosticCode } : {}),
    ...(message ? { message } : {}),
  };
}

function fallbackIntegrationResult(
  operation: ActiveHarnessIntegrationResult["operation"],
  outcome: ActiveHarnessIntegrationResult["outcome"],
  message: string,
  diagnosticCode: string,
): ActiveHarnessIntegrationResult {
  return {
    operation,
    outcome,
    registration: "error",
    activation: "unknown",
    coverage: "none",
    diagnosticCode,
    message,
  };
}

function failedMutationResult(
  operation: ActiveHarnessIntegrationResult["operation"],
  statusResult: IntegrationStatusResult,
  message: string,
  diagnosticCode = MUTATION_FAILED_CODE,
): ActiveHarnessIntegrationResult {
  if (!statusResult.ok) {
    return fallbackIntegrationResult(operation, "failed", message, diagnosticCode);
  }
  return integrationResultFromStatus(operation, "failed", statusResult.status, {
    diagnosticCode,
    message,
  });
}

function shouldRepair(status: IntegrationStatus): boolean {
  if (status.registration === "drifted" || status.registration === "error") return true;
  if (status.registration !== "installed") return false;

  return status.diagnostics.some(
    (diagnostic) => diagnostic.code !== "tool_not_detected" && diagnostic.code !== "needs_trust",
  );
}

function planMutation(enabled: boolean, statusResult: IntegrationStatusResult): PlannedMutation {
  if (!statusResult.ok) return { operation: "skipped", mutate: false };

  const status = statusResult.status;
  if (status.registration === "unsupported") return { operation: "skipped", mutate: false };

  if (enabled) {
    if (status.ownership === "ambiguous") return { operation: "skipped", mutate: false };
    if (status.activation === "needs_trust") return { operation: "skipped", mutate: false };
    if (status.registration === "absent") return { operation: "install", mutate: true };
    if (shouldRepair(status)) return { operation: "repair", mutate: true };
    return { operation: "skipped", mutate: false };
  }

  if (status.ownership === "ambiguous" && status.registration !== "absent") {
    return { operation: "skipped", mutate: false };
  }
  if (status.ownership === "ork_works" && status.registration !== "absent") {
    return { operation: "uninstall", mutate: true };
  }
  return { operation: "skipped", mutate: false };
}

function noMutationResult(
  statusResult: IntegrationStatusResult,
): ActiveHarnessIntegrationResult {
  if (!statusResult.ok) {
    return fallbackIntegrationResult("skipped", "failed", statusResult.error, STATUS_UNAVAILABLE_CODE);
  }

  const status = statusResult.status;
  if (status.registration === "unsupported") {
    return integrationResultFromStatus("skipped", "unsupported", status);
  }
  if (status.ownership === "ambiguous") {
    return integrationResultFromStatus("skipped", "failed", status, {
      diagnosticCode: OWNERSHIP_AMBIGUOUS_CODE,
      message: status.diagnostics[0]?.message
        ?? "OrkWorks cannot safely change the existing integration in this workspace.",
    });
  }

  return integrationResultFromStatus("skipped", "succeeded", status);
}

function isStale(initial: WorkspaceGuardSnapshot, current: WorkspaceGuardSnapshot): boolean {
  return initial.workspacePath !== current.workspacePath || initial.generation !== current.generation;
}

function staleWorkspaceResult(
  harnessIds: readonly string[],
  results: Record<string, ActiveHarnessIntegrationResult>,
  planned: ReadonlyMap<string, ActiveHarnessIntegrationResult["operation"]>,
  latestStatus: ReadonlyMap<string, IntegrationStatus>,
): ActiveHarnessSaveResult {
  const integrations: Record<string, ActiveHarnessIntegrationResult> = {};

  for (const harnessId of harnessIds) {
    const existing = results[harnessId];
    if (existing) {
      integrations[harnessId] = {
        ...existing,
        outcome: "stale_workspace",
        diagnosticCode: STALE_WORKSPACE_CODE,
        message: STALE_WORKSPACE_MESSAGE,
      };
      continue;
    }

    const status = latestStatus.get(harnessId);
    const operation = planned.get(harnessId) ?? "skipped";
    integrations[harnessId] = status
      ? integrationResultFromStatus(operation, "stale_workspace", status, {
        diagnosticCode: STALE_WORKSPACE_CODE,
        message: STALE_WORKSPACE_MESSAGE,
      })
      : fallbackIntegrationResult(operation, "stale_workspace", STALE_WORKSPACE_MESSAGE, STALE_WORKSPACE_CODE);
  }

  return {
    activeHarnesses: {
      outcome: "stale_workspace",
      message: STALE_WORKSPACE_MESSAGE,
    },
    integrations,
  };
}

export async function saveActiveHarnessesWithIntegrations(
  ids: string[],
  deps: ActiveHarnessIntegrationDeps,
): Promise<ActiveHarnessSaveResult> {
  const initialGuard = deps.captureWorkspaceGuard();
  const activeIds = new Set(ids);
  const persisted = await deps.persistActiveHarnesses(ids);

  if (isStale(initialGuard, deps.captureWorkspaceGuard())) {
    return {
      activeHarnesses: {
        outcome: "stale_workspace",
        message: STALE_WORKSPACE_MESSAGE,
      },
      integrations: {},
    };
  }

  if (!persisted.ok) {
    return {
      activeHarnesses: {
        outcome: "failed",
        message: persisted.error,
      },
      integrations: {},
    };
  }

  const harnesses = selectableHarnesses(await deps.listHarnesses());
  const harnessIds = harnesses.map((harness) => harness.id);

  if (isStale(initialGuard, deps.captureWorkspaceGuard())) {
    return staleWorkspaceResult(harnessIds, {}, new Map(), new Map());
  }

  const results: Record<string, ActiveHarnessIntegrationResult> = {};
  const latestStatus = new Map<string, IntegrationStatus>();
  const plannedOperations = new Map<string, ActiveHarnessIntegrationResult["operation"]>();

  for (const harness of harnesses) {
    const statusResult = await deps.getIntegrationStatus(harness.id);
    if (statusResult.ok) latestStatus.set(harness.id, statusResult.status);

    const plan = planMutation(activeIds.has(harness.id), statusResult);
    plannedOperations.set(harness.id, plan.operation);

    if (!plan.mutate) {
      results[harness.id] = noMutationResult(statusResult);
      continue;
    }

    if (isStale(initialGuard, deps.captureWorkspaceGuard())) {
      return staleWorkspaceResult(harnessIds, results, plannedOperations, latestStatus);
    }

    const mutationResult = plan.operation === "uninstall"
      ? await deps.uninstallIntegration(harness.id)
      : await deps.installIntegration(harness.id);

    if (isStale(initialGuard, deps.captureWorkspaceGuard())) {
      return staleWorkspaceResult(harnessIds, results, plannedOperations, latestStatus);
    }

    if (!mutationResult.ok) {
      results[harness.id] = failedMutationResult(plan.operation, statusResult, mutationResult.error);
      continue;
    }

    latestStatus.set(harness.id, mutationResult.status);
    results[harness.id] = integrationResultFromStatus(
      plan.operation,
      mutationResult.status.registration === "unsupported" ? "unsupported" : "succeeded",
      mutationResult.status,
    );
  }

  if (isStale(initialGuard, deps.captureWorkspaceGuard())) {
    return staleWorkspaceResult(harnessIds, results, plannedOperations, latestStatus);
  }

  return {
    activeHarnesses: {
      outcome: "persisted",
    },
    integrations: results,
  };
}
