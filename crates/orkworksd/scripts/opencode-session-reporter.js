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
// OpenCode 2.2.5 uses `form.created` with data.form.{id,sessionID}, and
// `form.replied` / `form.cancelled` with data.{id,sessionID}, instead of
// question.* (verified against the installed binary and official V2 API).
//
// Attention follows explicit permission and question request lifecycles.
// A turn boundary without a pending request maps to plain idle. The
// capture attention POST carries the normalized event name
// `session.capture` (not the triggering OpenCode event) so the sidecar's
// event/status matrix stays a small exact allow-list.
import { readFileSync, lstatSync } from 'node:fs';
import { isAbsolute } from 'node:path';
import { createHash } from 'node:crypto';
import { execFile } from 'node:child_process';

// Protocol 1 is tied to this owned reporter/helper set. Incompatible changes
// must bump the protocol; inherited selectors are stripped by the runtime.
const WORKTREE_HELPER_SHA256 = '5a8a588e60831a6ddd64bc1f60612aa67cb47659b9a83f088731b20c14eb1755';
function instructionSelector() {
  const env = process.env;
  const id = env.ORKWORKS_WORKTREE_INSTRUCTION_ROOT_ID;
  const helper = env.ORKWORKS_WORKTREE_INSTRUCTION_HELPER;
  if (env.ORKWORKS_WORKTREE_INSTRUCTION_PROTOCOL !== '1' ||
      typeof id !== 'string' || (!/^ses_[A-Za-z0-9_-]{1,252}$/.test(id) || id !== id.trim()) ||
      typeof helper !== 'string' || !isAbsolute(helper) || helper.length > 4096 ||
      /[\x00-\x1f\x7f]/.test(helper) ||
      !env.ORKWORKS_SESSION_ID || !env.ORKWORKS_PORT || !env.ORKWORKS_REPORT_TOKEN) return null;
  return {id, helper, env:{...env}};
}
function helperMatches(path) {
  try {
    const stat = lstatSync(path);
    return stat.isFile() && stat.size <= 1024 * 1024 &&
      createHash('sha256').update(readFileSync(path)).digest('hex') === WORKTREE_HELPER_SHA256;
  } catch { return false; }
}
function boundedLookup(promise, signal) {
  return new Promise(resolve => {
    const finish = value => {clearTimeout(timer); signal.removeEventListener('abort', aborted); resolve(value);};
    const aborted = () => finish(null);
    const timer = setTimeout(() => finish(null), 2000);
    signal.addEventListener('abort', aborted, {once:true});
    if (signal.aborted) aborted();
    promise.then(finish, () => finish(null));
  });
}
async function renderWorktreeContext(selector, signal) {
  if (signal.aborted || !helperMatches(selector.helper)) return null;
  return new Promise(resolve => {
    execFile('python3', [selector.helper, 'worktree-instructions'], {
      env:selector.env, timeout:2000, maxBuffer:64 * 1024, encoding:'utf8', signal,
    }, (error, stdout) => {
      if (error || signal.aborted) return resolve(null);
      try {
        const output = JSON.parse(stdout);
        const narrow = output?.hookSpecificOutput;
        const text = narrow?.additionalContext;
        if (Object.keys(output).length !== 1 || !narrow ||
            Object.keys(narrow).some(key => !['hookEventName','additionalContext'].includes(key)) ||
            narrow.hookEventName !== 'SessionStart' || typeof text !== 'string' ||
            !text.trim() || Buffer.byteLength(text, 'utf8') > 16 * 1024) return resolve(null);
        resolve(text);
      } catch { resolve(null); }
    });
  });
}

export default {
  id: 'orkworks.session-reporter',
  setup: async (ctx) => {
    const controller = new AbortController();
    const selector = instructionSelector();
    let lookupPending = false;
    if (selector && typeof ctx.session?.hook === 'function') {
      try {
      await ctx.session.hook('context', async event => {
        if (controller.signal.aborted || event.sessionID !== selector.id || lookupPending ||
            !Array.isArray(event.system) || typeof ctx.session.get !== 'function') return;
        lookupPending = true;
        let lookup;
        try {
          lookup = Promise.resolve(ctx.session.get({sessionID:selector.id}));
        } catch { lookupPending = false; return; }
        // A timed-out SDK call stays outstanding until it settles; do not
        // accumulate more reads behind an unresponsive native API.
        const tracked = lookup.finally(() => {lookupPending = false;});
        const info = await boundedLookup(tracked, controller.signal);
        if (controller.signal.aborted || !info || info.id !== selector.id || info.parentID != null) return;
        const text = await renderWorktreeContext(selector, controller.signal);
        if (!controller.signal.aborted && text) event.system.push({type:'text', text});
      });
      } catch { /* Optional context delivery must not disable legacy reporting. */ }
    }
    let openCodeSessionId = null;
    let identityDelivered = false;
    let turnStatus = 'idle';
    let lastReported = null;
    let lastMicros = 0;
    const permissionIds = new Set();
    const questionIds = new Set();
    const formIds = new Set();

    const effectiveAttention = () => {
      const needsAnswer = questionIds.size || formIds.size;
      if (permissionIds.size && needsAnswer) {
        return { status: 'waiting_for_input', message: 'OpenCode needs an answer or permission decision' };
      }
      if (permissionIds.size) {
        return { status: 'waiting_for_input', message: 'OpenCode is asking for a permission decision' };
      }
      if (needsAnswer) {
        return { status: 'waiting_for_input', message: 'OpenCode is asking for an answer' };
      }
      return { status: turnStatus };
    };

    const postAttention = async (event) => {
      if (!identityDelivered) return;
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
      identityDelivered = false;
      turnStatus = 'idle';
      permissionIds.clear();
      questionIds.clear();
      formIds.clear();
      lastReported = null;
    };

    const postHarnessSession = async () => {
      const port = process.env.ORKWORKS_PORT;
      const orkworksSessionId = process.env.ORKWORKS_SESSION_ID;
      if (!port || !orkworksSessionId) return;
      for (let attempt = 0; attempt < 3; attempt++) {
        try {
          const response = await fetch(`http://127.0.0.1:${port}/sessions/${orkworksSessionId}/harness-session`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              harnessSessionId: openCodeSessionId,
              source: "opencode_hook",
              confidence: 0.99,
            }),
            signal: AbortSignal.any([controller.signal, AbortSignal.timeout(2000)]),
            redirect: 'error',
          });
          // Consume the bounded identity response before another attempt.
          await response.body?.cancel();
          if (response.ok) {
            identityDelivered = true;
            return;
          }
          if (response.status !== 429 && response.status < 500) return;
        } catch {
          // Transport errors remain pending, including request deadlines.
        }
        if (controller.signal.aborted) return;
        if (attempt < 2) await new Promise(resolve => setTimeout(resolve, 100));
      }
    };

    const handleEvent = async (event) => {
      const type = event?.type;
      const data = event?.data;
      if (typeof type !== 'string' || typeof data !== 'object' || data === null) return;
      // OpenCode 2.2.5 replaced question.* with form.*. Creation nests
      // ownership inside data.form; resolution uses data.sessionID/id.
      const sessionID = type === 'form.created' ? data.form?.sessionID : data.sessionID;
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
      let capturedNow = false;
      if (!openCodeSessionId) {
        if (typeof sessionID !== 'string' || !sessionID || !type.startsWith('session.')) return;
        if (type === 'session.deleted' || type === 'session.error') return;
        resetCaptureState(sessionID);
        capturedNow = true;
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
      if (!identityDelivered && !capturedNow) await postHarnessSession();
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
        case 'form.created':
          if (typeof data.form?.id !== 'string' || !data.form.id) return;
          formIds.add(data.form.id);
          break;
        case 'form.replied':
        case 'form.cancelled':
          if (typeof id !== 'string' || !id || !formIds.delete(id)) return;
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
