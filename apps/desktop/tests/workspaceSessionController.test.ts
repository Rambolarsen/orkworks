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
    repo_root: path,
    branch: "main",
    dirty: false,
    lastActiveSessionId,
    activeHarnessIds: [],
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
    setActiveWorkspaceSession: async () => {},
    pruneTerminals: () => {},
    disposeTerminal: () => {},
    ...overrides,
  };
}

test("the controller owns one polling loop and starts no second loop per refresh", async () => {
  const polling = scheduler();
  let lists = 0;
  const controller = createWorkspaceSessionController({
    deps: deps({ listSessions: async () => { lists += 1; return []; } }),
    scheduler: polling.scheduler,
  });

  await controller.refreshSessions();
  assert.equal(lists, 2);
  assert.equal(polling.callbacks.length, 1);
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

test("restores last active session only after a matching live snapshot", async () => {
  const active: Array<string | null> = [];
  const controller = createWorkspaceSessionController({
    deps: deps({
      setWorkspace: async () => workspace("workspace", "remembered"),
      listSessions: async () => [session("other")],
    }),
    onActiveSession: (id) => active.push(id),
  });
  await controller.openWorkspace("workspace");
  assert.deepEqual(active, [null]);
  controller.dispose();
});

test("deleting the active session clears it before publishing the refreshed snapshot", async () => {
  const events: string[] = [];
  const controller = createWorkspaceSessionController({
    deps: deps({ listSessions: async () => [session("active")] }),
    onActiveSession: (id) => events.push(`active:${id}`),
    onSessions: () => events.push("sessions"),
  });
  controller.selectSession("active");
  events.length = 0;
  await controller.deleteSession("active", false);
  assert.deepEqual(events, ["active:null", "sessions"]);
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
