import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import type { SessionInfo } from "../src/api.ts";
import {
  needsAttention,
  sessionAttentionStatus,
  sortSessions,
  statusDotColor,
} from "../src/components/RightSidebarHelpers.ts";

test("DockviewApp registers panels through onReady", () => {
  const source = readFileSync(new URL("../src/components/DockviewApp.tsx", import.meta.url), "utf8");

  assert.match(source, /onReady=\{\(event: DockviewReadyEvent\) =>/);
  assert.doesNotMatch(source, /defaultLayout=/);
  assert.match(source, /api\.(fromJSON|addPanel)/);
});

test("App renders DockviewApp instead of the legacy three-panel layout", () => {
  const source = readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8");

  assert.match(source, /import DockviewApp from "\.\/components\/DockviewApp"/);
  assert.match(source, /<DockviewApp/);
  assert.doesNotMatch(source, /<TerminalTabs/);
  assert.doesNotMatch(source, /<LeftSidebar/);
  assert.doesNotMatch(source, /<RightSidebar/);
});

test("DockviewApp registers the five expected panel ids", () => {
  const source = readFileSync(new URL("../src/components/DockviewApp.tsx", import.meta.url), "utf8");

  for (const id of ["sessions", "detail", "terminal", "capacity", "recommendations"]) {
    assert.match(source, new RegExp(`component: "${id}"`));
  }
});

test("SessionDetailPanel includes the core detail sections", () => {
  const source = readFileSync(new URL("../src/components/SessionDetailPanel.tsx", import.meta.url), "utf8");

  for (const label of ["Task", "Status", "Directory", "Git", "Source", "Peon"]) {
    assert.match(source, new RegExp(`>${label}<`));
  }
  assert.match(source, /Select a session to see details/);
});

test("session list sorts by attention priority with lifecycle fallback", () => {
  const sessions: SessionInfo[] = [
    { id: "1", label: "s1", status: "running", cwd: "/tmp", created_at: "now", memoryState: "live", resumeStrategy: "none" },
    { id: "2", label: "s2", status: "running", observedStatus: "waiting_for_input", cwd: "/tmp", created_at: "now", memoryState: "live", resumeStrategy: "none" },
    { id: "3", label: "s3", status: "ended", cwd: "/tmp", created_at: "now", memoryState: "remembered", resumeStrategy: "none" },
    { id: "4", label: "s4", status: "running", observedStatus: "failed", cwd: "/tmp", created_at: "now", memoryState: "live", resumeStrategy: "none" },
    { id: "5", label: "s5", status: "running", observedStatus: "blocked", cwd: "/tmp", created_at: "now", memoryState: "live", resumeStrategy: "none" },
    { id: "6", label: "s6", status: "running", observedStatus: "done", cwd: "/tmp", created_at: "now", memoryState: "live", resumeStrategy: "none" },
  ];
  const sorted = sortSessions(sessions);
  assert.equal(sorted[0].id, "2"); // waiting_for_input
  assert.equal(sorted[1].id, "5"); // blocked
  assert.equal(sorted[2].id, "4"); // failed
  assert.equal(sorted[3].id, "6"); // done
  assert.equal(sorted[4].id, "1"); // running
  assert.equal(sorted[5].id, "3"); // ended
});

test("ended sessions do not have live status dot", () => {
  assert.equal(statusDotColor("ended"), "#666");
  assert.equal(statusDotColor("killed"), "#666");
  assert.equal(statusDotColor("error"), "#666");
});

test("sessionAttentionStatus falls back to lifecycle status", () => {
  const session: SessionInfo = {
    id: "1", label: "test", status: "running", cwd: "/tmp", created_at: "now",
    memoryState: "live", resumeStrategy: "none",
  };
  assert.equal(sessionAttentionStatus(session), "running");
});

test("needsAttention lifecycle statuses do not trigger from raw lifecycle", () => {
  assert.equal(needsAttention("running"), false);
  assert.equal(needsAttention("ended"), false);
  assert.equal(needsAttention("creating"), false);
});

test("session detail exposes resumable session action", () => {
  const source = readFileSync(
    new URL("../src/components/SessionDetailPanel.tsx", import.meta.url),
    "utf8",
  );

  assert.match(source, /onResumeSession/);
  assert.match(source, /Resume/);
  assert.match(source, /resumeStrategy/);
});

test("session list marks remembered sessions separately from live sessions", () => {
  const source = readFileSync(
    new URL("../src/components/SessionListPanel.tsx", import.meta.url),
    "utf8",
  );

  assert.match(source, /memoryState/);
  assert.match(source, /session-item--remembered/);
});

test("session list only offers kill for live sessions", () => {
  const source = readFileSync(
    new URL("../src/components/SessionListPanel.tsx", import.meta.url),
    "utf8",
  );

  assert.match(source, /s\.memoryState === "live" && \(\s*<button[\s\S]*session-kill-button/);
});

test("App restores the last active session from the initial workspace", () => {
  const source = readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8");

  assert.match(source, /info\.lastActiveSessionId/);
  assert.match(source, /setActiveSessionId\(info\.lastActiveSessionId\)/);
});
