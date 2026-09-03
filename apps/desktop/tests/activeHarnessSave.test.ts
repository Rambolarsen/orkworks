import test from "node:test";
import assert from "node:assert/strict";

import {
  integrationKeyId,
  reconcileGroupedIntegration,
  saveActiveHarnessesWithIntegrations,
  type ActiveHarnessIntegrationDeps,
  type ElectronHarnessConfig,
  type GroupedIntegrationStatusResult,
  type IntegrationKey,
  type IntegrationStatus,
  type PlannedIntegrationMutation,
} from "../electron/activeHarnessIntegration.ts";

function harness(id: string, integration: string | null = "copilot"): ElectronHarnessConfig {
  return {
    id,
    name: id,
    retired: false,
    launch: { kind: "command-template", command: id, args: [], modelPrefix: null },
    integration: integration ? { kind: integration } : null,
    origin: "builtin",
  };
}

function status(overrides: Partial<IntegrationStatus> = {}): IntegrationStatus {
  return {
    harnessId: "copilot",
    enabled: true,
    toolDetected: true,
    registration: "installed",
    ownership: "ork_works",
    activation: "active",
    coverage: "full",
    diagnostics: [],
    confirmation: null,
    ...overrides,
  };
}

function grouped(
  key: IntegrationKey,
  consumers: Array<{ harnessId: string; harnessName?: string }>,
  overrides: Partial<IntegrationStatus> = {},
): GroupedIntegrationStatusResult {
  return {
    ok: true,
    group: {
      key,
      consumers: consumers.map((consumer) => ({
        harnessId: consumer.harnessId,
        harnessName: consumer.harnessName ?? consumer.harnessId,
      })),
      status: status({ harnessId: key.adapterId, ...overrides }),
    },
  };
}

function createDeps(
  overrides: Partial<ActiveHarnessIntegrationDeps> = {},
): ActiveHarnessIntegrationDeps & {
  calls: string[];
  setGuard: (next: { workspacePath: string | null; generation: number; activeHarnessRevision: number }) => void;
} {
  const calls: string[] = [];
  let guard = { workspacePath: "/repo", generation: 1, activeHarnessRevision: 7 };
  const defaultKey: IntegrationKey = { adapterId: "copilot", targetId: "workspace" };

  return {
    calls,
    setGuard: (next) => {
      guard = next;
    },
    captureWorkspaceGuard: () => guard,
    persistActiveHarnesses: async (_ids, expectedActiveHarnessRevision) => {
      calls.push(`persist:${expectedActiveHarnessRevision}`);
      return { ok: true, activeHarnessRevision: expectedActiveHarnessRevision + 1 };
    },
    listHarnesses: async () => {
      calls.push("list");
      return { documentRevision: "doc-1", harnesses: [] };
    },
    getGroupedIntegrationStatus: async (key) => {
      calls.push(`status:${integrationKeyId(key)}`);
      return grouped(key, []);
    },
    installGroupedIntegration: async (key, expected) => {
      calls.push(`install:${integrationKeyId(key)}:${expected.expectedDocumentRevision}:${expected.expectedActiveHarnessRevision}`);
      return grouped(key, []);
    },
    repairGroupedIntegration: async (key, expected) => {
      calls.push(`repair:${integrationKeyId(key)}:${expected.expectedDocumentRevision}:${expected.expectedActiveHarnessRevision}`);
      return grouped(key, []);
    },
    uninstallGroupedIntegration: async (key, expected) => {
      calls.push(`uninstall:${integrationKeyId(key)}:${expected.expectedDocumentRevision}:${expected.expectedActiveHarnessRevision}`);
      return grouped(key, [], { registration: "absent", ownership: "none", activation: "disabled" });
    },
    confirmMutations: async () => true,
    ...overrides,
  };
}

test("active selection persistence sends and returns the active revision", async () => {
  const deps = createDeps({
    persistActiveHarnesses: async (_ids, revision) => {
      deps.calls.push(`persist:${revision}`);
      return { ok: true, activeHarnessRevision: 19 };
    },
  });

  const result = await saveActiveHarnessesWithIntegrations(["copilot"], deps);

  assert.deepEqual(result.activeHarnesses, { outcome: "persisted" });
  assert.deepEqual(deps.calls, ["persist:7", "list"]);
});

test("two Copilot-compatible consumers share one status read, confirmation, and install", async () => {
  const confirmations: PlannedIntegrationMutation[][] = [];
  const deps = createDeps({
    listHarnesses: async () => {
      deps.calls.push("list");
      return {
        documentRevision: "doc-42",
        harnesses: [harness("copilot"), { ...harness("copilot-local"), origin: "custom" }],
      };
    },
    getGroupedIntegrationStatus: async (key) => {
      deps.calls.push(`status:${integrationKeyId(key)}`);
      return grouped(key, [
        { harnessId: "copilot", harnessName: "Copilot" },
        { harnessId: "copilot-local", harnessName: "Copilot Local" },
      ], { registration: "absent", ownership: "none", activation: "unknown" });
    },
    installGroupedIntegration: async (key, expected) => {
      deps.calls.push(`install:${integrationKeyId(key)}:${expected.expectedDocumentRevision}:${expected.expectedActiveHarnessRevision}`);
      return grouped(key, [
        { harnessId: "copilot", harnessName: "Copilot" },
        { harnessId: "copilot-local", harnessName: "Copilot Local" },
      ]);
    },
    confirmMutations: async (planned) => {
      deps.calls.push("confirm");
      confirmations.push(planned);
      return true;
    },
  });

  const result = await saveActiveHarnessesWithIntegrations(["copilot-local"], deps);
  const key = "copilot/workspace";

  assert.deepEqual(deps.calls, [
    "persist:7",
    "list",
    "status:copilot/workspace",
    "confirm",
    "install:copilot/workspace:doc-42:8",
  ]);
  assert.deepEqual(confirmations[0], [{
    key: { adapterId: "copilot", targetId: "workspace" },
    consumerHarnessIds: ["copilot", "copilot-local"],
    consumerHarnessNames: ["copilot", "copilot-local"],
    operation: "install",
    confirmation: null,
  }]);
  assert.deepEqual(result.integrations[key], {
    key: { adapterId: "copilot", targetId: "workspace" },
    consumerHarnessIds: ["copilot", "copilot-local"],
    operation: "install",
    outcome: "succeeded",
    registration: "installed",
    activation: "active",
    coverage: "full",
  });
});

test("disabling one shared consumer does not uninstall the adapter, but disabling both does", async () => {
  let activeIds = ["copilot"];
  const deps = createDeps({
    listHarnesses: async () => ({
      documentRevision: "doc-1",
      harnesses: [harness("copilot"), { ...harness("copilot-local"), origin: "custom" }],
    }),
    getGroupedIntegrationStatus: async (key) => grouped(key, [
      { harnessId: "copilot", harnessName: "Copilot" },
      { harnessId: "copilot-local", harnessName: "Copilot Local" },
    ]),
    uninstallGroupedIntegration: async (key, expected) => {
      deps.calls.push(`uninstall:${integrationKeyId(key)}:${expected.expectedActiveHarnessRevision}`);
      return grouped(key, [], { registration: "absent", ownership: "none", activation: "disabled" });
    },
  });

  await saveActiveHarnessesWithIntegrations(activeIds, deps);
  assert.equal(deps.calls.includes("uninstall:copilot/workspace:8"), false);

  deps.calls.length = 0;
  activeIds = [];
  await saveActiveHarnessesWithIntegrations(activeIds, deps);
  assert.equal(deps.calls.includes("uninstall:copilot/workspace:8"), true);
  assert.equal(deps.calls.filter((call) => call.startsWith("uninstall:")).length, 1);
});

test("a failed grouped mutation is isolated and reports all consumers", async () => {
  const deps = createDeps({
    listHarnesses: async () => ({
      documentRevision: "doc-1",
      harnesses: [harness("copilot"), harness("codex", "codex")],
    }),
    getGroupedIntegrationStatus: async (key) => grouped(key, [{ harnessId: key.adapterId, harnessName: key.adapterId }], {
      registration: "absent",
      ownership: "none",
      activation: "unknown",
    }),
    installGroupedIntegration: async (key) => {
      deps.calls.push(`install:${integrationKeyId(key)}`);
      return key.adapterId === "copilot"
        ? grouped(key, [{ harnessId: "copilot", harnessName: "copilot" }])
        : { ok: false, error: "permission denied" };
    },
  });

  const result = await saveActiveHarnessesWithIntegrations(["copilot", "codex"], deps);

  assert.equal(result.activeHarnesses.outcome, "persisted");
  assert.equal(result.integrations["copilot/workspace"]?.outcome, "succeeded");
  assert.equal(result.integrations["codex/workspace"]?.outcome, "failed");
  assert.equal(result.integrations["codex/workspace"]?.message, "permission denied");
});

test("a workspace switch while mutation is in flight marks every grouped result stale", async () => {
  const deps = createDeps({
    listHarnesses: async () => ({
      documentRevision: "doc-1",
      harnesses: [harness("copilot"), { ...harness("copilot-local"), origin: "custom" }],
    }),
    getGroupedIntegrationStatus: async (key) => grouped(key, [
      { harnessId: "copilot", harnessName: "Copilot" },
      { harnessId: "copilot-local", harnessName: "Copilot Local" },
    ], { registration: "absent", ownership: "none", activation: "unknown" }),
    installGroupedIntegration: async (key) => {
      deps.calls.push(`install:${integrationKeyId(key)}`);
      deps.setGuard({ workspacePath: "/other", generation: 2, activeHarnessRevision: 1 });
      return grouped(key, [
        { harnessId: "copilot", harnessName: "Copilot" },
        { harnessId: "copilot-local", harnessName: "Copilot Local" },
      ]);
    },
  });

  const result = await saveActiveHarnessesWithIntegrations(["copilot-local"], deps);

  assert.equal(result.activeHarnesses.outcome, "stale_workspace");
  assert.equal(result.integrations["copilot/workspace"]?.outcome, "stale_workspace");
  assert.deepEqual(result.integrations["copilot/workspace"]?.consumerHarnessIds, ["copilot", "copilot-local"]);
});

test("stale active-selection revision prevents listing or integration mutation", async () => {
  const deps = createDeps({
    persistActiveHarnesses: async (_ids, revision) => {
      deps.calls.push(`persist:${revision}`);
      return { ok: false, code: "active_harness_revision_changed", error: "selection changed" };
    },
  });

  const result = await saveActiveHarnessesWithIntegrations(["copilot"], deps);

  assert.equal(result.activeHarnesses.outcome, "stale_workspace");
  assert.deepEqual(deps.calls, ["persist:7"]);
});

test("unsupported grouped integrations remain visible without prompting", async () => {
  const deps = createDeps({
    listHarnesses: async () => ({ documentRevision: null, harnesses: [harness("generic", "generic")] }),
    getGroupedIntegrationStatus: async (key) => grouped(key, [{ harnessId: "generic", harnessName: "Generic" }], {
      registration: "unsupported",
      ownership: "none",
      activation: "not_applicable",
      coverage: "none",
    }),
    confirmMutations: async () => {
      deps.calls.push("confirm");
      return true;
    },
  });

  const result = await saveActiveHarnessesWithIntegrations(["generic"], deps);

  assert.equal(result.integrations["generic/workspace"]?.outcome, "unsupported");
  assert.equal(deps.calls.includes("confirm"), false);
});

test("reconcile repairs a drifted enabled integration without persisting the selection", async () => {
  const deps = createDeps({
    listHarnesses: async () => {
      deps.calls.push("list");
      return { documentRevision: "doc-9", harnesses: [harness("copilot")] };
    },
    getGroupedIntegrationStatus: async (key) => {
      deps.calls.push(`status:${integrationKeyId(key)}`);
      return grouped(key, [{ harnessId: "copilot", harnessName: "copilot" }], { registration: "drifted" });
    },
    confirmMutations: async () => {
      deps.calls.push("confirm");
      return true;
    },
  });

  const result = await reconcileGroupedIntegration(
    { adapterId: "copilot", targetId: "workspace" },
    new Set(["copilot"]),
    deps,
  );

  // The revision expectation comes from the captured guard (7), not a PUT
  // response — reconcile never persists the active selection.
  assert.deepEqual(deps.calls, [
    "list",
    "status:copilot/workspace",
    "confirm",
    "repair:copilot/workspace:doc-9:7",
  ]);
  assert.equal(result.operation, "repair");
  assert.equal(result.outcome, "succeeded");
  assert.deepEqual(result.consumerHarnessIds, ["copilot"]);
});

test("reconcile installs an absent integration for an enabled group", async () => {
  const deps = createDeps({
    listHarnesses: async () => ({ documentRevision: "doc-1", harnesses: [harness("copilot")] }),
    getGroupedIntegrationStatus: async (key) => grouped(key, [{ harnessId: "copilot", harnessName: "copilot" }], {
      registration: "absent",
      ownership: "none",
      activation: "unknown",
    }),
  });

  const result = await reconcileGroupedIntegration(
    { adapterId: "copilot", targetId: "workspace" },
    new Set(["copilot"]),
    deps,
  );

  assert.equal(deps.calls.includes("install:copilot/workspace:doc-1:7"), true);
  assert.equal(result.operation, "install");
  assert.equal(result.outcome, "succeeded");
});

test("reconcile uninstalls an OrkWorks-owned integration when no consumer is active", async () => {
  const deps = createDeps({
    listHarnesses: async () => ({ documentRevision: "doc-1", harnesses: [harness("copilot")] }),
    getGroupedIntegrationStatus: async (key) => grouped(key, [{ harnessId: "copilot", harnessName: "copilot" }]),
  });

  const result = await reconcileGroupedIntegration(
    { adapterId: "copilot", targetId: "workspace" },
    new Set<string>(),
    deps,
  );

  assert.equal(deps.calls.includes("uninstall:copilot/workspace:doc-1:7"), true);
  assert.equal(result.operation, "uninstall");
  assert.equal(result.outcome, "succeeded");
});

test("reconcile treats a shared group as enabled when any consumer is active", async () => {
  const deps = createDeps({
    listHarnesses: async () => ({
      documentRevision: "doc-1",
      harnesses: [harness("copilot"), { ...harness("copilot-local"), origin: "custom" }],
    }),
    getGroupedIntegrationStatus: async (key) => {
      deps.calls.push(`status:${integrationKeyId(key)}`);
      return grouped(key, [
        { harnessId: "copilot", harnessName: "Copilot" },
        { harnessId: "copilot-local", harnessName: "Copilot Local" },
      ], { registration: "drifted" });
    },
  });

  await reconcileGroupedIntegration(
    { adapterId: "copilot", targetId: "workspace" },
    new Set(["copilot-local"]),
    deps,
  );

  assert.equal(deps.calls.includes("repair:copilot/workspace:doc-1:7"), true);
});

test("reconcile reports status unavailability as a failed row instead of silently skipping", async () => {
  const deps = createDeps({
    listHarnesses: async () => {
      deps.calls.push("list");
      return { documentRevision: "doc-1", harnesses: [harness("copilot")] };
    },
    getGroupedIntegrationStatus: async (key) => {
      deps.calls.push(`status:${integrationKeyId(key)}`);
      return { ok: false, error: "route missing" };
    },
  });

  const result = await reconcileGroupedIntegration(
    { adapterId: "copilot", targetId: "workspace" },
    new Set(["copilot"]),
    deps,
  );

  assert.deepEqual(deps.calls, ["list", "status:copilot/workspace"]);
  assert.equal(result.outcome, "failed");
  assert.equal(result.operation, "skipped");
  assert.equal(result.diagnosticCode, "status_unavailable");
  assert.equal(result.message, "route missing");
});

test("reconcile reports a group that no longer exists instead of throwing", async () => {
  const deps = createDeps({
    listHarnesses: async () => {
      deps.calls.push("list");
      return { documentRevision: "doc-1", harnesses: [] };
    },
  });

  const result = await reconcileGroupedIntegration(
    { adapterId: "copilot", targetId: "workspace" },
    new Set(["copilot"]),
    deps,
  );

  assert.deepEqual(deps.calls, ["list"]);
  assert.equal(result.outcome, "failed");
  assert.equal(result.message, "The integration is no longer configured for this workspace.");
});

test("reconcile reports a stale workspace when the workspace switches mid-flow", async () => {
  const deps = createDeps({
    listHarnesses: async () => ({ documentRevision: "doc-1", harnesses: [harness("copilot")] }),
    getGroupedIntegrationStatus: async (key) => {
      deps.setGuard({ workspacePath: "/other", generation: 2, activeHarnessRevision: 1 });
      return grouped(key, [{ harnessId: "copilot", harnessName: "copilot" }], { registration: "drifted" });
    },
  });

  const result = await reconcileGroupedIntegration(
    { adapterId: "copilot", targetId: "workspace" },
    new Set(["copilot"]),
    deps,
  );

  assert.equal(result.outcome, "stale_workspace");
  assert.equal(deps.calls.includes("confirm"), false);
  assert.equal(deps.calls.some((call) => call.startsWith("repair:")), false);
});

test("reconcile reports a declined confirmation without mutating", async () => {
  const deps = createDeps({
    listHarnesses: async () => ({ documentRevision: "doc-1", harnesses: [harness("copilot")] }),
    getGroupedIntegrationStatus: async (key) => grouped(key, [{ harnessId: "copilot", harnessName: "copilot" }], {
      registration: "drifted",
    }),
    confirmMutations: async () => false,
  });

  const result = await reconcileGroupedIntegration(
    { adapterId: "copilot", targetId: "workspace" },
    new Set(["copilot"]),
    deps,
  );

  assert.equal(result.outcome, "failed");
  assert.equal(result.operation, "repair");
  assert.equal(result.diagnosticCode, "confirmation_declined");
  assert.equal(deps.calls.some((call) => call.startsWith("repair:")), false);
});

test("reconcile skips healthy integrations without prompting", async () => {
  const deps = createDeps({
    listHarnesses: async () => ({ documentRevision: "doc-1", harnesses: [harness("copilot")] }),
    getGroupedIntegrationStatus: async (key) => grouped(key, [{ harnessId: "copilot", harnessName: "copilot" }]),
    confirmMutations: async () => {
      deps.calls.push("confirm");
      return true;
    },
  });

  const result = await reconcileGroupedIntegration(
    { adapterId: "copilot", targetId: "workspace" },
    new Set(["copilot"]),
    deps,
  );

  assert.equal(result.outcome, "succeeded");
  assert.equal(result.operation, "skipped");
  assert.equal(deps.calls.includes("confirm"), false);
});

test("reconcile reports a failed mutation with the route's error message", async () => {
  const deps = createDeps({
    listHarnesses: async () => ({ documentRevision: "doc-1", harnesses: [harness("copilot")] }),
    getGroupedIntegrationStatus: async (key) => grouped(key, [{ harnessId: "copilot", harnessName: "copilot" }], {
      registration: "drifted",
    }),
    repairGroupedIntegration: async () => ({ ok: false, error: "permission denied" }),
  });

  const result = await reconcileGroupedIntegration(
    { adapterId: "copilot", targetId: "workspace" },
    new Set(["copilot"]),
    deps,
  );

  assert.equal(result.outcome, "failed");
  assert.equal(result.operation, "repair");
  assert.equal(result.message, "permission denied");
});

test("a status-unavailable group during save surfaces a failed row, not silent success", async () => {
  const deps = createDeps({
    listHarnesses: async () => ({ documentRevision: "doc-1", harnesses: [harness("copilot")] }),
    getGroupedIntegrationStatus: async (key) => {
      deps.calls.push(`status:${integrationKeyId(key)}`);
      return { ok: false, error: "grouped status route missing" };
    },
  });

  const result = await saveActiveHarnessesWithIntegrations(["copilot"], deps);

  assert.equal(result.activeHarnesses.outcome, "persisted");
  assert.equal(result.integrations["copilot/workspace"]?.outcome, "failed");
  assert.equal(result.integrations["copilot/workspace"]?.diagnosticCode, "status_unavailable");
  assert.equal(result.integrations["copilot/workspace"]?.message, "grouped status route missing");
});
