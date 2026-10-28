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

test("a list started during creation cannot remove the new card or prune its terminal", async () => {
  const response = deferred<SessionInfo>();
  const oldList = deferred<SessionInfo[]>();
  const snapshots: string[][] = [];
  const pruned: string[][] = [];
  const errors: string[] = [];
  let serverRows = [session("existing")];
  let listPending = false;
  const controller = createWorkspaceSessionController({
    deps: deps({
      createSession: () => response.promise,
      listSessions: () => listPending ? oldList.promise : Promise.resolve(serverRows),
      pruneTerminals: ids => pruned.push([...ids]),
    }),
    onSessions: rows => snapshots.push(rows.map(row => row.id)),
    onError: error => errors.push(error.key),
    scheduler: scheduler().scheduler,
  });
  await controller.refreshSessions();
  const creating = controller.createSession({});
  listPending = true;
  const staleRefresh = controller.refreshSessions();
  await new Promise(resolve => setImmediate(resolve));
  response.resolve(session("created", "creating", "creating"));
  assert.equal(await creating, true);
  assert.deepEqual(new Set(snapshots.at(-1)), new Set(["existing", "created"]));
  const published = snapshots.length;
  const pruneCalls = pruned.length;

  oldList.resolve([session("existing")]);
  await staleRefresh;
  assert.equal(snapshots.length, published, "a pre-create snapshot must not erase the new card");
  assert.equal(pruned.length, pruneCalls, "a pre-create snapshot must not detach the new terminal");

  listPending = false;
  serverRows = [session("existing"), session("created", "dead", "error")];
  await controller.refreshSessions();
  assert.deepEqual(new Set(snapshots.at(-1)), new Set(["existing", "created"]));
  assert.deepEqual(errors, ["create"], "the stale list must not lose startup failure tracking");
  serverRows = [session("existing")];
  await controller.refreshSessions();
  assert.deepEqual(snapshots.at(-1), ["existing"], "a fresh authoritative removal still applies");
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

for (const pending of ["create", "resume"] as const) {
  for (const action of ["archive", "delete"] as const) {
    test(`row ${action} fences a pending ${pending} response already exposed by polling`, async () => {
      const response = deferred<SessionInfo>();
      const original = { ...session("one"), sessionState: "active" as const, sessionActionToken: "active-token" };
      const archived = { ...session("one", "dead", "ended"), sessionState: "archived" as const, sessionActionToken: "archived-token" };
      let serverRows: SessionInfo[] = [original, session("other")];
      let latest: readonly SessionInfo[] = [];
      const active: Array<string | null> = [];
      const controller = createWorkspaceSessionController({
        deps: deps({
          createSession: () => response.promise,
          resumeSession: () => response.promise,
          listSessions: async () => serverRows,
          sessionAction: async () => { serverRows = action === "delete" ? [session("other")] : [archived, session("other")]; },
        }),
        onSessions: rows => { latest = rows; },
        onActiveSession: id => active.push(id),
        scheduler: scheduler().scheduler,
      });
      const operation = pending === "create" ? controller.createSession({}) : controller.resumeSession("one");
      await new Promise(resolve => setImmediate(resolve));
      await controller.refreshSessions();
      controller.selectSession("other");
      await controller.sessionAction("one", action, "active-token");
      active.length = 0;
      response.resolve(original);
      await operation;
      assert.deepEqual(active, [], "the older completion must not select its row");
      assert.equal(latest.find(row => row.id === "one")?.sessionState, action === "delete" ? undefined : "archived");
      controller.dispose();
    });
  }
}

test("pruning excludes archived stale runtime rows and restoration does not select them", async () => {
  const kept: string[][] = [];
  const active: Array<string|null> = [];
  const rows = [{...session("saved"),sessionState:"archived" as const}, session("live")];
  const controller = createWorkspaceSessionController({
    deps:deps({listSessions:async()=>rows,pruneTerminals:ids=>kept.push([...ids])}),
    onActiveSession:id=>active.push(id),
  });
  await controller.adoptRestoredWorkspace(workspace("/same", "saved"));
  assert.deepEqual(kept, [["live"]]);
  assert.deepEqual(active, [null]);
  controller.dispose();
});

for (const outcome of ["success","error"] as const) {
  for (const changedSelection of [false,true]) {
    test(`Restore ${outcome} reconciles retention removal with changed selection=${changedSelection}`, async () => {
      const refreshed = deferred<SessionInfo[]>();
      const active: Array<string|null> = [];
      const disposed: string[] = [];
      const controller = createWorkspaceSessionController({
        deps:deps({
          sessionAction:async()=>{if(outcome==="error")throw new Error("state save failed");},
          listSessions:()=>refreshed.promise,
          disposeTerminal:id=>disposed.push(id),
        }),
        onActiveSession:id=>active.push(id),
      });
      controller.selectSession("one");
      const restoring = controller.sessionAction("one","restore","token");
      const settled = outcome==="error" ? assert.rejects(restoring,/state save failed/) : restoring;
      await new Promise(resolve=>setImmediate(resolve));
      if(changedSelection) controller.selectSession("other");
      refreshed.resolve([session("other")]);
      await settled;
      assert.deepEqual(disposed, ["one"]);
      assert.deepEqual(active, changedSelection ? ["one","other"] : ["one",null]);
      controller.dispose();
    });
  }
}

test("Restore makes no missing-row inference from an unavailable refresh", async () => {
  const active: Array<string|null> = [];
  const disposed: string[] = [];
  const controller=createWorkspaceSessionController({
    deps:deps({listSessions:async()=>{throw new Error("offline");},disposeTerminal:id=>disposed.push(id)}),
    onActiveSession:id=>active.push(id),
  });
  controller.selectSession("one");
  await controller.sessionAction("one","restore","token");
  assert.deepEqual(active,["one"]);
  assert.deepEqual(disposed,[]);
  controller.dispose();
});


test("healthy update stream fences an older REST response and disconnect restores polling", async () => {
  const stale = deferred<SessionInfo[]>();
  let streamOptions: Parameters<NonNullable<WorkspaceSessionControllerDeps["startUpdates"]>>[0] | undefined;
  let calls = 0;
  const lists: string[][] = [];
  const timing = scheduler();
  const controller = createWorkspaceSessionController({
    scheduler: timing.scheduler,
    deps: deps({
      listSessions: async () => { calls++; return calls === 2 ? stale.promise : [session("current")]; },
      startUpdates: options => { streamOptions = options; options.onHealthy(false); return { dispose() {}, resync() {} }; },
    }),
    onSessions: rows => lists.push(rows.map(row => row.id)),
  });
  await controller.adoptRestoredWorkspace(workspace("/workspace"));
  controller.setPollingEnabled(true);
  await Promise.resolve(); await Promise.resolve();
  assert.ok(streamOptions);
  streamOptions.onHealthy(true); streamOptions.onSessions([session("new")]);
  stale.resolve([session("stale")]); await Promise.resolve(); await Promise.resolve();
  assert.deepEqual(lists.at(-1), ["new"]);
  const before = calls; streamOptions.onHealthy(false);
  await Promise.resolve(); await Promise.resolve();
  assert.ok(calls > before);
  controller.dispose();
});

for (const opening of ["adopt", "open"] as const) {
  for (const userNavigates of [false, true]) {
    test(`${opening}: stream winning initial REST restores the remembered session; navigation=${userNavigates}`, async () => {
      const delayed = deferred<SessionInfo[]>();
      const started = deferred<Parameters<NonNullable<WorkspaceSessionControllerDeps["startUpdates"]>>[0]>();
      const active: Array<string | null> = [];
      let controller!: ReturnType<typeof createWorkspaceSessionController>;
      controller = createWorkspaceSessionController({
        scheduler: scheduler().scheduler,
        deps: deps({
          setWorkspace: async () => workspace("/workspace", "remembered"),
          listSessions: () => delayed.promise,
          startUpdates: options => { started.resolve(options); options.onHealthy(false); return { dispose() {}, resync() {} }; },
        }),
        onWorkspace: value => { if (value) controller.setPollingEnabled(true); },
        onActiveSession: id => active.push(id),
      });
      const openingPromise = opening === "adopt"
        ? controller.adoptRestoredWorkspace(workspace("/workspace", "remembered"))
        : controller.openWorkspace("/workspace");
      const stream = await started.promise;
      if (userNavigates) controller.selectSession("other");
      stream.onHealthy(true); stream.onSessions([session("remembered"), session("other")]);
      assert.equal(active.at(-1), userNavigates ? "other" : "remembered");
      delayed.resolve([]); await openingPromise;
      assert.equal(active.at(-1), userNavigates ? "other" : "remembered");
      controller.dispose();
    });
  }
}

for (const opening of ["adopt", "open"] as const) {
  test(`${opening}: REST restoration publishes the matching list before its active session`, async () => {
    let published: readonly SessionInfo[] = [];
    const active: Array<string|null> = [];
    const controller = createWorkspaceSessionController({
      deps: deps({ setWorkspace: async () => workspace("/workspace", "remembered"), listSessions: async () => [session("remembered")] }),
      onSessions: rows => { published = rows; },
      onActiveSession: id => { if (id) assert.ok(published.some(row => row.id === id), "restore must never publish an active ID before its list"); active.push(id); },
    });
    if (opening === "adopt") await controller.adoptRestoredWorkspace(workspace("/workspace", "remembered"));
    else await controller.openWorkspace("/workspace");
    assert.equal(active.at(-1), "remembered");
    controller.dispose();
  });
}

for (const action of ["restore", "delete"] as const) {
  for (const fails of [false,true]) {
    if (action === "delete" && !fails) continue;
    for (const changesSelection of [false,true]) {
      test(`${action} error=${fails}: accepted stream reconciles removed target before stale REST; navigation=${changesSelection}`, async () => {
        const oldList = deferred<SessionInfo[]>();
        const request = deferred<void>();
        let streamOptions!: Parameters<NonNullable<WorkspaceSessionControllerDeps["startUpdates"]>>[0];
        const active: Array<string|null> = [], disposed: string[] = [];
        let delayLists = false;
        const controller = createWorkspaceSessionController({
          scheduler: scheduler().scheduler,
          deps: deps({
            listSessions: async () => delayLists ? oldList.promise : [session("target"), session("other")],
            sessionAction: async () => { await request.promise; if (fails) throw new Error("Action partially failed"); },
            disposeTerminal: id => disposed.push(id),
            startUpdates: options => {
              streamOptions = options; options.onHealthy(false);
              return { dispose() {}, resync() { options.onHealthy(false); } };
            },
          }),
          onActiveSession: id => active.push(id),
        });
        await controller.adoptRestoredWorkspace(workspace("/workspace"));
        controller.setPollingEnabled(true); await Promise.resolve(); await Promise.resolve();
        streamOptions.onHealthy(true); streamOptions.onSessions([session("target"), session("other")]);
        controller.selectSession("target");
        delayLists = true;
        const operation = controller.sessionAction("target", action, "token").catch(error => error);
        request.resolve();
        await new Promise(resolve => setImmediate(resolve));
        if (changesSelection) controller.selectSession("other");
        streamOptions.onHealthy(true); streamOptions.onSessions([session("other")]);
        assert.deepEqual(disposed,["target"], "accepted authority must reconcile even while REST is pending");
        assert.equal(active.at(-1), changesSelection ? "other" : null);
        oldList.resolve([session("target"), session("other")]); await operation;
        assert.deepEqual(disposed,["target"]); assert.equal(active.at(-1), changesSelection ? "other" : null);
        controller.dispose();
      });
    }
  }
}

for (const changesSelection of [false,true]) {
  test(`pre-mutation fallback cannot consume post-action stream reconciliation; navigation=${changesSelection}`, async () => {
    const earlier = deferred<SessionInfo[]>(), actionList = deferred<SessionInfo[]>(), request = deferred<void>();
    const timing = scheduler();
    let phase = "initial", resyncs = 0;
    let stream!: Parameters<NonNullable<WorkspaceSessionControllerDeps["startUpdates"]>>[0];
    const active: Array<string|null> = [], disposed: string[] = [];
    const controller = createWorkspaceSessionController({
      scheduler: timing.scheduler,
      deps: deps({
        listSessions: async () => phase === "during" ? earlier.promise : phase === "after" ? actionList.promise : [session("target"),session("other")],
        sessionAction: () => request.promise,
        disposeTerminal: id => disposed.push(id),
        startUpdates: options => { stream = options; options.onHealthy(false); return { dispose() {}, resync() { resyncs++; options.onHealthy(false); } }; },
      }),
      onActiveSession: id => active.push(id),
    });
    await controller.adoptRestoredWorkspace(workspace("/workspace"));
    controller.setPollingEnabled(true); await new Promise(resolve=>setImmediate(resolve));
    controller.selectSession("target");
    const operation = controller.sessionAction("target","restore","token");
    phase = "during";
    assert.ok(timing.callbacks.length); timing.callbacks.at(-1)!();
    await new Promise(resolve=>setImmediate(resolve));
    phase = "after"; request.resolve(); await new Promise(resolve=>setImmediate(resolve));
    earlier.resolve([session("target"),session("other")]); await new Promise(resolve=>setImmediate(resolve));
    if (changesSelection) controller.selectSession("other");
    stream.onHealthy(true); stream.onSessions([session("other")]);
    assert.deepEqual(disposed,["target"]);
    assert.equal(active.at(-1),changesSelection ? "other" : null);
    assert.ok(resyncs > 0,"post-action refresh must fence an initial socket query that is still connecting");
    actionList.resolve([session("target"),session("other")]); await operation;
    assert.deepEqual(disposed,["target"]); controller.dispose();
  });
}


for (const replacement of ["admission", "foreground", "workspace", "source", "disposal"] as const) {
  for (const outcome of ["success", "failure"] as const) {
    test(`replaced ${replacement} identity ignores an old refresh ${outcome}`, async () => {
      const old = deferred<SessionInfo[]>();
      const snapshots: string[][] = [];
      const statuses: string[] = [];
      let calls = 0;
      const controller = createWorkspaceSessionController({
        deps: deps({ listSessions: async () => ++calls === 1 ? old.promise : [session("current")] }),
        onSessions: rows => snapshots.push(rows.map(row => row.id)),
        onSessionSourceStatus: status => statuses.push(status),
      });
      const pending = controller.refreshSessions();
      await new Promise(resolve => setImmediate(resolve));
      if (replacement === "admission") {
        controller.setAdmissionEnabled(false);
        controller.setAdmissionEnabled(true);
      } else if (replacement === "foreground") await controller.createSession({});
      else if (replacement === "workspace") await controller.adoptRestoredWorkspace(workspace("/new"));
      else if (replacement === "source") controller.setPollingEnabled(false);
      else controller.dispose();
      if (replacement !== "disposal") await controller.refreshSessions();
      const currentSnapshots = snapshots.slice();
      const currentStatuses = statuses.slice();
      if (outcome === "success") old.resolve([session("obsolete")]);
      else old.reject(new Error("obsolete failure"));
      assert.equal(await pending, null);
      assert.deepEqual(snapshots, currentSnapshots);
      assert.deepEqual(statuses, currentStatuses);
      controller.dispose();
    });
  }
}

test("an old REST failure cannot mark a healthy stream snapshot unavailable", async () => {
  const old = deferred<SessionInfo[]>();
  let stream: Parameters<NonNullable<WorkspaceSessionControllerDeps["startUpdates"]>>[0] | undefined;
  let calls = 0;
  const statuses: string[] = [];
  const snapshots: string[][] = [];
  const controller = createWorkspaceSessionController({
    scheduler: scheduler().scheduler,
    deps: deps({
      listSessions: async () => ++calls === 2 ? old.promise : [],
      startUpdates: options => { stream = options; return { dispose() {}, resync() {} }; },
    }),
    onSessions: rows => snapshots.push(rows.map(row => row.id)),
    onSessionSourceStatus: status => statuses.push(status),
  });
  await controller.adoptRestoredWorkspace(workspace("/workspace"));
  controller.setPollingEnabled(true);
  await new Promise(resolve => setImmediate(resolve));
  assert.ok(stream);
  stream.onHealthy(true);
  stream.onSessions([session("stream-current")]);
  statuses.length = 0;
  old.reject(new Error("obsolete REST failure"));
  await new Promise(resolve => setImmediate(resolve));
  assert.deepEqual(statuses, []);
  assert.deepEqual(snapshots.at(-1), ["stream-current"]);
  controller.dispose();
});


for (const outcome of ["success", "failure"] as const) {
  test(`an older overlapping REST ${outcome} cannot replace a newer refresh`, async () => {
    const first = deferred<SessionInfo[]>(), second = deferred<SessionInfo[]>();
    const snapshots: string[][] = [], statuses: string[] = [];
    let calls = 0;
    const controller = createWorkspaceSessionController({
      deps: deps({ listSessions: () => ++calls === 1 ? first.promise : second.promise }),
      onSessions: rows => snapshots.push(rows.map(row => row.id)),
      onSessionSourceStatus: status => statuses.push(status),
    });
    const old = controller.refreshSessions();
    const current = controller.refreshSessions();
    second.resolve([session("newer")]);
    await current;
    if (outcome === "success") first.resolve([session("older")]);
    else first.reject(new Error("old failure"));
    assert.equal(await old, null);
    assert.deepEqual(snapshots, [["newer"]]);
    assert.deepEqual(statuses, ["ready"]);
    controller.dispose();
  });
}

test("a new stream snapshot fences an explicit REST refresh started while the stream was already healthy", async () => {
  const pending = deferred<SessionInfo[]>();
  let stream: Parameters<NonNullable<WorkspaceSessionControllerDeps["startUpdates"]>>[0] | undefined;
  let defer = false;
  const snapshots: string[][] = [];
  const controller = createWorkspaceSessionController({
    scheduler: scheduler().scheduler,
    deps: deps({
      listSessions: async () => defer ? pending.promise : [],
      startUpdates: options => { stream = options; return { dispose() {}, resync() {} }; },
    }),
    onSessions: rows => snapshots.push(rows.map(row => row.id)),
  });
  await controller.adoptRestoredWorkspace(workspace("/workspace"));
  controller.setPollingEnabled(true);
  assert.ok(stream);
  stream.onHealthy(true);
  await new Promise(resolve => setImmediate(resolve));
  defer = true;
  const refresh = controller.refreshSessions();
  await new Promise(resolve => setImmediate(resolve));
  stream.onSessions([session("stream-newer")]);
  pending.resolve([session("REST-older")]);
  assert.equal(await refresh, null);
  assert.deepEqual(snapshots.at(-1), ["stream-newer"]);
  controller.dispose();
});

for (const outcome of ["success", "failure"] as const) {
  test(`resume publishes immediate progress, suppresses duplicates and clears after ${outcome}`, async () => {
    const response = deferred<SessionInfo>();
    const progress: string[][] = [];
    const errors: string[] = [];
    let requests = 0;
    const controller = createWorkspaceSessionController({
      deps: deps({ resumeSession: async () => { requests++; return response.promise; } }),
      onResumingSessions: ids => progress.push([...ids]),
      onError: error => errors.push(error.message),
    });
    const operation = controller.resumeSession("one");
    assert.deepEqual(progress, [["one"]], "progress precedes backend readiness");
    const duplicate = controller.resumeSession("one");
    await new Promise(resolve => setImmediate(resolve));
    assert.equal(requests, 1);
    if (outcome === "success") response.resolve(session("one"));
    else response.reject(new Error("startup failed"));
    await Promise.all([operation, duplicate]);
    assert.deepEqual(progress, [["one"], []]);
    assert.equal(errors.length, outcome === "failure" ? 1 : 0);
    controller.dispose();
  });
}

test("workspace replacement clears resume progress and an old response cannot clear a new resume", async () => {
  const oldResponse = deferred<SessionInfo>();
  const newResponse = deferred<SessionInfo>();
  const progress: string[][] = [];
  let requests = 0;
  const controller = createWorkspaceSessionController({
    deps: deps({ resumeSession: async () => ++requests === 1 ? oldResponse.promise : newResponse.promise }),
    onResumingSessions: ids => progress.push([...ids]),
  });
  const oldOperation = controller.resumeSession("one");
  await new Promise(resolve => setImmediate(resolve));
  controller.setAdmissionEnabled(false);
  assert.deepEqual(progress.at(-1), []);
  controller.setAdmissionEnabled(true);
  await controller.openWorkspace("/replacement");
  const newOperation = controller.resumeSession("one");
  await new Promise(resolve => setImmediate(resolve));
  oldResponse.resolve(session("one"));
  await oldOperation;
  assert.deepEqual(progress.at(-1), ["one"]);
  newResponse.resolve(session("one"));
  await newOperation;
  assert.deepEqual(progress.at(-1), []);
  controller.dispose();
});
