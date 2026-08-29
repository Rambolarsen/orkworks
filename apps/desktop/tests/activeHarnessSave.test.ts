import test from "node:test";
import assert from "node:assert/strict";

import {
  saveActiveHarnessesWithIntegrations,
  type ActiveHarnessIntegrationDeps,
  type ElectronHarnessConfig,
  type IntegrationStatus,
  type IntegrationStatusResult,
} from "../electron/activeHarnessIntegration.ts";

function harness(id: string): ElectronHarnessConfig {
  return {
    id,
    name: id,
    retired: false,
    launch: { kind: "command-template", command: id, args: [], modelPrefix: null },
    integration: null,
  };
}

function status(overrides: Partial<IntegrationStatus> = {}): IntegrationStatusResult {
  return {
    ok: true,
    status: {
      harnessId: "codex",
      enabled: true,
      toolDetected: true,
      registration: "installed",
      ownership: "ork_works",
      activation: "active",
      coverage: "full",
      diagnostics: [],
      confirmation: null,
      ...overrides,
    },
  };
}

function createDeps(
  overrides: Partial<ActiveHarnessIntegrationDeps> = {},
): ActiveHarnessIntegrationDeps & {
  calls: string[];
  setGuard: (next: { workspacePath: string | null; generation: number }) => void;
} {
  const calls: string[] = [];
  let guard = { workspacePath: "/repo", generation: 1 };

  return {
    calls,
    setGuard: (next) => {
      guard = next;
    },
    captureWorkspaceGuard: () => guard,
    persistActiveHarnesses: async () => {
      calls.push("persist");
      return { ok: true };
    },
    listHarnesses: async () => {
      calls.push("list");
      return [];
    },
    getIntegrationStatus: async (harnessId) => {
      calls.push(`status:${harnessId}`);
      return status({ harnessId });
    },
    installIntegration: async (harnessId) => {
      calls.push(`install:${harnessId}`);
      return status({ harnessId });
    },
    uninstallIntegration: async (harnessId) => {
      calls.push(`uninstall:${harnessId}`);
      return status({
        harnessId,
        registration: "absent",
        ownership: "none",
        activation: "disabled",
      });
    },
    ...overrides,
  };
}

test("active persistence failure prevents every integration mutation", async () => {
  const deps = createDeps({
    persistActiveHarnesses: async () => {
      deps.calls.push("persist");
      return { ok: false, error: "disk full" };
    },
    listHarnesses: async () => {
      deps.calls.push("list");
      return [harness("claude-code")];
    },
  });

  const result = await saveActiveHarnessesWithIntegrations(["claude-code"], deps);

  assert.deepEqual(result, {
    activeHarnesses: { outcome: "failed", message: "disk full" },
    integrations: {},
  });
  assert.deepEqual(deps.calls, ["persist"]);
});

test("orchestrates per-tool install, repair, uninstall, unsupported skip, and isolated failures", async () => {
  const deps = createDeps({
    listHarnesses: async () => {
      deps.calls.push("list");
      return [
        harness("claude-code"),
        harness("codex"),
        harness("copilot"),
        harness("antigravity"),
        harness("opencode"),
        harness("generic-shell"),
      ];
    },
    getIntegrationStatus: async (harnessId) => {
      deps.calls.push(`status:${harnessId}`);
      switch (harnessId) {
        case "claude-code":
          return status({
            harnessId,
            registration: "absent",
            ownership: "none",
            activation: "unknown",
          });
        case "codex":
          return status({
            harnessId,
            registration: "installed",
            activation: "active",
            diagnostics: [{ code: "needs_repair", message: "Repair the hook." }],
          });
        case "copilot":
          return status({
            harnessId,
            registration: "installed",
            ownership: "ork_works",
            activation: "disabled",
          });
        case "antigravity":
          return status({
            harnessId,
            registration: "unsupported",
            ownership: "none",
            activation: "not_applicable",
            coverage: "none",
          });
        case "opencode":
          return status({
            harnessId,
            registration: "absent",
            ownership: "none",
            activation: "unknown",
          });
        default:
          throw new Error(`unexpected harness ${harnessId}`);
      }
    },
    installIntegration: async (harnessId) => {
      deps.calls.push(`install:${harnessId}`);
      if (harnessId === "claude-code") {
        return status({ harnessId, registration: "installed", activation: "active" });
      }
      if (harnessId === "codex") {
        return status({
          harnessId,
          registration: "installed",
          activation: "needs_trust",
          diagnostics: [{ code: "needs_trust", message: "Approve the hook in Codex." }],
        });
      }
      return { ok: false, error: "permission denied" };
    },
    uninstallIntegration: async (harnessId) => {
      deps.calls.push(`uninstall:${harnessId}`);
      return status({
        harnessId,
        registration: "absent",
        ownership: "none",
        activation: "disabled",
      });
    },
  });

  const result = await saveActiveHarnessesWithIntegrations(
    ["claude-code", "codex", "antigravity", "opencode"],
    deps,
  );

  assert.deepEqual(deps.calls, [
    "persist",
    "list",
    "status:claude-code",
    "install:claude-code",
    "status:codex",
    "install:codex",
    "status:copilot",
    "uninstall:copilot",
    "status:antigravity",
    "status:opencode",
    "install:opencode",
  ]);
  assert.deepEqual(result.activeHarnesses, { outcome: "persisted" });
  assert.deepEqual(result.integrations["claude-code"], {
    operation: "install",
    outcome: "succeeded",
    registration: "installed",
    activation: "active",
    coverage: "full",
  });
  assert.deepEqual(result.integrations.codex, {
    operation: "repair",
    outcome: "succeeded",
    registration: "installed",
    activation: "needs_trust",
    coverage: "full",
    diagnosticCode: "needs_trust",
    message: "Approve the hook in Codex.",
  });
  assert.deepEqual(result.integrations.copilot, {
    operation: "uninstall",
    outcome: "succeeded",
    registration: "absent",
    activation: "disabled",
    coverage: "full",
  });
  assert.deepEqual(result.integrations.antigravity, {
    operation: "skipped",
    outcome: "unsupported",
    registration: "unsupported",
    activation: "not_applicable",
    coverage: "none",
  });
  assert.deepEqual(result.integrations.opencode, {
    operation: "install",
    outcome: "failed",
    registration: "absent",
    activation: "unknown",
    coverage: "full",
    diagnosticCode: "mutation_failed",
    message: "permission denied",
  });
});

test("workspace switches abort the batch and erase old-workspace successes", async () => {
  const deps = createDeps({
    listHarnesses: async () => {
      deps.calls.push("list");
      return [harness("claude-code"), harness("copilot")];
    },
    getIntegrationStatus: async (harnessId) => {
      deps.calls.push(`status:${harnessId}`);
      if (harnessId === "claude-code") {
        return status({
          harnessId,
          registration: "absent",
          ownership: "none",
          activation: "unknown",
        });
      }
      return status({
        harnessId,
        registration: "installed",
        ownership: "ork_works",
        activation: "disabled",
      });
    },
    installIntegration: async (harnessId) => {
      deps.calls.push(`install:${harnessId}`);
      deps.setGuard({ workspacePath: "/other", generation: 2 });
      return status({ harnessId, registration: "installed", activation: "active" });
    },
    uninstallIntegration: async (harnessId) => {
      deps.calls.push(`uninstall:${harnessId}`);
      return status({
        harnessId,
        registration: "absent",
        ownership: "none",
        activation: "disabled",
      });
    },
  });

  const result = await saveActiveHarnessesWithIntegrations(["claude-code"], deps);

  assert.equal(result.activeHarnesses.outcome, "stale_workspace");
  assert.equal(result.integrations["claude-code"]?.outcome, "stale_workspace");
  assert.equal(result.integrations.copilot?.outcome, "stale_workspace");
  assert.deepEqual(deps.calls, [
    "persist",
    "list",
    "status:claude-code",
    "install:claude-code",
  ]);
});
