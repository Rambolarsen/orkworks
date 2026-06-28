import test from "node:test";
import assert from "node:assert/strict";

import {
  attentionLabel,
  attentionTone,
  memoryStateLabel,
  relativeTime,
  resumeActionLabel,
  sourceLabel,
  sourceWithConfidence,
  VOCAB,
} from "../src/labels.ts";

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
