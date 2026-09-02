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
// Attention mapping (issue #104): OpenCode's turn boundary (`session.idle`)
// maps to plain "idle" — unread state carries the needs-attention display —
// while a pending permission request (`permission.asked`) is the genuine
// "needs you" signal. `session.status` with type "busy" clears a stale
// waiting/idle state when the agent starts working again.
export const OrkWorksSessionReporter = async () => {
  let openCodeSessionId = null;
  const postAttention = async ({ status, message }) => {
    const port = process.env.ORKWORKS_PORT;
    const orkworksSessionId = process.env.ORKWORKS_SESSION_ID;
    if (!port || !orkworksSessionId) return;
    const payload = { status, observedAt: new Date().toISOString() };
    if (message) payload.message = message;
    await fetch(`http://127.0.0.1:${port}/sessions/${orkworksSessionId}/attention`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    }).catch(() => {});
  };
  return {
    event: async ({ event }) => {
      if (event.type === "session.created") {
        const port = process.env.ORKWORKS_PORT;
        const orkworksSessionId = process.env.ORKWORKS_SESSION_ID;
        // Only overwrite the captured ID when the event actually carries
        // one — a malformed session.created must not disable attention
        // reporting for the rest of the process.
        const capturedId = event.properties?.info?.id;
        if (capturedId) openCodeSessionId = capturedId;
        if (!port || !orkworksSessionId || !openCodeSessionId) return;
        await fetch(`http://127.0.0.1:${port}/sessions/${orkworksSessionId}/harness-session`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            harnessSessionId: openCodeSessionId,
            source: "opencode_hook",
            confidence: 0.99,
          }),
        }).catch(() => {});
        return;
      }
      // Attention events only apply to the captured session — a TUI can
      // host several OpenCode sessions, and stale ones must not steer
      // OrkWorks' attention state.
      if (!openCodeSessionId || event.properties?.sessionID !== openCodeSessionId) return;
      switch (event.type) {
        case "session.idle":
          await postAttention({ status: "idle" });
          return;
        case "permission.asked":
          await postAttention({
            status: "waiting_for_input",
            message: "OpenCode is asking for a permission decision",
          });
          return;
        case "permission.replied":
        case "session.status":
          if (event.type === "session.status" && event.properties?.status?.type !== "busy") return;
          await postAttention({ status: "working" });
          return;
      }
    },
  };
};
