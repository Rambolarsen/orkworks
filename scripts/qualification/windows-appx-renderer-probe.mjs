import { setTimeout as delay } from "node:timers/promises";

// This function is evaluated inside the installed app's renderer. Keep the
// renderer credential and responses containing local data inside that process.
async function qualifyWorkspace(workspace, retainedSessionId) {
  const bridge = window.orkworks;
  if (!bridge) throw new Error("The desktop preload bridge is unavailable");

  if (retainedSessionId) {
    const { recent, pinned } = await bridge.getWorkspaceHistory();
    if (![...recent, ...pinned].some((path) => path.toLowerCase() === workspace.toLowerCase())) {
      throw new Error("The upgraded desktop did not remember the test workspace");
    }
  }
  if (!(await bridge.openRememberedWorkspace(workspace))) {
    throw new Error("The desktop did not open the test workspace");
  }

  const backend = await bridge.getBackendUrl();
  const token = bridge.getBackendToken();
  async function request(path, options = {}) {
    const response = await fetch(backend + path, {
      ...options,
      headers: { "x-orkworks-renderer-token": token, ...options.headers },
      signal: AbortSignal.timeout(5_000),
    });
    if (!response.ok) throw new Error("Sidecar request failed with HTTP " + response.status);
    return response;
  }
  async function waitForSession(id, lifecycle) {
    for (let attempt = 0; attempt < 30; attempt++) {
      const sessions = await (await request("/sessions")).json();
      if (sessions.some((session) => session.id === id && session.lifecycle === lifecycle)) return;
      await new Promise((resolve) => setTimeout(resolve, 300));
    }
    throw new Error("The test shell session did not reach lifecycle " + lifecycle);
  }

  if (retainedSessionId) {
    await waitForSession(retainedSessionId, "dead");
    return true;
  }

  const session = await (await request("/sessions", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ harnessId: "generic-shell" }),
  })).json();
  try {
    await waitForSession(session.id, "alive");
  } finally {
    await request("/sessions/" + encodeURIComponent(session.id), { method: "DELETE" });
  }
  await waitForSession(session.id, "dead");
  return session.id;
}

async function findRenderer(port) {
  for (let attempt = 0; attempt < 60; attempt++) {
    let targets = [];
    try {
      const response = await fetch("http://127.0.0.1:" + port + "/json/list", {
        signal: AbortSignal.timeout(1_000),
      });
      if (response.ok) targets = await response.json();
    } catch {
      // Chromium can show its window before the debugger is ready.
    }
    const page = targets.find(({ type, url }) =>
      type === "page" && url.startsWith("file:") && url.endsWith("/dist/index.html"));
    if (page) {
      const socketUrl = new URL(page.webSocketDebuggerUrl);
      if (socketUrl.protocol !== "ws:" || socketUrl.hostname !== "127.0.0.1" || socketUrl.port !== port) {
        throw new Error("The renderer debugger did not return a loopback endpoint");
      }
      return socketUrl;
    }
    await delay(500);
  }
  throw new Error("The installed desktop renderer did not expose its debugger");
}

async function evaluate(socketUrl, expression) {
  const socket = new WebSocket(socketUrl);
  try {
    return await new Promise((resolve, reject) => {
      const timeout = setTimeout(() => reject(new Error("The desktop probe timed out")), 60_000);
      socket.addEventListener("open", () => {
        socket.send(JSON.stringify({
          id: 1,
          method: "Runtime.evaluate",
          params: { expression, awaitPromise: true, returnByValue: true },
        }));
      }, { once: true });
      socket.addEventListener("error", () => reject(new Error("The renderer debugger disconnected")), { once: true });
      socket.addEventListener("close", () => reject(new Error("The renderer debugger closed")), { once: true });
      socket.addEventListener("message", ({ data }) => {
        const reply = JSON.parse(data);
        if (reply.id !== 1) return;
        clearTimeout(timeout);
        if (reply.error || reply.result.exceptionDetails) {
          reject(new Error(reply.result?.exceptionDetails?.exception?.description ?? "Renderer evaluation failed"));
          return;
        }
        resolve(reply.result.result.value);
      });
      socket.addEventListener("close", () => clearTimeout(timeout), { once: true });
    });
  } finally {
    socket.close();
  }
}

const [, , port, workspace, retainedSessionId] = process.argv;
const expression = "(" + qualifyWorkspace.toString() + ")(" +
  JSON.stringify(workspace) + "," + JSON.stringify(retainedSessionId ?? null) + ")";
const result = await evaluate(await findRenderer(port), expression);
if (retainedSessionId) {
  if (result !== true) throw new Error("The upgraded desktop probe did not complete");
} else {
  if (typeof result !== "string" || !result) throw new Error("The desktop did not return a shell session ID");
  console.log(result);
}
