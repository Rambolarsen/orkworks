import test from "node:test";
import assert from "node:assert/strict";

import type { SessionInfo } from "../src/api.ts";
import {
  attentionLabel,
  attentionTone,
  detailActionZone,
  lifecyclePhaseLabel,
  memoryStateLabel,
  relativeTime,
  resumeActionLabel,
  resumeChoices,
  situationHeadline,
  situationTail,
  sourceLabel,
  sourceWithConfidence,
  VOCAB,
  workPhaseLabel,
} from "../src/labels.ts";

function baseSession(overrides: Partial<SessionInfo> = {}): SessionInfo {
  return {
    id: "s1",
    label: "session",
    status: "running",
    cwd: "/home/user/orkworks",
    created_at: "2026-06-19T12:00:00Z",
    memoryState: "live",
    resumeStrategy: "none",
    ...overrides,
  };
}

test("VOCAB uses 'Workspace' consistently (no 'Folder' drift)", () => {
  assert.equal(VOCAB.workspace, "Workspace");
  assert.equal(VOCAB.openWorkspace, "Open workspace…");
  assert.equal(VOCAB.switchWorkspace, "Switch workspace");
});

test("attentionLabel maps every known status to plain English", () => {
  assert.equal(attentionLabel("waiting_for_input"), "Needs you");
  assert.equal(attentionLabel("blocked"), "Blocked");
  assert.equal(attentionLabel("failed"), "Failed");
  assert.equal(attentionLabel("done"), "Done");
  assert.equal(attentionLabel("stale"), "Idle");
  assert.equal(attentionLabel("idle"), "Idle");
  assert.equal(attentionLabel("working"), "Working");
  assert.equal(attentionLabel("running"), "Running");
  assert.equal(attentionLabel("creating"), "Starting");
  assert.equal(attentionLabel("ended"), "Ended");
  assert.equal(attentionLabel("killed"), "Killed");
  assert.equal(attentionLabel("error"), "Error");
  assert.equal(attentionLabel("anything-else"), "Unknown");
});

test("attentionTone collapses statuses into the visual-weight axis", () => {
  assert.equal(attentionTone("waiting_for_input"), "needs-you");
  assert.equal(attentionTone("failed"), "failed");
  assert.equal(attentionTone("blocked"), "blocked");
  assert.equal(attentionTone("done"), "done");
  assert.equal(attentionTone("working"), "working");
  assert.equal(attentionTone("running"), "working");
  assert.equal(attentionTone("creating"), "working");
  assert.equal(attentionTone("stale"), "idle");
  assert.equal(attentionTone("idle"), "idle");
  assert.equal(attentionTone("ended"), "neutral");
  assert.equal(attentionTone("anything-else"), "neutral");
});

test("memoryStateLabel maps every memory state to a single word", () => {
  assert.equal(memoryStateLabel("live"), "Live");
  assert.equal(memoryStateLabel("resumable"), "Resumable");
  assert.equal(memoryStateLabel("remembered"), "Remembered");
  assert.equal(memoryStateLabel("unsupported"), "—");
});

test("workPhaseLabel and lifecyclePhaseLabel format session lifecycle fields", () => {
  assert.equal(workPhaseLabel("ideation"), "Ideation");
  assert.equal(workPhaseLabel("implementation"), "Implementation");
  assert.equal(workPhaseLabel("review"), "Review");
  assert.equal(workPhaseLabel("debugging"), "Debugging");
  assert.equal(workPhaseLabel("unknown"), "Unknown");
  assert.equal(workPhaseLabel(undefined), "Unknown");

  assert.equal(lifecyclePhaseLabel("creating"), "Creating");
  assert.equal(lifecyclePhaseLabel("active"), "Active");
  assert.equal(lifecyclePhaseLabel("ending"), "Ending");
  assert.equal(lifecyclePhaseLabel("ended"), "Ended");
  assert.equal(lifecyclePhaseLabel(undefined), "Unknown");
});

test("resumeActionLabel produces button-ready prose for every resume strategy", () => {
  assert.equal(resumeActionLabel("exact"), "Resume session");
  assert.equal(resumeActionLabel("latest_cwd"), "Resume latest in folder");
  assert.equal(resumeActionLabel("latest_repo"), "Resume latest in repo");
  assert.equal(resumeActionLabel("none"), "Resume unavailable");
});

test("sourceLabel handles the typed sources and falls back for unknowns", () => {
  assert.equal(sourceLabel("agent"), "Agent");
  assert.equal(sourceLabel("peon"), "Peon");
  assert.equal(sourceLabel("process"), "Process");
  assert.equal(sourceLabel(undefined), "Unknown");
});

test("sourceWithConfidence renders the confidence as a percentage", () => {
  assert.equal(sourceWithConfidence("agent", 1), "Agent · 100% confidence");
  assert.equal(sourceWithConfidence("agent", 0.95), "Agent · 95% confidence");
  assert.equal(sourceWithConfidence("peon", 0.512), "Peon · 51% confidence");
  assert.equal(sourceWithConfidence("agent", undefined), "Agent");
  assert.equal(sourceWithConfidence(undefined, undefined), "Unknown");
});

test("relativeTime buckets recent timestamps into human-readable spans", () => {
  const now = new Date("2026-06-19T12:00:00Z");
  assert.equal(relativeTime("2026-06-19T11:59:59Z", now), "just now");
  assert.equal(relativeTime("2026-06-19T11:59:30Z", now), "30s ago");
  assert.equal(relativeTime("2026-06-19T11:55:00Z", now), "5m ago");
  assert.equal(relativeTime("2026-06-19T10:00:00Z", now), "2h ago");
  assert.equal(relativeTime("2026-06-17T12:00:00Z", now), "2d ago");
  assert.equal(relativeTime(undefined, now), "");
  assert.equal(relativeTime("not-a-date", now), "");
});

test("situationHeadline falls back through question, blocker, summary, next action", () => {
  assert.equal(situationHeadline(baseSession({ detectedQuestion: "Q?" })), "Q?");
  assert.equal(situationHeadline(baseSession({ blockerDescription: "B" })), "B");
  assert.equal(situationHeadline(baseSession({ summary: "S" })), "S");
  assert.equal(situationHeadline(baseSession({ nextAction: "N" })), "N");
  assert.equal(situationHeadline(baseSession({})), "No additional detail recorded.");
  assert.equal(
    situationHeadline(baseSession({ detectedQuestion: "Q?", blockerDescription: "B" })),
    "Q?",
  );
});

test("situationTail quotes the peon's detected options or the raw failure text, never the headline fields", () => {
  assert.equal(
    situationTail(baseSession({ suggestedOptions: ["lazy migrate", "one-shot migrate"] }), "needs-you"),
    "lazy migrate  ·  one-shot migrate",
  );
  assert.equal(situationTail(baseSession({ failedTest: "test::foo" }), "failed"), "test::foo");
  assert.equal(situationTail(baseSession({ failedCommand: "cargo test" }), "failed"), "cargo test");
  assert.equal(
    situationTail(baseSession({ failedTest: "test::foo", failedCommand: "cargo test" }), "failed"),
    "test::foo",
  );
  assert.equal(situationTail(baseSession({ summary: "S" }), "idle"), undefined);
});

test("situationTail ignores stale suggestedOptions once a session has moved past needs-you (backend keeps the field sticky across peon updates)", () => {
  const staleSession = baseSession({
    suggestedOptions: ["lazy migrate", "one-shot migrate"],
    failedTest: "test::foo",
  });
  assert.equal(situationTail(staleSession, "failed"), "test::foo");
  assert.equal(situationTail(staleSession, "blocked"), undefined);
  assert.equal(situationTail(staleSession, "done"), undefined);
});

test("resumeChoices maps backend resumeOptions into chooser rows", () => {
  const session = baseSession({
    memoryState: "resumable",
    resumeStrategy: "latest_cwd",
    cwd: "/home/user/orkworks/specs",
    resumeOptions: [
      { strategy: "latest_cwd", label: "Resume latest in folder", available: true, preferred: true },
      { strategy: "exact", label: "Resume this exact session", available: false, preferred: false, reason: "not available — process ended" },
    ],
  });
  const choices = resumeChoices(session);
  assert.equal(choices.length, 2);
  assert.deepEqual(choices[0], { label: "Resume latest in folder", sub: "Newest session in specs", recommended: true, unavailable: false });
  assert.deepEqual(choices[1], { label: "Resume this exact session", sub: "not available — process ended", recommended: false, unavailable: true });
});

test("resumeChoices synthesizes a single row when the backend sends no resumeOptions", () => {
  const session = baseSession({ memoryState: "resumable", resumeStrategy: "exact" });
  const choices = resumeChoices(session);
  assert.deepEqual(choices, [{ label: "Resume session", sub: "Reattach to this exact session", recommended: true, unavailable: false }]);
});

test("resumeChoices is empty when there is no resume strategy at all", () => {
  assert.deepEqual(resumeChoices(baseSession({ memoryState: "unsupported", resumeStrategy: "none" })), []);
});

test("detailActionZone prefers the resume chooser for non-live sessions", () => {
  const session = baseSession({
    memoryState: "remembered",
    resumeStrategy: "latest_repo",
  });
  const zone = detailActionZone(session, "idle");
  assert.equal(zone.kind, "resume");
  if (zone.kind === "resume") {
    assert.equal(zone.note, VOCAB.resumeNoteRemembered);
    assert.equal(zone.options.length, 1);
  }
});

test("detailActionZone shows a cue for a live needs-you session, buttons when done, nothing otherwise", () => {
  assert.deepEqual(detailActionZone(baseSession(), "needs-you"), { kind: "cue", text: VOCAB.cueReplyInTerminal });
  assert.deepEqual(detailActionZone(baseSession(), "done"), { kind: "buttons" });
  assert.deepEqual(detailActionZone(baseSession(), "working"), { kind: "none" });
  assert.deepEqual(detailActionZone(baseSession(), "neutral"), { kind: "none" });
});

test("detailActionZone omits the resume chooser when resumable but nothing to offer", () => {
  const session = baseSession({ memoryState: "resumable", resumeStrategy: "none" });
  assert.deepEqual(detailActionZone(session, "idle"), { kind: "none" });
});

test("detailActionZone shows resume chooser for unsupported sessions when backend sends unavailable options", () => {
  const session = baseSession({
    memoryState: "unsupported",
    resumeStrategy: "none",
    resumeOptions: [
      { strategy: "exact", label: "Resume exact session", available: false, preferred: false, reason: "No harness session id was captured" },
    ],
  });
  const zone = detailActionZone(session, "idle");
  assert.equal(zone.kind, "resume");
  if (zone.kind === "resume") {
    assert.equal(zone.options.length, 1);
    assert.equal(zone.options[0].unavailable, true);
  }
});
