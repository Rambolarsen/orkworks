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
        case "generic-shell":
          return status({
            harnessId,
            registration: "unsupported",
            ownership: "none",
            activation: "not_applicable",
            coverage: "none",
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
    "status:generic-shell",
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
  assert.deepEqual(result.integrations["generic-shell"], {
    operation: "skipped",
    outcome: "unsupported",
    registration: "unsupported",
    activation: "not_applicable",
    coverage: "none",
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

test("retired Gemini is excluded from reconciliation and keeps its owned workspace entry untouched", async () => {
  const deps = createDeps({
    listHarnesses: async () => {
      deps.calls.push("list");
      return [
        { ...harness("gemini"), retired: true },
        harness("antigravity"),
        harness("generic-shell"),
      ];
    },
    getIntegrationStatus: async (harnessId) => {
      deps.calls.push(`status:${harnessId}`);
      if (harnessId === "gemini") throw new Error("retired Gemini must not be reconciled");
      if (harnessId === "antigravity" || harnessId === "generic-shell") {
        return status({
          harnessId,
          registration: "unsupported",
          ownership: "none",
          activation: "not_applicable",
          coverage: "none",
        });
      }
      throw new Error(`unexpected harness ${harnessId}`);
    },
    uninstallIntegration: async (harnessId) => {
      deps.calls.push(`uninstall:${harnessId}`);
      throw new Error(`unexpected uninstall:${harnessId}`);
    },
  });

  const result = await saveActiveHarnessesWithIntegrations(["antigravity"], deps);

  assert.deepEqual(deps.calls, [
    "persist",
    "list",
    "status:antigravity",
    "status:generic-shell",
  ]);
  assert.equal("gemini" in result.integrations, false);
  assert.deepEqual(result.integrations.antigravity, {
    operation: "skipped",
    outcome: "unsupported",
    registration: "unsupported",
    activation: "not_applicable",
    coverage: "none",
  });
  assert.deepEqual(result.integrations["generic-shell"], {
    operation: "skipped",
    outcome: "unsupported",
    registration: "unsupported",
    activation: "not_applicable",
    coverage: "none",
  });
});

test("an installed integration with only a version-mismatch diagnostic is not repaired", async () => {
  const deps = createDeps({
    listHarnesses: async () => {
      deps.calls.push("list");
      return [harness("opencode")];
    },
    getIntegrationStatus: async (harnessId) => {
      deps.calls.push(`status:${harnessId}`);
      return status({
        harnessId,
        registration: "installed",
        ownership: "ork_works",
        activation: "unknown",
        diagnostics: [{ code: "unsupported_tool_version", message: "The detected OpenCode version is not eligible for this integration." }],
      });
    },
    installIntegration: async (harnessId) => {
      deps.calls.push(`install:${harnessId}`);
      throw new Error(`unexpected install:${harnessId}`);
    },
    uninstallIntegration: async (harnessId) => {
      deps.calls.push(`uninstall:${harnessId}`);
      throw new Error(`unexpected uninstall:${harnessId}`);
    },
  });

  const result = await saveActiveHarnessesWithIntegrations(["opencode"], deps);

  assert.deepEqual(deps.calls, ["persist", "list", "status:opencode"]);
  assert.deepEqual(result.integrations.opencode, {
    operation: "skipped",
    outcome: "succeeded",
    registration: "installed",
    activation: "unknown",
    coverage: "full",
    diagnosticCode: "unsupported_tool_version",
    message: "The detected OpenCode version is not eligible for this integration.",
  });
});

test("a harness-listing failure after a successful persist still reports persisted, not a rejected save", async () => {
  const deps = createDeps({
    listHarnesses: async () => {
      deps.calls.push("list");
      throw new Error("backend unreachable");
    },
  });

  const result = await saveActiveHarnessesWithIntegrations(["claude-code", "codex"], deps);

  assert.deepEqual(deps.calls, ["persist", "list"]);
  assert.deepEqual(result.activeHarnesses, { outcome: "persisted" });
  assert.equal(result.integrations["claude-code"]?.outcome, "failed");
  assert.equal(result.integrations["claude-code"]?.diagnosticCode, "status_unavailable");
  assert.equal(result.integrations.codex?.outcome, "failed");
  assert.equal(result.integrations.codex?.diagnosticCode, "status_unavailable");
});

test("ambiguous ownership returns structured failure without mutating either enable or disable flows", async () => {
  const deps = createDeps({
    listHarnesses: async () => {
      deps.calls.push("list");
      return [harness("claude-code"), harness("copilot"), harness("generic-shell")];
    },
    getIntegrationStatus: async (harnessId) => {
      deps.calls.push(`status:${harnessId}`);
      if (harnessId === "claude-code") {
        return status({
          harnessId,
          registration: "installed",
          ownership: "ambiguous",
          activation: "active",
          diagnostics: [{ code: "ownership_unclear", message: "Resolve the existing hook manually." }],
        });
      }
      if (harnessId === "copilot") {
        return status({
          harnessId,
          registration: "installed",
          ownership: "ambiguous",
          activation: "disabled",
          diagnostics: [{ code: "ownership_unclear", message: "Resolve the existing hook manually." }],
        });
      }
      if (harnessId === "generic-shell") {
        return status({
          harnessId,
          registration: "unsupported",
          ownership: "none",
          activation: "not_applicable",
          coverage: "none",
        });
      }
      throw new Error(`unexpected harness ${harnessId}`);
    },
    installIntegration: async (harnessId) => {
      deps.calls.push(`install:${harnessId}`);
      throw new Error(`unexpected install:${harnessId}`);
    },
    uninstallIntegration: async (harnessId) => {
      deps.calls.push(`uninstall:${harnessId}`);
      throw new Error(`unexpected uninstall:${harnessId}`);
    },
  });

  const result = await saveActiveHarnessesWithIntegrations(["claude-code"], deps);

  assert.deepEqual(deps.calls, [
    "persist",
    "list",
    "status:claude-code",
    "status:copilot",
    "status:generic-shell",
  ]);
  assert.deepEqual(result.integrations["claude-code"], {
    operation: "skipped",
    outcome: "failed",
    registration: "installed",
    activation: "active",
    coverage: "full",
    diagnosticCode: "ownership_ambiguous",
    message: "Resolve the existing hook manually.",
  });
  assert.deepEqual(result.integrations.copilot, {
    operation: "skipped",
    outcome: "failed",
    registration: "installed",
    activation: "disabled",
    coverage: "full",
    diagnosticCode: "ownership_ambiguous",
    message: "Resolve the existing hook manually.",
  });
});
