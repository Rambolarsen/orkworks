import test from "node:test";
import assert from "node:assert/strict";

import {
  createWorkspaceSessionController,
  type WorkspaceSessionControllerDeps,
} from "../src/workspaceSessionController.ts";
import type { SessionInfo, WorkspaceInfo } from "../src/api.ts";
import type { CreateSessionOptions } from "../src/harnessTypes.ts";
import type { PollScheduler } from "../src/sessionPolling.ts";

function session(id: string, lifecycle: SessionInfo["lifecycle"] = "alive", status = "running"): SessionInfo {
  return {
    id,
    label: id,
    lifecycle,
    status,
    cwd: "/tmp",
    created_at: "2026-08-17T07:00:00Z",
    memoryState: "live",
    resumeStrategy: "none",
  };
}

function workspace(path: string, lastActiveSessionId: string | null = null): WorkspaceInfo {
  return {
    path,
    workspaceIdentity: path,
    repo_root: path,
    branch: "main",
    dirty: false,
    lastActiveSessionId,
    activeHarnessIds: [],
    activeHarnessRevision: 0,
  };
}

function deferred<T>(): { promise: Promise<T>; resolve: (value: T) => void; reject: (error: unknown) => void } {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((done, fail) => {
    resolve = done;
    reject = fail;
  });
  return { promise, resolve, reject };
}

function scheduler(): { scheduler: PollScheduler; callbacks: Array<() => void> } {
  const callbacks: Array<() => void> = [];
  return {
    callbacks,
    scheduler: {
      set(callback) {
        callbacks.push(callback);
        return callback;
      },
      clear() {},
    },
  };
}

function deps(overrides: Partial<WorkspaceSessionControllerDeps> = {}): WorkspaceSessionControllerDeps {
  return {
    getBackendUrl: async () => "http://backend",
    setWorkspace: async (_baseUrl, path) => workspace(path),
    listSessions: async () => [],
    createSession: async (_baseUrl, _options) => session("created", "creating", "creating"),
    resumeSession: async (_baseUrl, id) => session(id),
    deleteSession: async () => {},
    forgetSession: async () => {},
    sessionAction: async () => {},
    setActiveWorkspaceSession: async () => {},
    pruneTerminals: () => {},
    disposeTerminal: () => {},
    ...overrides,
  };
}

test("the controller starts one polling loop only when explicitly enabled", async () => {
  const polling = scheduler();
  let lists = 0;
  const controller = createWorkspaceSessionController({
    deps: deps({ listSessions: async () => { lists += 1; return []; } }),
    scheduler: polling.scheduler,
  });

  assert.equal(polling.callbacks.length, 0);
  controller.setPollingEnabled(true);
  await controller.refreshSessions();
  assert.equal(lists, 2);
  assert.equal(polling.callbacks.length, 1);
  controller.setPollingEnabled(true);
  assert.equal(polling.callbacks.length, 1);
  controller.dispose();
});

test("a poll response cannot make an in-flight foreground create stale", async () => {
  const pollList = deferred<SessionInfo[]>();
  const create = deferred<SessionInfo>();
  const active: Array<string | null> = [];
  const controller = createWorkspaceSessionController({
    deps: deps({
      listSessions: async () => pollList.promise,
      createSession: async () => create.promise,
    }),
    onActiveSession: (id) => active.push(id),
    scheduler: scheduler().scheduler,
  });
  const creating = controller.createSession({} satisfies CreateSessionOptions);
  await new Promise((resolve) => setImmediate(resolve));
  controller.setPollingEnabled(true);
  await new Promise((resolve) => setImmediate(resolve));
  pollList.resolve([]);
  await new Promise((resolve) => setImmediate(resolve));
  create.resolve(session("created", "creating", "creating"));
  await creating;
  assert.deepEqual(active, ["created"]);
  controller.dispose();
});

test("publishes a newly created session alongside the current snapshot before the next poll", async () => {
  let listCalls = 0;
  const snapshots: string[][] = [];
  const controller = createWorkspaceSessionController({
    deps: deps({
      listSessions: async () => {
        listCalls += 1;
        return [session("existing")];
      },
      createSession: async () => session("created", "creating", "creating"),
    }),
    onSessions: (next) => snapshots.push(next.map((item) => item.id)),
    scheduler: scheduler().scheduler,
  });

  await controller.refreshSessions();
  snapshots.length = 0;

  assert.equal(await controller.createSession({} satisfies CreateSessionOptions), true);

  assert.equal(listCalls, 1);
  assert.deepEqual(snapshots.map((ids) => ids.sort()), [["created", "existing"]]);
  controller.dispose();
});

test("a stale workspace response cannot publish callbacks", async () => {
  const first = deferred<WorkspaceInfo>();
  const second = deferred<WorkspaceInfo>();
  const published: string[] = [];
  let calls = 0;
  const controller = createWorkspaceSessionController({
    deps: deps({
      setWorkspace: async () => ++calls === 1 ? first.promise : second.promise,
      listSessions: async () => [session("live")],
    }),
    onWorkspace: (info) => published.push(info.path),
  });

  const old = controller.openWorkspace("old");
  const next = controller.openWorkspace("new");
  second.resolve(workspace("new"));
  await next;
  first.resolve(workspace("old"));
  await old;
  assert.deepEqual(published, ["new"]);
  controller.dispose();
});

test("dispose makes late workspace and poll work no-ops", async () => {
  const pending = deferred<WorkspaceInfo>();
  const snapshots: SessionInfo[][] = [];
  const controller = createWorkspaceSessionController({
    deps: deps({ setWorkspace: async () => pending.promise }),
    onSessions: (next) => snapshots.push([...next]),
  });
  const operation = controller.openWorkspace("workspace");
  controller.dispose();
  pending.resolve(workspace("workspace"));
  await operation;
  assert.deepEqual(snapshots, []);
});

test("disposal stops an enabled poll and rejects its late response", async () => {
  const pending = deferred<SessionInfo[]>();
  const polling = scheduler();
  const snapshots: SessionInfo[][] = [];
  const controller = createWorkspaceSessionController({
    deps: deps({ listSessions: async () => pending.promise }),
    scheduler: polling.scheduler,
    onSessions: (next) => snapshots.push([...next]),
  });
  controller.setPollingEnabled(true);
  controller.dispose();
  pending.resolve([session("late")]);
  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(snapshots, []);
  assert.equal(polling.callbacks.length, 0);
});

test("disabling and re-enabling polling rejects the old in-flight response", async () => {
  const oldList = deferred<SessionInfo[]>();
  const newList = deferred<SessionInfo[]>();
  const polling = scheduler();
  let calls = 0;
  const snapshots: string[] = [];
  const controller = createWorkspaceSessionController({
    deps: deps({
      listSessions: async () => ++calls === 1 ? oldList.promise : newList.promise,
    }),
    scheduler: polling.scheduler,
    onSessions: (next) => snapshots.push(next.map((item) => item.id).join(",")),
  });

  controller.setPollingEnabled(true);
  await new Promise((resolve) => setImmediate(resolve));
  controller.setPollingEnabled(false);
  controller.setPollingEnabled(true);
  await new Promise((resolve) => setImmediate(resolve));

  oldList.resolve([session("old")]);
  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(snapshots, []);

  newList.resolve([session("new")]);
  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(snapshots, ["new"]);
  controller.dispose();
});

test("only the exact tracked creating id reaching error reports a create error", async () => {
  const errors: string[] = [];
  let polled: SessionInfo[] = [session("tracked", "creating", "creating"), session("other")];
  const controller = createWorkspaceSessionController({
    deps: deps({ listSessions: async () => polled, createSession: async () => session("tracked", "creating", "creating") }),
    onError: (error) => errors.push(error.key),
  });
  await controller.createSession({} satisfies CreateSessionOptions);
  await controller.refreshSessions();
  assert.deepEqual(errors, []);
  polled = [session("tracked", "dead", "error"), session("other", "dead", "error")];
  await controller.refreshSessions();
  assert.deepEqual(errors, ["create"]);
  controller.dispose();
});

test("restores a matching live last active session after the refreshed snapshot", async () => {
  const active: Array<string | null> = [];
  const controller = createWorkspaceSessionController({
    deps: deps({
      setWorkspace: async () => workspace("workspace", "remembered"),
      listSessions: async () => [session("remembered")],
    }),
    onActiveSession: (id) => active.push(id),
  });
  await controller.openWorkspace("workspace");
  assert.deepEqual(active, [null, "remembered"]);
  controller.dispose();
});

test("does not restore a matching dead last active session", async () => {
  const active: Array<string | null> = [];
  const controller = createWorkspaceSessionController({
    deps: deps({
      setWorkspace: async () => workspace("workspace", "remembered"),
      listSessions: async () => [session("remembered", "dead", "ended")],
    }),
    onActiveSession: (id) => active.push(id),
  });
  await controller.openWorkspace("workspace");
  assert.deepEqual(active, [null]);
  controller.dispose();
});

test("adopts the restored workspace before polling after a failed switch and retry", async () => {
  let currentWorkspace: WorkspaceInfo | null = workspace("old");
  const observedByList: Array<string | null> = [];
  const published: string[] = [];
  const controller = createWorkspaceSessionController({
    deps: deps({
      setWorkspace: async () => { throw new Error("replacement sidecar failed"); },
      listSessions: async () => {
        observedByList.push(currentWorkspace?.path ?? null);
        return [];
      },
    }),
    onWorkspace: (info) => {
      currentWorkspace = info;
      published.push(info.path);
    },
  });

  await controller.openWorkspace("new");
  assert.equal(currentWorkspace?.path, "old");

  await controller.adoptRestoredWorkspace(workspace("new"));

  assert.deepEqual(published, ["new"]);
  assert.deepEqual(observedByList, ["new"]);
  assert.equal(currentWorkspace?.path, "new");
  controller.dispose();
});

test("deleting the active session clears it deliberately before publishing the refreshed snapshot", async () => {
  const events: string[] = [];
  const controller = createWorkspaceSessionController({
    deps: deps({ listSessions: async () => [session("active")] }),
    onActiveSession: (id, cause) => events.push(`active:${id}${cause?.deliberate ? ":deliberate" : ""}`),
    onSessions: () => events.push("sessions"),
  });
  controller.selectSession("active");
  events.length = 0;
  await controller.deleteSession("active", false);
  assert.deepEqual(events, ["active:null:deliberate", "sessions"]);
  controller.dispose();
});

test("polling prunes terminal attachments before publishing the snapshot", async () => {
  const events: string[] = [];
  const controller = createWorkspaceSessionController({
    deps: deps({
      listSessions: async () => [session("live"), session("dead", "dead", "error")],
      pruneTerminals: (ids) => events.push(`prune:${[...ids].join(",")}`),
    }),
    onSessions: () => events.push("sessions"),
  });
  await controller.refreshSessions();
  assert.deepEqual(events, ["prune:live", "sessions"]);
  controller.dispose();
});

test("workspace transitions reject session admissions and foreground submission", async () => {
  let creates = 0;
  let resumes = 0;
  let foregroundSubmissions = 0;
  const controller = createWorkspaceSessionController({
    deps: deps({
      createSession: async () => {
        creates += 1;
        return session("created", "creating", "creating");
      },
      resumeSession: async (_baseUrl, id) => {
        resumes += 1;
        return session(id);
      },
      setActiveWorkspaceSession: async () => {
        foregroundSubmissions += 1;
      },
    }),
  });

  controller.setAdmissionEnabled(false);

  await assert.rejects(controller.createSession({} satisfies CreateSessionOptions), /workspace transition/i);
  await assert.rejects(controller.resumeSession("remembered"), /workspace transition/i);
  await assert.rejects(controller.submitActiveSession("active"), /workspace transition/i);
  assert.equal(controller.selectSession("active"), false);
  assert.equal(creates, 0);
  assert.equal(resumes, 0);
  assert.equal(foregroundSubmissions, 0);
  controller.dispose();
});

test("disabling admission invalidates an in-flight create", async () => {
  const create = deferred<SessionInfo>();
  const active: Array<string | null> = [];
  const controller = createWorkspaceSessionController({
    deps: deps({ createSession: async () => create.promise }),
    onActiveSession: (id) => active.push(id),
  });

  controller.setAdmissionEnabled(true);
  const pending = controller.createSession({} satisfies CreateSessionOptions);
  controller.setAdmissionEnabled(false);
  create.resolve(session("late", "creating", "creating"));
  await pending;

  assert.deepEqual(active, []);
  controller.dispose();
});

test("foreground session submission returns stale after admission closes during the write", async () => {
  const submission = deferred<void>();
  const controller = createWorkspaceSessionController({
    deps: deps({ setActiveWorkspaceSession: async () => submission.promise }),
  });

  const admission = controller.captureAdmission();
  assert.notEqual(admission, null);
  const handoff = controller.submitActiveSession("active", admission!);
  controller.setAdmissionEnabled(false);
  submission.resolve();

  assert.equal(await handoff, false);
  controller.dispose();
});

test("failed session creation reports failure so the composer can preserve its draft", async () => {
  const errors: string[] = [];
  const controller = createWorkspaceSessionController({
    deps: deps({ createSession: async () => { throw new Error("offline"); } }),
    onError: error => errors.push(error.message),
    scheduler: scheduler().scheduler,
  });
  assert.equal(await controller.createSession({}), false);
  assert.equal(errors.length, 1);
  controller.dispose();
});

test("leaving creation keeps the created session in history without changing the current selection", async () => {
  const response = deferred<SessionInfo>();
  const selected: Array<string | null> = [];
  const snapshots: string[][] = [];
  let stillCreating = true;
  const controller = createWorkspaceSessionController({
    deps: deps({ createSession: () => response.promise }),
    onActiveSession: id => selected.push(id),
    onSessions: sessions => snapshots.push(sessions.map(session => session.id)),
    scheduler: scheduler().scheduler,
  });
  const creating = controller.createSession({}, () => stillCreating);
  controller.selectSession("existing");
  stillCreating = false;
  response.resolve(session("created"));
  assert.equal(await creating, true);
  assert.deepEqual(selected, ["existing"]);
  assert.deepEqual(snapshots, [["created"]]);
  controller.dispose();
});


test("session source availability follows successful polls, failures, and admission loss", async () => {
  const statuses: string[] = [];
  let fail = false;
  const controller = createWorkspaceSessionController({
    deps: deps({listSessions: async () => { if (fail) throw new Error("offline"); return []; }}),
    onSessionSourceStatus: status => statuses.push(status),
  });
  await controller.refreshSessions();
  fail = true;
  await controller.refreshSessions();
  fail = false;
  await controller.refreshSessions();
  controller.setAdmissionEnabled(false);
  assert.deepEqual(statuses, ["ready", "unavailable", "ready", "unavailable"]);
  controller.dispose();
});

test("archive refreshes exact-row state without disposing or selecting a terminal", async () => {
  const active: Array<string | null> = [];
  const disposed: string[] = [];
  const calls: unknown[] = [];
  const controller = createWorkspaceSessionController({
    deps: deps({
      sessionAction: async (...args) => { calls.push(args); },
      listSessions: async () => [{ ...session("one", "dead", "ended"), sessionState: "archived", sessionActionToken: "new" }],
      disposeTerminal: id => disposed.push(id),
    }),
    onActiveSession: id => active.push(id),
  });
  controller.selectSession("one");
  await controller.sessionAction("one", "archive", "captured");
  assert.deepEqual(calls, [["http://backend", "one", "archive", "captured"]]);
  assert.deepEqual(disposed, []);
  assert.deepEqual(active, ["one"]);
  controller.dispose();
});

test("an old workspace action cannot clear a new workspace selection", async () => {
  const operation = deferred<void>();
  const active: Array<string | null> = [];
  const disposed: string[] = [];
  const controller = createWorkspaceSessionController({
    deps: deps({ sessionAction: async () => operation.promise, disposeTerminal: id => disposed.push(id) }),
    onActiveSession: id => active.push(id),
  });
  controller.selectSession("one");
  const deleting = controller.sessionAction("one", "delete", "captured");
  await new Promise(resolve => setImmediate(resolve));
  await controller.openWorkspace("/new");
  controller.selectSession("one");
  operation.resolve();
  await deleting;
  assert.equal(active.at(-1), "one");
  assert.deepEqual(disposed, []);
  controller.dispose();
});


test("failed Delete reconciles a removed row while preserving the cleanup error", async () => {
  const active: Array<string | null> = [];
  const disposed: string[] = [];
  const controller = createWorkspaceSessionController({
    deps: deps({
      sessionAction: async () => { throw new Error("Recommendation cleanup failed"); },
      listSessions: async () => [],
      disposeTerminal: id => disposed.push(id),
    }),
    onActiveSession: id => active.push(id),
  });
  controller.selectSession("one");
  await assert.rejects(controller.sessionAction("one", "delete", "token"), /Recommendation cleanup failed/);
  assert.deepEqual(active, ["one", null]);
  assert.deepEqual(disposed, ["one"]);
  controller.dispose();
});

test("failed Delete keeps a retained row and any newer selection", async () => {
  const active: Array<string | null> = [];
  const disposed: string[] = [];
  const controller = createWorkspaceSessionController({
    deps: deps({ sessionAction: async () => { throw new Error("Close is still finishing"); },
      listSessions: async () => [session("one")], disposeTerminal: id => disposed.push(id) }),
    onActiveSession: id => active.push(id),
  });
  controller.selectSession("other");
  await assert.rejects(controller.sessionAction("one", "delete", "token"), /Close is still finishing/);
  assert.deepEqual(active, ["other"]);
  assert.deepEqual(disposed, []);
  controller.dispose();
});


test("partial Delete preserves selection changed while the request was pending", async () => {
  const operation = deferred<void>();
  const active: Array<string | null> = [];
  const disposed: string[] = [];
  const controller = createWorkspaceSessionController({
    deps: deps({ sessionAction: async () => operation.promise,
      listSessions: async () => [session("other")], disposeTerminal: id => disposed.push(id) }),
    onActiveSession: id => active.push(id),
  });
  controller.selectSession("one");
  const deleting = controller.sessionAction("one", "delete", "token");
  await new Promise(resolve => setImmediate(resolve));
  controller.selectSession("other");
  operation.reject(new Error("Recommendation cleanup failed"));
  await assert.rejects(deleting, /Recommendation cleanup failed/);
  assert.deepEqual(active, ["one", "other"]);
  assert.deepEqual(disposed, ["one"]);
  controller.dispose();
});
