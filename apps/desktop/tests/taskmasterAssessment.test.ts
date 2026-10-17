import { test } from "node:test";
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { build } from "esbuild";
import { createElement, type ReactElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { taskmasterRequest } from "../electron/taskmasterSettings.ts";
import {
  formatTaskmasterAssessment,
  formatTaskmasterManualRequest,
  formatTaskmasterProposalCounts,
  formatTaskmasterAssessmentAction,
  taskmasterAnalysisBusy,
  taskmasterAnalysisRequestBusy,
  taskmasterAssessmentRequestBusy,
  formatTaskmasterAnalysisAction,
  shouldFocusAssessmentRecommendation,
  taskmasterAssessmentForWorkspace,
  type TaskmasterAssessmentStatus,
  type AssessmentReport,
  type TaskmasterRunStatus,
} from "../src/taskmasterSettings.ts";

const require = createRequire(import.meta.url);
const compiledSummary = await build({
  entryPoints: [new URL("../src/components/TaskmasterAssessmentSummary.tsx", import.meta.url).pathname],
  bundle: true, write: false, platform: "node", format: "cjs", packages: "external",
});
const summaryModule = { exports: {} as Record<string, unknown> };
new Function("require", "module", "exports", compiledSummary.outputFiles[0].text)(require, summaryModule, summaryModule.exports);
const TaskmasterAssessmentSummary = summaryModule.exports.default as (props: { assessment: TaskmasterAssessmentStatus }) => ReactElement;
const compiledPanel = await build({
  entryPoints: [new URL("../src/components/RecommendationsPanel.tsx", import.meta.url).pathname],
  bundle: true, write: false, platform: "node", format: "cjs", packages: "external",
});
const panelModule = { exports: {} as Record<string, unknown> };
new Function("require", "module", "exports", compiledPanel.outputFiles[0].text)(require, panelModule, panelModule.exports);
const RecommendationsPanel = panelModule.exports.default as (props: {
  hasWorkspace: boolean;
  workspacePath: string | null;
  taskmasterReady: boolean;
  canFixWithAi: boolean;
}) => ReactElement;

const report: AssessmentReport = {
  id: "assessment-1",
  workspacePath: "/workspace/a",
  observedAt: "2026-10-17T10:00:00Z",
  provider: "codex",
  model: "gpt-x",
  bundleVersion: "bundle-4",
  outcome: "no_proposal",
  summary: "The workspace has enough structure for its current needs.",
  noProposalReason: "No evidence-backed improvement stood out.",
  unverifiedChecks: ["Build and tests were not run."],
  repositoryEvidence: [{ path: "README.md", sha256: "a".repeat(64), excerpt: "Project overview", observedAt: "2026-10-17T09:59:00Z" }],
  knowledgeEvidence: [{ pageId: "brain-1", title: "Keep changes small", status: "reviewed", bundleVersion: "bundle-4", sha256: "b".repeat(64), excerpt: "Prefer a narrow next step." }],
  recommendationId: null,
};

test("assessment transport sends an empty authenticated POST to its fixed route", async () => {
  let received: RequestInit | undefined;
  const result = await taskmasterRequest(4321, "test-authority", "assess-workflow", undefined, async (url, init) => {
    assert.equal(url, "http://127.0.0.1:4321/taskmaster/assess-workflow");
    received = init;
    return new Response(JSON.stringify({ status: "active_recommendation", recommendation: { id: "rec-1", status: "proposed" } }));
  });

  assert.equal(received?.method, "POST");
  assert.equal(received?.body, undefined);
  assert.equal((received?.headers as Record<string, string>)["x-orkworks-open-plan-token"], "test-authority");
  assert.deepEqual(result, { status: "active_recommendation", recommendation: { id: "rec-1", status: "proposed" } });
});

test("assessment transport rejects a response after the backend generation changes", async () => {
  let current = true;
  await assert.rejects(taskmasterRequest(4321, "test-authority", "assess-workflow", undefined, async () => {
    current = false;
    return new Response(JSON.stringify({ status: "scheduled" }));
  }, () => {
    if (!current) throw new Error("Workspace changed");
  }), /Workspace changed/);
});

test("assessment transport rejects caller-supplied context before network access", async () => {
  let calls = 0;
  await assert.rejects(taskmasterRequest(4321, "test-authority", "assess-workflow", { workspace: "/other" }, async () => {
    calls++;
    return new Response("{}");
  }), /does not accept a request body/);
  assert.equal(calls, 0);
});

test("assessment copy and provenance are returned only for their selected workspace", () => {
  assert.equal(taskmasterAssessmentForWorkspace({ workspacePath: "/workspace/b", assessment: { state: "succeeded", report, reused: true } }, "/workspace/a"), null);
  const status = { workspacePath: "/workspace/a", assessment: { state: "succeeded" as const, report, reused: true } };
  assert.deepEqual(taskmasterAssessmentForWorkspace(status, "/workspace/a"), status.assessment);
  const display = formatTaskmasterAssessment(status.assessment);
  assert.match(display, /No evidence-backed improvement stood out/);
  assert.match(display, /The workspace has enough structure/);
  assert.match(display, /codex \/ gpt-x/);
  assert.equal(report.unverifiedChecks[0], "Build and tests were not run.");
});

test("assessment summary renders no-proposal reason, provenance and unverified checks as text", () => {
  const markup = renderToStaticMarkup(createElement(TaskmasterAssessmentSummary, {
    assessment: { state: "succeeded", report: { ...report, summary: "<img src=x onerror=alert(1)>", outcome: "no_proposal" }, reused: false },
  }));
  assert.match(markup, /No evidence-backed improvement stood out/);
  assert.match(markup, /Brain bundle bundle-4/);
  assert.match(markup, /Assessment assessment-1 · Workspace \/workspace\/a/);
  assert.match(markup, /Build and tests were not run/);
  assert.match(markup, /&lt;img src=x onerror=alert\(1\)&gt;/);
  assert.doesNotMatch(markup, /<img src=x/);
});

test("proposed assessment identifies its historical recommendation without claiming it is still active", () => {
  const proposed = renderToStaticMarkup(createElement(TaskmasterAssessmentSummary, {
    assessment: { state: "succeeded", report: { ...report, outcome: "proposed", noProposalReason: null, recommendationId: "rec-1" }, reused: false },
  }));
  assert.match(proposed, /Assessment recommendation ID: rec-1/);
  assert.doesNotMatch(proposed, /available below/);
  assert.match(renderToStaticMarkup(createElement(TaskmasterAssessmentSummary, {
    assessment: { state: "running", report: null, reused: false },
  })), /Assessing this workspace/);
});

test("assessment action is enabled while workspace readiness is known, even before a report loads", () => {
  const buttonMarkup = (props: { hasWorkspace: boolean; taskmasterReady: boolean }) => {
    const markup = renderToStaticMarkup(createElement(RecommendationsPanel, {
      ...props,
      workspacePath: props.hasWorkspace ? "/workspace/a" : null,
      canFixWithAi: false,
    }));
    return markup.match(/<button type="button"[^>]*>Assess workflow<\/button>/)?.[0] ?? null;
  };

  const readyButton = buttonMarkup({ hasWorkspace: true, taskmasterReady: true });
  assert.ok(readyButton, "expected the assessment action in a ready workspace");
  assert.doesNotMatch(readyButton, /\sdisabled(?:=|\s|>)/);

  const noWorkspaceButton = buttonMarkup({ hasWorkspace: false, taskmasterReady: true });
  assert.ok(noWorkspaceButton);
  assert.match(noWorkspaceButton, /\sdisabled(?:=|\s|>)/);

  const notReadyButton = buttonMarkup({ hasWorkspace: true, taskmasterReady: false });
  assert.ok(notReadyButton);
  assert.match(notReadyButton, /\sdisabled(?:=|\s|>)/);
});

test("ordinary outcome summaries use server counts and keep legacy successes generic", () => {
  const base: TaskmasterRunStatus = {
    workspacePath: "/workspace/a",
    activeAttempt: null,
    latestOutcome: { state: "succeeded", startedAt: "t0", completedAt: "t1", trigger: "manual", provider: "codex", model: "gpt-x", errorSummary: null },
  };
  assert.equal(formatTaskmasterProposalCounts(base.latestOutcome), "");
  assert.equal(base.latestOutcome?.proposalCounts, undefined);
  assert.equal(formatTaskmasterProposalCounts({ ...base.latestOutcome!, proposalCounts: { created: 0, suppressed: 2 } }), "No new proposals · 2 duplicates suppressed");
  assert.equal(formatTaskmasterProposalCounts({ ...base.latestOutcome!, proposalCounts: { created: 1, suppressed: 1 } }), "1 proposal created · 1 duplicate suppressed");
  assert.equal(formatTaskmasterAssessment({ state: "idle", report: null, reused: false }), "No assessment has run for this workspace.");
  assert.equal(formatTaskmasterManualRequest({ id: 2, requestedAt: "t0", completedAt: "t1", disposition: "cached", reason: "unchanged_inputs" }), "Analysis reused the cached result because its inputs were unchanged.");
});

test("analysis-busy status includes ordinary attempts and persisted assessment progress", () => {
  const base: TaskmasterRunStatus = { workspacePath: "/workspace/a", activeAttempt: null, latestOutcome: null };
  assert.equal(taskmasterAnalysisBusy({ ...base, activeAttempt: { id: 1, state: "queued", queuedAt: "t0", startedAt: null, trigger: "manual", provider: "codex", model: "gpt-x" } }), true);
  assert.equal(taskmasterAnalysisBusy({ ...base, assessment: { state: "running", report: null, reused: false } }), true);
  assert.equal(taskmasterAnalysisBusy(base), false);
});

test("persisted ordinary attempts expose an explicit retry while local work and assessment progress stay guarded", () => {
  const base: TaskmasterRunStatus = { workspacePath: "/workspace/a", activeAttempt: null, latestOutcome: null };
  const queued = { ...base, activeAttempt: { id: 1, state: "queued" as const, queuedAt: "t0", startedAt: null, trigger: "manual" as const, provider: "codex", model: "gpt-x" } };
  const running = { ...queued, activeAttempt: { ...queued.activeAttempt, state: "running" as const, startedAt: "t1" } };
  const assessing = { ...base, assessment: { state: "running" as const, report: null, reused: false } };

  assert.equal(taskmasterAnalysisRequestBusy(queued, false, false), false);
  assert.equal(taskmasterAnalysisRequestBusy(running, false, false), false);
  assert.equal(taskmasterAnalysisRequestBusy(queued, true, false), true);
  assert.equal(taskmasterAnalysisRequestBusy(queued, false, true), true);
  assert.equal(taskmasterAnalysisRequestBusy(assessing, false, false), true);
  assert.equal(taskmasterAnalysisRequestBusy({ ...queued, assessment: assessing.assessment }, false, false), false,
    "ordinary retry remains reachable if both durable recovery records exist");
  assert.equal(taskmasterAnalysisRequestBusy({ ...queued, assessment: assessing.assessment }, true, false), true,
    "a locally pending request still blocks retries");
  assert.equal(formatTaskmasterAnalysisAction(queued), "Retry analysis");
  assert.equal(formatTaskmasterAnalysisAction(running), "Retry analysis");
  assert.equal(formatTaskmasterAnalysisAction(base), "Analyze now");
});

test("persisted assessment progress can be retried while actual local work remains guarded", () => {
  const base: TaskmasterRunStatus = { workspacePath: "/workspace/a", activeAttempt: null, latestOutcome: null };
  const queued = { ...base, assessment: { state: "queued" as const, report: null, reused: false } };
  const running = { ...base, assessment: { state: "running" as const, report: null, reused: false } };
  assert.equal(taskmasterAnalysisBusy(queued), true, "Analyze now remains blocked during assessment progress");
  assert.equal(taskmasterAssessmentRequestBusy(queued, false, false), false);
  assert.equal(taskmasterAssessmentRequestBusy(running, false, false), false);
  assert.equal(taskmasterAssessmentRequestBusy(base, true, false), true);
  assert.equal(taskmasterAssessmentRequestBusy(base, false, true), true);
  assert.equal(taskmasterAssessmentRequestBusy({ ...base, activeAttempt: {
    id: 1, state: "queued", queuedAt: "t0", startedAt: null, trigger: "manual", provider: "codex", model: "gpt-x",
  } }, false, false), true);
  assert.equal(formatTaskmasterAssessmentAction(queued.assessment), "Retry assessment");
  assert.equal(formatTaskmasterAssessmentAction(running.assessment), "Retry assessment");
  assert.equal(formatTaskmasterAssessmentAction(null), "Assess workflow");
});

test("assessment recommendation focus happens only once for a new actionable proposal", () => {
  const surfaced = new Set<string>();
  assert.equal(shouldFocusAssessmentRecommendation("rec-1", "proposed", surfaced), true);
  assert.equal(shouldFocusAssessmentRecommendation("rec-1", "proposed", surfaced), false);
  assert.equal(shouldFocusAssessmentRecommendation("rec-2", "dismissed", surfaced), false);
  assert.equal(surfaced.has("rec-2"), true);
});
