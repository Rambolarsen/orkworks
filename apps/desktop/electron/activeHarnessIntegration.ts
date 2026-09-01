export type HarnessOrigin = "builtin" | "override" | "custom";

export interface IntegrationKey {
  adapterId: string;
  targetId: string;
}

export interface IntegrationConsumer {
  harnessId: string;
  harnessName: string;
}

export interface ElectronHarnessConfig {
  id: string;
  name: string;
  retired: boolean;
  launch:
    | { kind: "command-template"; command: string; args: string[]; modelPrefix: string | null }
    | { kind: "platform-shell"; login: boolean };
  defaultModel?: string | null;
  resume?: unknown;
  models?: unknown;
  peon?: unknown;
  capacity?: unknown;
  sessionSignals?: unknown;
  integration: unknown;
  voice?: unknown;
  origin: HarnessOrigin;
  profile?: string | null;
}

export interface HarnessSnapshot {
  documentRevision: string | null;
  harnesses: ElectronHarnessConfig[];
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

export interface GroupedIntegrationStatus {
  key: IntegrationKey;
  consumers: IntegrationConsumer[];
  status: IntegrationStatus;
}

export type IntegrationStatusResult =
  | { ok: true; status: IntegrationStatus }
  | { ok: false; error: string; code?: string };

export type GroupedIntegrationStatusResult =
  | { ok: true; group: GroupedIntegrationStatus }
  | { ok: false; error: string; code?: string };

export interface IntegrationRevisionExpectation {
  expectedDocumentRevision: string | null;
  expectedActiveHarnessRevision: number;
}

export interface ActiveHarnessIntegrationResult {
  key: IntegrationKey;
  consumerHarnessIds: string[];
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
  activeHarnessRevision: number;
}

export interface PlannedIntegrationMutation {
  key: IntegrationKey;
  consumerHarnessIds: string[];
  consumerHarnessNames: string[];
  operation: ActiveHarnessIntegrationResult["operation"];
  confirmation: IntegrationConfirmation | null;
}

export interface ActiveHarnessIntegrationDeps {
  captureWorkspaceGuard(): WorkspaceGuardSnapshot;
  persistActiveHarnesses(
    ids: string[],
    expectedActiveHarnessRevision: number,
  ): Promise<
    | { ok: true; activeHarnessRevision: number }
    | { ok: false; error: string; code?: string }
  >;
  listHarnesses(): Promise<HarnessSnapshot>;
  getGroupedIntegrationStatus(key: IntegrationKey): Promise<GroupedIntegrationStatusResult>;
  installGroupedIntegration(
    key: IntegrationKey,
    expected: IntegrationRevisionExpectation,
  ): Promise<GroupedIntegrationStatusResult>;
  uninstallGroupedIntegration(
    key: IntegrationKey,
    expected: IntegrationRevisionExpectation,
  ): Promise<GroupedIntegrationStatusResult>;
  repairGroupedIntegration(
    key: IntegrationKey,
    expected: IntegrationRevisionExpectation,
  ): Promise<GroupedIntegrationStatusResult>;
  /**
   * Electron-main confirmation, required by specs/orkworks-mvp.md before any
   * install/repair/uninstall mutation. Called at most once per save, with
   * every planned mutation batched together.
   */
  confirmMutations(planned: PlannedIntegrationMutation[]): Promise<boolean>;
}

const WORKSPACE_TARGET_ID = "workspace";
const STALE_WORKSPACE_MESSAGE = "Workspace changed while saving coding tools. Reload the current workspace and retry.";
const STATUS_UNAVAILABLE_CODE = "status_unavailable";
const MUTATION_FAILED_CODE = "mutation_failed";
const OWNERSHIP_AMBIGUOUS_CODE = "ownership_ambiguous";
const STALE_WORKSPACE_CODE = "stale_workspace";
const CONFIRMATION_DECLINED_CODE = "confirmation_declined";
const CONFIRMATION_DECLINED_MESSAGE = "Declined the confirmation prompt.";

interface PlannedMutation {
  operation: ActiveHarnessIntegrationResult["operation"];
  mutate: boolean;
}

interface IntegrationGroup {
  key: IntegrationKey;
  consumers: IntegrationConsumer[];
}

export function integrationKeyId(key: IntegrationKey): string {
  return `${key.adapterId}/${key.targetId}`;
}

function integrationKeyForHarness(harness: ElectronHarnessConfig): IntegrationKey | null {
  if (!harness.integration || typeof harness.integration !== "object") return null;
  const kind = (harness.integration as { kind?: unknown }).kind;
  return typeof kind === "string" && kind.length > 0
    ? { adapterId: kind, targetId: WORKSPACE_TARGET_ID }
    : null;
}

function selectableHarnesses(harnesses: ElectronHarnessConfig[]): ElectronHarnessConfig[] {
  return harnesses.filter((harness) => !harness.retired);
}

function groupHarnesses(harnesses: ElectronHarnessConfig[]): IntegrationGroup[] {
  const groups = new Map<string, IntegrationGroup>();
  for (const harness of selectableHarnesses(harnesses)) {
    const key = integrationKeyForHarness(harness);
    if (!key) continue;
    const id = integrationKeyId(key);
    const group = groups.get(id) ?? { key, consumers: [] };
    group.consumers.push({ harnessId: harness.id, harnessName: harness.name });
    groups.set(id, group);
  }
  return [...groups.values()];
}

function resultForGroup(
  group: IntegrationGroup,
  operation: ActiveHarnessIntegrationResult["operation"],
  outcome: ActiveHarnessIntegrationResult["outcome"],
  status: IntegrationStatus,
  overrides: Partial<Pick<ActiveHarnessIntegrationResult, "diagnosticCode" | "message">> = {},
): ActiveHarnessIntegrationResult {
  const firstDiagnostic = status.diagnostics[0];
  const diagnosticCode = overrides.diagnosticCode ?? firstDiagnostic?.code;
  const message = overrides.message ?? firstDiagnostic?.message;
  return {
    key: group.key,
    consumerHarnessIds: group.consumers.map((consumer) => consumer.harnessId),
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
  group: IntegrationGroup,
  operation: ActiveHarnessIntegrationResult["operation"],
  outcome: ActiveHarnessIntegrationResult["outcome"],
  message: string,
  diagnosticCode: string,
): ActiveHarnessIntegrationResult {
  return {
    key: group.key,
    consumerHarnessIds: group.consumers.map((consumer) => consumer.harnessId),
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
  group: IntegrationGroup,
  operation: ActiveHarnessIntegrationResult["operation"],
  statusResult: GroupedIntegrationStatusResult,
  message: string,
  diagnosticCode = MUTATION_FAILED_CODE,
): ActiveHarnessIntegrationResult {
  if (!statusResult.ok) {
    return fallbackIntegrationResult(group, operation, "failed", message, diagnosticCode);
  }
  return resultForGroup(group, operation, "failed", statusResult.group.status, {
    diagnosticCode,
    message,
  });
}

function shouldRepair(status: IntegrationStatus): boolean {
  if (status.registration === "drifted" || status.registration === "error") return true;
  if (status.registration !== "installed") return false;

  return status.diagnostics.some(
    (diagnostic) =>
      diagnostic.code !== "tool_not_detected"
      && diagnostic.code !== "needs_trust"
      && diagnostic.code !== "unsupported_tool_version",
  );
}

function planMutation(enabled: boolean, statusResult: GroupedIntegrationStatusResult): PlannedMutation {
  if (!statusResult.ok) return { operation: "skipped", mutate: false };

  const status = statusResult.group.status;
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
  group: IntegrationGroup,
  statusResult: GroupedIntegrationStatusResult,
): ActiveHarnessIntegrationResult {
  if (!statusResult.ok) {
    return fallbackIntegrationResult(group, "skipped", "failed", statusResult.error, STATUS_UNAVAILABLE_CODE);
  }

  const status = statusResult.group.status;
  if (status.registration === "unsupported") {
    return resultForGroup(group, "skipped", "unsupported", status);
  }
  if (status.ownership === "ambiguous") {
    return resultForGroup(group, "skipped", "failed", status, {
      diagnosticCode: OWNERSHIP_AMBIGUOUS_CODE,
      message: status.diagnostics[0]?.message
        ?? "OrkWorks cannot safely change the existing integration in this workspace.",
    });
  }

  return resultForGroup(group, "skipped", "succeeded", status);
}

export function isStale(initial: WorkspaceGuardSnapshot, current: WorkspaceGuardSnapshot): boolean {
  return initial.workspacePath !== current.workspacePath || initial.generation !== current.generation;
}

function staleWorkspaceResult(
  groups: readonly IntegrationGroup[],
  results: Record<string, ActiveHarnessIntegrationResult>,
  planned: ReadonlyMap<string, ActiveHarnessIntegrationResult["operation"]>,
  latestStatus: ReadonlyMap<string, IntegrationStatus>,
): ActiveHarnessSaveResult {
  const integrations: Record<string, ActiveHarnessIntegrationResult> = {};

  for (const group of groups) {
    const id = integrationKeyId(group.key);
    const existing = results[id];
    if (existing) {
      integrations[id] = {
        ...existing,
        outcome: "stale_workspace",
        diagnosticCode: STALE_WORKSPACE_CODE,
        message: STALE_WORKSPACE_MESSAGE,
      };
      continue;
    }

    const status = latestStatus.get(id);
    const operation = planned.get(id) ?? "skipped";
    integrations[id] = status
      ? resultForGroup(group, operation, "stale_workspace", status, {
        diagnosticCode: STALE_WORKSPACE_CODE,
        message: STALE_WORKSPACE_MESSAGE,
      })
      : fallbackIntegrationResult(group, operation, "stale_workspace", STALE_WORKSPACE_MESSAGE, STALE_WORKSPACE_CODE);
  }

  return {
    activeHarnesses: {
      outcome: "stale_workspace",
      message: STALE_WORKSPACE_MESSAGE,
    },
    integrations,
  };
}

function activeSelectionStaleResult(): ActiveHarnessSaveResult {
  return {
    activeHarnesses: {
      outcome: "stale_workspace",
      message: "The active coding tools changed while saving. Reload the current workspace and retry.",
    },
    integrations: {},
  };
}

export async function saveActiveHarnessesWithIntegrations(
  ids: string[],
  deps: ActiveHarnessIntegrationDeps,
): Promise<ActiveHarnessSaveResult> {
  const initialGuard = deps.captureWorkspaceGuard();
  const activeIds = new Set(ids);
  const persisted = await deps.persistActiveHarnesses(ids, initialGuard.activeHarnessRevision);

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
    return persisted.code === "active_harness_revision_changed"
      ? activeSelectionStaleResult()
      : {
        activeHarnesses: {
          outcome: "failed",
          message: persisted.error,
        },
        integrations: {},
      };
  }

  let snapshot: HarnessSnapshot;
  try {
    snapshot = await deps.listHarnesses();
  } catch {
    // The active selection is durable even when status reconciliation cannot
    // begin. Keep the result explicit; the next Save can retry the groups.
    return {
      activeHarnesses: { outcome: "persisted" },
      integrations: {},
    };
  }

  const groups = groupHarnesses(snapshot.harnesses);
  if (isStale(initialGuard, deps.captureWorkspaceGuard())) {
    return staleWorkspaceResult(groups, {}, new Map(), new Map());
  }

  const results: Record<string, ActiveHarnessIntegrationResult> = {};
  const latestStatus = new Map<string, IntegrationStatus>();
  const plannedOperations = new Map<string, ActiveHarnessIntegrationResult["operation"]>();
  const statusByGroup = new Map<string, GroupedIntegrationStatusResult>();
  const toMutate: { group: IntegrationGroup; operation: ActiveHarnessIntegrationResult["operation"] }[] = [];

  for (const group of groups) {
    const id = integrationKeyId(group.key);
    let statusResult: GroupedIntegrationStatusResult;
    try {
      statusResult = await deps.getGroupedIntegrationStatus(group.key);
    } catch (error) {
      statusResult = {
        ok: false,
        error: error instanceof Error ? error.message : "Couldn't read integration status.",
      };
    }
    if (statusResult.ok) latestStatus.set(id, statusResult.group.status);
    statusByGroup.set(id, statusResult);

    const enabled = group.consumers.some((consumer) => activeIds.has(consumer.harnessId));
    const plan = planMutation(enabled, statusResult);
    plannedOperations.set(id, plan.operation);

    if (!plan.mutate) {
      results[id] = noMutationResult(group, statusResult);
      continue;
    }

    if (isStale(initialGuard, deps.captureWorkspaceGuard())) {
      return staleWorkspaceResult(groups, results, plannedOperations, latestStatus);
    }

    toMutate.push({ group, operation: plan.operation });
  }

  const expected: IntegrationRevisionExpectation = {
    expectedDocumentRevision: snapshot.documentRevision,
    expectedActiveHarnessRevision: persisted.activeHarnessRevision,
  };

  if (toMutate.length > 0) {
    const confirmed = await deps.confirmMutations(
      toMutate.map(({ group, operation }) => {
        const statusResult = statusByGroup.get(integrationKeyId(group.key));
        return {
          key: group.key,
          consumerHarnessIds: group.consumers.map((consumer) => consumer.harnessId),
          consumerHarnessNames: group.consumers.map((consumer) => consumer.harnessName),
          operation,
          confirmation: statusResult?.ok ? statusResult.group.status.confirmation : null,
        };
      }),
    );

    if (isStale(initialGuard, deps.captureWorkspaceGuard())) {
      return staleWorkspaceResult(groups, results, plannedOperations, latestStatus);
    }

    if (!confirmed) {
      for (const { group, operation } of toMutate) {
        const id = integrationKeyId(group.key);
        results[id] = failedMutationResult(
          group,
          operation,
          statusByGroup.get(id)!,
          CONFIRMATION_DECLINED_MESSAGE,
          CONFIRMATION_DECLINED_CODE,
        );
      }
      return {
        activeHarnesses: { outcome: "persisted" },
        integrations: results,
      };
    }

    for (const { group, operation } of toMutate) {
      const id = integrationKeyId(group.key);
      if (isStale(initialGuard, deps.captureWorkspaceGuard())) {
        return staleWorkspaceResult(groups, results, plannedOperations, latestStatus);
      }

      let mutationResult: GroupedIntegrationStatusResult;
      try {
        mutationResult = operation === "uninstall"
          ? await deps.uninstallGroupedIntegration(group.key, expected)
          : operation === "repair"
            ? await deps.repairGroupedIntegration(group.key, expected)
            : await deps.installGroupedIntegration(group.key, expected);
      } catch (error) {
        mutationResult = {
          ok: false,
          error: error instanceof Error ? error.message : "Couldn't update the integration.",
        };
      }

      if (isStale(initialGuard, deps.captureWorkspaceGuard())) {
        return staleWorkspaceResult(groups, results, plannedOperations, latestStatus);
      }

      if (!mutationResult.ok) {
        if (mutationResult.code === "integration_revision_changed") {
          return staleWorkspaceResult(groups, results, plannedOperations, latestStatus);
        }
        results[id] = failedMutationResult(group, operation, statusByGroup.get(id)!, mutationResult.error);
        continue;
      }

      latestStatus.set(id, mutationResult.group.status);
      results[id] = resultForGroup(
        group,
        operation,
        mutationResult.group.status.registration === "unsupported" ? "unsupported" : "succeeded",
        mutationResult.group.status,
      );
    }
  }

  if (isStale(initialGuard, deps.captureWorkspaceGuard())) {
    return staleWorkspaceResult(groups, results, plannedOperations, latestStatus);
  }

  return {
    activeHarnesses: {
      outcome: "persisted",
    },
    integrations: results,
  };
}
