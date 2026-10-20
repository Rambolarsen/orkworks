// orkworks:harness-integration:v2:opencode
// Installed by OrkWorks (Settings > Coding Tools). Reinstall from there
// instead of hand-editing — OrkWorks treats this file as byte-owned and
// will overwrite local edits when it detects drift.
//
// OpenCode 2 plugin API: this file default-exports a definition with a
// stable `id` and a `setup(ctx)` function. OpenCode 2 stopped loading the
// V1 named-factory + returned-hooks shape entirely ("V1 plugin
// implementations do not run in V2"), and a configured `plugins` entry
// must be a plugin *directory* — a bare `.opencode/plugins/` file like
// this one is only reached through V2's auto-discovery of that directory.
//
// Session lifecycle events arrive through `ctx.event.subscribe()`. The V2
// envelope carries the payload under `data` (V1 used `properties`),
// verified against a live OpenCode 2.0.32 session on 2026-10-19:
//   - every session-scoped event carries `data.sessionID`
//   - `session.step.started` / `session.step.ended` bracket a model step
//   - `session.execution.succeeded` closes a turn; no `session.idle` or
//     `session.status` event was observed in V2 run mode, and no
//     `session.created` event either — the native session ID is captured
//     from the first session-scoped event instead
//   - `permission.asked` carries `{ id, sessionID, ... }` and
//     `permission.replied` carries `{ requestID, sessionID, reply }`
//     (live fixture); `question.asked`/`question.replied`/
//     `question.rejected` are handled with the same id/requestID shape
//
// Attention follows explicit permission and question request lifecycles.
// A turn boundary without a pending request maps to plain idle. The
// capture attention POST carries the normalized event name
// `session.capture` (not the triggering OpenCode event) so the sidecar's
// event/status matrix stays a small exact allow-list.
export default {
  id: 'orkworks.session-reporter',
  setup: async (ctx) => {
    let openCodeSessionId = null;
    let turnStatus = 'idle';
    let lastReported = null;
    let lastMicros = 0;
    const permissionIds = new Set();
    const questionIds = new Set();

    const effectiveAttention = () => {
      if (permissionIds.size && questionIds.size) {
        return { status: 'waiting_for_input', message: 'OpenCode needs an answer or permission decision' };
      }
      if (permissionIds.size) {
        return { status: 'waiting_for_input', message: 'OpenCode is asking for a permission decision' };
      }
      if (questionIds.size) {
        return { status: 'waiting_for_input', message: 'OpenCode is asking for an answer' };
      }
      return { status: turnStatus };
    };

    const postAttention = async (event) => {
      const current = effectiveAttention();
      if (lastReported?.status === current.status && lastReported?.message === current.message) return;
      lastReported = current;
      const port = process.env.ORKWORKS_PORT;
      const orkworksSessionId = process.env.ORKWORKS_SESSION_ID;
      const token = process.env.ORKWORKS_REPORT_TOKEN;
      if (!port || !orkworksSessionId || !token) return;
      const micros = Math.max(
        Math.floor((performance.timeOrigin + performance.now()) * 1000),
        lastMicros + 1,
      );
      lastMicros = micros;
      const fraction = String(micros % 1_000_000).padStart(6, '0');
      const observedAt = new Date(Math.floor(micros / 1000))
        .toISOString().replace(/\.\d{3}Z$/, `.${fraction}Z`);
      const payload = { ...current, observedAt, source: 'opencode_hook', event };
      await fetch(`http://127.0.0.1:${port}/sessions/${orkworksSessionId}/attention`, {
        method: "POST",
        headers: { "Content-Type": "application/json", Authorization: `Bearer ${token}` },
        body: JSON.stringify(payload),
      }).catch(() => {});
    };

    const resetCaptureState = (harnessSessionId) => {
      openCodeSessionId = harnessSessionId;
      turnStatus = 'idle';
      permissionIds.clear();
      questionIds.clear();
      lastReported = null;
    };

    const postHarnessSession = async () => {
      const port = process.env.ORKWORKS_PORT;
      const orkworksSessionId = process.env.ORKWORKS_SESSION_ID;
      if (!port || !orkworksSessionId) return;
      await fetch(`http://127.0.0.1:${port}/sessions/${orkworksSessionId}/harness-session`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          harnessSessionId: openCodeSessionId,
          source: "opencode_hook",
          confidence: 0.99,
        }),
      }).catch(() => {});
    };

    const handleEvent = async (event) => {
      const type = event?.type;
      const data = event?.data;
      if (typeof type !== 'string' || typeof data !== 'object' || data === null) return;
      const sessionID = data.sessionID;
      // An explicit session.created announcement re-captures (matching the
      // V1 behavior for a TUI hosting several sessions); otherwise the
      // first session-scoped event establishes the capture, because V2 run
      // mode emitted no session.created at all. Teardown events for a
      // session that is already gone must not bind the capture.
      if (type === 'session.created') {
        const capturedId = typeof data.info?.id === 'string' && data.info.id
          ? data.info.id
          : (typeof sessionID === 'string' && sessionID ? sessionID : null);
        if (!capturedId || capturedId === openCodeSessionId) return;
        resetCaptureState(capturedId);
        await postHarnessSession();
        // The capture attention POST carries a normalized event name so the
        // sidecar's event/status matrix stays a small exact allow-list
        // regardless of which OpenCode event happened to arrive first.
        await postAttention('session.capture');
        return;
      }
      if (!openCodeSessionId) {
        if (typeof sessionID !== 'string' || !sessionID || !type.startsWith('session.')) return;
        if (type === 'session.deleted' || type === 'session.error') return;
        resetCaptureState(sessionID);
        await postHarnessSession();
        await postAttention('session.capture');
        // The capture event still flows through the switch below so the
        // triggering event (e.g. session.step.started) can immediately
        // update the turn state it reports.
      }
      // Attention events only apply to the captured session — a TUI can
      // host several OpenCode sessions, and stale ones must not steer
      // OrkWorks' attention state.
      if (sessionID !== openCodeSessionId) return;
      const { id, requestID } = data;
      switch (type) {
        case 'session.execution.succeeded':
        case 'session.execution.failed':
          turnStatus = 'idle';
          break;
        case 'session.execution.interrupted':
          // A shutdown interruption keeps OpenCode's execution claim — the
          // resumed drain continues the turn — so only a non-shutdown
          // interruption (user cancel) is a turn boundary.
          if (data.reason === 'shutdown') return;
          turnStatus = 'idle';
          break;
        case 'session.step.started':
          turnStatus = 'working';
          break;
        case 'permission.asked':
        case 'question.asked':
          if (typeof id !== 'string' || !id) return;
          (type === 'permission.asked' ? permissionIds : questionIds).add(id);
          break;
        case 'permission.replied':
        case 'question.replied':
        case 'question.rejected':
          if (typeof requestID !== 'string' || !requestID) return;
          if (!(type === 'permission.replied' ? permissionIds : questionIds).delete(requestID)) return;
          break;
        default:
          return;
      }
      await postAttention(type);
    };

    const controller = new AbortController();
    void (async () => {
      try {
        for await (const event of ctx.event.subscribe({ signal: controller.signal })) {
          // A single bad event must not kill the subscription for the
          // plugin's lifetime — isolate the loop body per event.
          try {
            await handleEvent(event);
          } catch (error) {
            console.error('[orkworks.session-reporter] event handling failed:', error);
          }
        }
      } catch (error) {
        if (!controller.signal.aborted) {
          // The subscription must not die silently: report one diagnostics
          // line through the same channel OpenCode uses for plugin errors.
          console.error('[orkworks.session-reporter] event subscription failed:', error);
        }
      }
    })();
    return () => controller.abort();
  },
};
