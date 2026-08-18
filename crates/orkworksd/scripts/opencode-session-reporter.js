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
export const OrkWorksSessionReporter = async () => {
  return {
    event: async ({ event }) => {
      if (event.type !== "session.created") return;
      const port = process.env.ORKWORKS_PORT;
      const orkworksSessionId = process.env.ORKWORKS_SESSION_ID;
      const openCodeSessionId = event.properties?.info?.id;
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
    },
  };
};
