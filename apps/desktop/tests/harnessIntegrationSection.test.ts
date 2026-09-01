import test from "node:test";
import assert from "node:assert/strict";

import {
  deriveIntegrationDisplayState,
  integrationOperationForHarness,
  isAttentionSignal,
  shouldShowInstalledConfirmation,
} from "../src/harnessIntegrationPresentation.ts";
import type {
  ActiveHarnessIntegrationResult,
  IntegrationStatusResult,
} from "../src/harnessIntegrationPresentation.ts";

test("isAttentionSignal is false for codex, whose hook only reports a session ID (issue #271)", () => {
  assert.equal(isAttentionSignal("codex"), false);
});

test("isAttentionSignal is false for opencode, whose session.created hook only reports a session ID (issue #110)", () => {
  assert.equal(isAttentionSignal("opencode"), false);
});

test("isAttentionSignal is true for harnesses whose hook reports needs-input attention", () => {
  assert.equal(isAttentionSignal("claude-code"), true);
  assert.equal(isAttentionSignal("gemini"), true);
  assert.equal(isAttentionSignal("copilot"), true);
});

test("unsupported tool versions suppress installed confirmation", () => {
  assert.equal(
    shouldShowInstalledConfirmation([{ code: "unsupported_tool_version", message: "unsupported" }]),
    false,
  );
  assert.equal(shouldShowInstalledConfirmation([]), true);
});

function integrationStatus(
  overrides: Partial<Exclude<IntegrationStatusResult, { ok: false }>["status"]> = {},
): IntegrationStatusResult {
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

function integrationOperation(
  overrides: Partial<ActiveHarnessIntegrationResult> = {},
): ActiveHarnessIntegrationResult {
  return {
    key: { adapterId: "copilot", targetId: "workspace" },
    consumerHarnessIds: ["copilot", "copilot-local"],
    operation: "install",
    outcome: "succeeded",
    registration: "installed",
    activation: "active",
    coverage: "full",
    ...overrides,
  };
}

test("grouped operation results project the same outcome to every consuming row", () => {
  const operation = integrationOperation({ outcome: "failed", message: "permission denied" });
  const results = { "copilot/workspace": operation };

  assert.equal(integrationOperationForHarness(results, "copilot"), operation);
  assert.equal(integrationOperationForHarness(results, "copilot-local"), operation);
  assert.equal(integrationOperationForHarness(results, "codex"), undefined);
});

test("deriveIntegrationDisplayState returns healthy for enabled installed full coverage", () => {
  const display = deriveIntegrationDisplayState({
    harnessName: "Codex",
    enabled: true,
    status: integrationStatus(),
  });

  assert.equal(display.appearance, "healthy");
  assert.equal(display.label, "healthy");
  assert.match(display.description, /Installed/);
  assert.match(display.tooltip, /Codex/);
});

test("deriveIntegrationDisplayState returns needs-you for enabled but absent integrations", () => {
  const display = deriveIntegrationDisplayState({
    harnessName: "Claude Code",
    enabled: true,
    status: integrationStatus({
      harnessId: "claude-code",
      registration: "absent",
      ownership: "none",
      activation: "unknown",
    }),
  });

  assert.equal(display.appearance, "needs-you");
  assert.notEqual(display.appearance, "warning");
  assert.equal(display.glyph, "warning");
  assert.match(display.description, /needs installation/);
});

test("deriveIntegrationDisplayState keeps Codex trust-pending in needs-you state", () => {
  const display = deriveIntegrationDisplayState({
    harnessName: "Codex",
    enabled: true,
    status: integrationStatus({
      activation: "needs_trust",
    }),
  });

  assert.equal(display.appearance, "needs-you");
  assert.equal(display.glyph, "trust");
  assert.match(display.tooltip, /approve/i);
});

test("deriveIntegrationDisplayState returns needs-you for disabled tools with owned cleanup remaining", () => {
  const display = deriveIntegrationDisplayState({
    harnessName: "Claude Code",
    enabled: false,
    status: integrationStatus({
      harnessId: "claude-code",
      enabled: false,
      registration: "installed",
      ownership: "ork_works",
      activation: "disabled",
    }),
  });

  assert.equal(display.appearance, "needs-you");
  assert.equal(display.glyph, "warning");
  assert.match(display.description, /cleanup/i);
});

test("deriveIntegrationDisplayState does not call a disabled shared consumer cleanup-needed while another consumer is active", () => {
  const display = deriveIntegrationDisplayState({
    harnessName: "Copilot",
    enabled: false,
    status: integrationStatus({
      harnessId: "copilot",
      activeConsumerCount: 1,
    }),
  });

  assert.equal(display.appearance, "off");
  assert.equal(display.label, "off");
  assert.match(display.description, /shared integration is still in use/);
});

test("deriveIntegrationDisplayState returns neutral for enabled unsupported integrations even with the backend's always-present diagnostic", () => {
  const display = deriveIntegrationDisplayState({
    harnessName: "Antigravity CLI",
    enabled: true,
    status: integrationStatus({
      harnessId: "antigravity",
      registration: "unsupported",
      ownership: "none",
      activation: "not_applicable",
      coverage: "none",
      diagnostics: [{ code: "no_deterministic_integration", message: "No deterministic OrkWorks integration exists for this coding tool." }],
    }),
  });

  assert.equal(display.appearance, "neutral");
  assert.equal(display.glyph, "neutral");
  assert.match(display.description, /No OrkWorks hook support/);
});

test("deriveIntegrationDisplayState returns healthy with limited coverage explanation for aider even with its always-present diagnostic", () => {
  const display = deriveIntegrationDisplayState({
    harnessName: "Aider",
    enabled: true,
    status: integrationStatus({
      harnessId: "aider",
      activation: "unknown",
      coverage: "limited",
      diagnostics: [{ code: "no_native_session_id", message: "Aider notifications report attention only; Aider has no native session ID contract." }],
      confirmation: {
        toolName: "Aider",
        workspaceLabel: "workspace",
        coverageSummary: "limited notification coverage",
        relativePaths: [".aider"],
        executableCodeWarning: false,
      },
    }),
  });

  assert.equal(display.appearance, "healthy");
  assert.equal(display.glyph, "healthy");
  assert.match(display.tooltip, /limited notification coverage/);
});

test("deriveIntegrationDisplayState returns error when status is unavailable", () => {
  const display = deriveIntegrationDisplayState({
    harnessName: "OpenCode",
    enabled: true,
    status: { ok: false, error: "backend unavailable" },
  });

  assert.equal(display.appearance, "error");
  assert.equal(display.glyph, "offline");
  assert.match(display.tooltip, /Retry status check/);
});

test("deriveIntegrationDisplayState keeps failed operations actionable even when status refresh is unavailable", () => {
  const display = deriveIntegrationDisplayState({
    harnessName: "Codex",
    enabled: true,
    status: { ok: false, error: "backend unavailable" },
    operation: integrationOperation({
      outcome: "failed",
      diagnosticCode: "permission_denied",
      message: "Hook installation failed: permission denied.",
    }),
  });

  assert.equal(display.appearance, "needs-you");
  assert.equal(display.label, "action required");
  assert.equal(display.glyph, "warning");
  assert.match(display.tooltip, /permission denied/i);
});

test("deriveIntegrationDisplayState gives operation failure precedence over status diagnostics", () => {
  const display = deriveIntegrationDisplayState({
    harnessName: "Codex",
    enabled: true,
    status: integrationStatus({
      diagnostics: [{ code: "needs_repair", message: "Status says reinstall." }],
    }),
    operation: integrationOperation({
      outcome: "failed",
      diagnosticCode: "permission_denied",
      message: "Hook installation failed: permission denied.",
    }),
  });

  assert.equal(display.appearance, "needs-you");
  assert.equal(display.glyph, "warning");
  assert.match(display.tooltip, /permission denied/i);
});

test("deriveIntegrationDisplayState ignores stale_workspace outcomes in the display contract", () => {
  const display = deriveIntegrationDisplayState({
    harnessName: "Codex",
    enabled: true,
    status: integrationStatus(),
    operation: integrationOperation({
      outcome: "stale_workspace",
      message: "Workspace changed while saving.",
    }),
  });

  assert.equal(display.appearance, "healthy");
  assert.equal(display.label, "healthy");
  assert.equal(display.glyph, "healthy");
  assert.doesNotMatch(display.tooltip, /workspace changed/i);
});

test("deriveIntegrationDisplayState returns neutral spinner state while integration work is in progress", () => {
  const display = deriveIntegrationDisplayState({
    harnessName: "Codex",
    enabled: true,
    status: integrationStatus(),
    inProgress: true,
  });

  assert.equal(display.appearance, "in-progress");
  assert.equal(display.glyph, "spinner");
  assert.match(display.description, /in progress/i);
});
