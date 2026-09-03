import type {
  IntegrationActivation,
  IntegrationCoverage,
  IntegrationRegistration,
  IntegrationStatus,
  IntegrationStatusResult,
} from "./harnessTypes.ts";

// TODO(#271): derive this from a backend-declared event-semantics field on
// the integration status instead of a per-harness special case here — Codex
// is the only integration today whose hook doesn't mean "needs input" (it
// reports a session ID only; see issue #110). OpenCode's plugin gained
// attention reporting (idle/permission/busy events) in issue #104.
export function isAttentionSignal(harnessId: string): boolean {
  return harnessId !== "codex";
}

export function shouldShowInstalledConfirmation(
  diagnostics: ReadonlyArray<{ code: string }>,
): boolean {
  return !diagnostics.some((diagnostic) => diagnostic.code === "unsupported_tool_version");
}

// Diagnostics that describe a permanent, inherent capability limitation
// rather than something actionable — the backend attaches these
// unconditionally alongside an otherwise-healthy status, so they must not
// force the "needs-you" display state.
const INFORMATIONAL_DIAGNOSTIC_CODES = new Set(["no_native_session_id", "no_deterministic_integration"]);

export interface IntegrationDisplayState {
  appearance: "off" | "neutral" | "healthy" | "needs-you" | "error" | "in-progress";
  label: string;
  description: string;
  tooltip: string;
  glyph: "neutral" | "healthy" | "warning" | "trust" | "offline" | "spinner";
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

export interface IntegrationKey {
  adapterId: string;
  targetId: string;
}

export interface IntegrationConsumer {
  harnessId: string;
  harnessName: string;
}

export interface GroupedIntegrationStatus {
  key: IntegrationKey;
  consumers: IntegrationConsumer[];
  status: IntegrationStatus;
}

export interface ActiveHarnessSaveResult {
  activeHarnesses: {
    outcome: "persisted" | "failed" | "stale_workspace";
    message?: string;
  };
  integrations: Record<string, ActiveHarnessIntegrationResult>;
}

/** Projects one grouped save outcome onto the row that consumes its adapter. */
export function integrationOperationForHarness(
  results: Record<string, ActiveHarnessIntegrationResult>,
  harnessId: string,
): ActiveHarnessIntegrationResult | undefined {
  return Object.values(results).find((result) => result.consumerHarnessIds.includes(harnessId));
}

// Renderer-side mirror of the main process's planMutation mutate conditions
// (electron/activeHarnessIntegration.ts): the per-row Reconcile affordance is
// offered exactly when a reconcile would actually plan a mutation. Duplication
// across the electron/src boundary is intentional (see apps/desktop/AGENTS.md).
// `enabled` must be the persisted selection, not the modal's draft.
export function isReconcileActionable(
  enabled: boolean,
  status: IntegrationStatusResult,
): boolean {
  if (!status.ok) return false;
  const current = status.status;
  if (current.registration === "unsupported") return false;

  if (enabled) {
    if (current.ownership === "ambiguous") return false;
    if (current.activation === "needs_trust") return false;
    if (current.registration === "absent") return true;
    if (current.registration === "drifted" || current.registration === "error") return true;
    if (current.registration === "installed") {
      return current.diagnostics.some(
        (diagnostic) =>
          diagnostic.code !== "tool_not_detected"
          && diagnostic.code !== "needs_trust"
          && diagnostic.code !== "unsupported_tool_version",
      );
    }
    return false;
  }

  return current.ownership === "ork_works" && current.registration !== "absent";
}

interface DeriveIntegrationDisplayStateInput {
  harnessName: string;
  enabled: boolean;
  status: IntegrationStatusResult;
  operation?: ActiveHarnessIntegrationResult | null;
  inProgress?: boolean;
}

function displayState(
  appearance: IntegrationDisplayState["appearance"],
  label: IntegrationDisplayState["label"],
  description: string,
  tooltip: string,
  glyph: IntegrationDisplayState["glyph"],
): IntegrationDisplayState {
  return { appearance, label, description, tooltip, glyph };
}

function unsupportedState(harnessName: string, enabled: boolean): IntegrationDisplayState {
  if (!enabled) {
    return displayState(
      "off",
      "off",
      "Disabled. No OrkWorks integration remains.",
      `${harnessName} is disabled and no OrkWorks-owned integration remains in this workspace.`,
      "neutral",
    );
  }
  return displayState(
    "neutral",
    "no hook support",
    "Enabled. No OrkWorks hook support for this coding tool.",
    `${harnessName} is enabled, but this coding tool has no OrkWorks hook capability.`,
    "neutral",
  );
}

export function deriveIntegrationDisplayState({
  harnessName,
  enabled,
  status,
  operation,
  inProgress = false,
}: DeriveIntegrationDisplayStateInput): IntegrationDisplayState {
  if (inProgress) {
    return displayState(
      "in-progress",
      "updating",
      "Integration operation in progress.",
      `OrkWorks is updating the ${harnessName} integration.`,
      "spinner",
    );
  }

  if (operation?.outcome === "failed") {
    const message = operation.message ?? "The last integration operation needs attention.";
    return displayState("needs-you", "action required", message, message, "warning");
  }

  if (!status.ok) {
    return displayState(
      "error",
      "status unavailable",
      "Integration status unavailable.",
      `${harnessName} integration status is unavailable. Retry status check.`,
      "offline",
    );
  }

  const current = status.status;
  const diagnostic = current.diagnostics.find((d) => !INFORMATIONAL_DIAGNOSTIC_CODES.has(d.code));

  if (diagnostic) {
    return displayState(
      "needs-you",
      "action required",
      diagnostic.message,
      `${harnessName}: ${diagnostic.message}`,
      diagnostic.code === "needs_trust" ? "trust" : "warning",
    );
  }

  if (!enabled) {
    if ((current.activeConsumerCount ?? 0) > 0) {
      const message = `${harnessName} is disabled; the shared integration is still in use by another coding tool.`;
      return displayState("off", "off", message, message, "neutral");
    }
    if (current.ownership === "ork_works" && current.registration !== "absent") {
      const message = `${harnessName} is disabled, but OrkWorks-owned integration cleanup is still needed.`;
      return displayState("needs-you", "cleanup needed", message, message, "warning");
    }
    return unsupportedState(harnessName, false);
  }

  if (current.activation === "needs_trust") {
    return displayState(
      "needs-you",
      "approval needed",
      "Enabled, but the coding tool still needs to approve the hook.",
      `${harnessName} is enabled, but you still need to approve the hook inside the coding tool.`,
      "trust",
    );
  }

  if (current.ownership === "ambiguous") {
    return displayState(
      "needs-you",
      "ownership unclear",
      "Enabled, but OrkWorks cannot safely change the existing integration.",
      `${harnessName} has an ambiguous workspace-local integration. Reconcile it outside OrkWorks, then retry.`,
      "warning",
    );
  }

  if (current.registration === "absent") {
    return displayState(
      "needs-you",
      "install needed",
      "Enabled, but the integration needs installation.",
      `${harnessName} is enabled, but its OrkWorks integration needs installation.`,
      "warning",
    );
  }

  if (current.registration === "drifted" || current.registration === "error") {
    return displayState(
      "needs-you",
      "repair needed",
      "Enabled, but the integration needs repair.",
      `${harnessName} is enabled, but its OrkWorks integration needs repair.`,
      "warning",
    );
  }

  if (current.registration === "unsupported") {
    return unsupportedState(harnessName, true);
  }

  if (current.coverage === "limited") {
    const summary = current.confirmation?.coverageSummary ?? "limited coverage";
    return displayState(
      "healthy",
      "limited coverage",
      "Installed with limited OrkWorks coverage.",
      `${harnessName} integration is installed with ${summary}.`,
      "healthy",
    );
  }

  return displayState(
    "healthy",
    "healthy",
    "Installed and healthy.",
    `${harnessName} integration is installed and healthy.`,
    "healthy",
  );
}
