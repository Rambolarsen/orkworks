// orkworks:harness-integration:v2:opencode
// Installed by OrkWorks (Settings > Coding Tools). Reinstall from there
// instead of hand-editing — OrkWorks treats this file as byte-owned and
// will overwrite local edits when it detects drift.
//
// Session lifecycle events (including "session.created") are not individual
// top-level hook keys — they arrive through the single generic `event` hook
// as `{ event }` with `event.type`, and the OpenCode session's ID lives at
// `event.properties.info.id` (verified against @opencode-ai/sdk's
// EventSessionCreated / Session type definitions, not just the plugin docs
// prose).
//
// Attention follows explicit permission and question request lifecycles.
// A turn boundary without a pending request maps to plain idle.
export const OrkWorksSessionReporter = async () => {
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
  return {
    event: async ({ event }) => {
      if (event?.type === "session.created") {
        const port = process.env.ORKWORKS_PORT;
        const orkworksSessionId = process.env.ORKWORKS_SESSION_ID;
        const capturedId = event.properties?.info?.id;
        if (typeof capturedId !== 'string' || !capturedId || capturedId === openCodeSessionId) return;
        openCodeSessionId = capturedId;
        turnStatus = 'idle';
        permissionIds.clear();
        questionIds.clear();
        lastReported = null;
        if (port && orkworksSessionId) {
          await fetch(`http://127.0.0.1:${port}/sessions/${orkworksSessionId}/harness-session`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              harnessSessionId: openCodeSessionId,
              source: "opencode_hook",
              confidence: 0.99,
            }),
          }).catch(() => {});
        }
        await postAttention(event.type);
        return;
      }
      // Attention events only apply to the captured session — a TUI can
      // host several OpenCode sessions, and stale ones must not steer
      // OrkWorks' attention state.
      if (!openCodeSessionId || event?.properties?.sessionID !== openCodeSessionId) return;
      const { id, requestID } = event.properties;
      switch (event.type) {
        case "session.idle":
          turnStatus = 'idle';
          break;
        case "session.status":
          if (event.properties.status?.type !== 'busy') return;
          turnStatus = 'working';
          break;
        case "permission.asked":
        case "question.asked":
          if (typeof id !== 'string' || !id) return;
          (event.type === 'permission.asked' ? permissionIds : questionIds).add(id);
          break;
        case "permission.replied":
        case "question.replied":
        case "question.rejected":
          if (typeof requestID !== 'string' || !requestID) return;
          if (!(event.type === 'permission.replied' ? permissionIds : questionIds).delete(requestID)) return;
          break;
        default:
          return;
      }
      await postAttention(event.type);
    },
  };
};
