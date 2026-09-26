# OpenCode Prompt Attention Implementation Plan

> **Superseded draft (2026-09-26):** The design now requires session-scoped
> OpenCode hook activation and a Codex/OpenCode Peon attention rule. This plan
> describes the earlier narrow rule and must be rewritten after review of the
> revised spec. Do not execute its tasks as written.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show Needs You for unresolved OpenCode questions and permissions, preserve it through unrelated turn events and Peon observations, and avoid dropping rapid prompt reports.

**Architecture:** The project-local OpenCode reporter owns request-ID sets and projects them to the existing attention route. A narrow sidecar Peon merge rule protects only an active OpenCode `agent`-sourced waiting status; Codex retains its current authority rule. The route and shared input/hook timestamp guards remain intact.

**Tech Stack:** OpenCode plugin JavaScript, Node's built-in test runner, Rust sidecar, Axum attention route, repository Markdown.

**Spec:** `docs/superpowers/specs/2026-09-26-opencode-prompt-attention-design.md`

## Global Constraints

- Needs You means an unresolved `permission.asked` or `question.asked` request; ordinary `session.idle` means Idle.
- Events affect only the OpenCode session ID captured from `session.created`; malformed or foreign request IDs do not change pending state.
- Status and generic prompt message are both part of the reporter's emitted projection.
- Use `performance.timeOrigin + performance.now()` and strictly increasing integer microseconds formatted with six UTC fractional digits.
- Preserve OpenCode waiting through Peon merges only for live, active, attention-capable OpenCode sessions with current `agent` status; retain Codex and user priority behavior.
- OpenCode integration coverage remains limited until attention events are observed end to end in a live OpenCode process.
- The reporter cannot reconstruct an already pending prompt after plugin reload and cannot guarantee recovery if a resolution report is lost.
- Use `rtk` for shell commands and `pnpm` for any Node package-management operation. Work in the existing `opencode-prompt-attention` sibling worktree.

## Evidence and blind-spot checkpoint

**Least confidence:** exact ordering of live OpenCode event callbacks relative to terminal input has not been observed inside an OrkWorks-launched process. The 1.18.18 and 1.18.32 schemas/publisher establish event names and payloads; behavioral tests will establish our projection, while integration coverage stays limited. The current reporter quantizes `observedAt` to milliseconds; the sidecar independently rejects hook time at or before `accepted_input_at`.

**Project blind spot:** `active_work_hook` is a resolved capability, not proof that the installed reporter sent a particular POST. The narrow OpenCode rule deliberately shares the current local attention-route trust boundary. A later authenticated reporter protocol would require a separate design. The existing preserving merge writes `codex_hook` provenance internally; the implementation must parameterize it rather than calling it unchanged for OpenCode.

## File map

| File | Responsibility |
| --- | --- |
| `docs/adr/0065-opencode-prompt-attention-authority.md`, `docs/adr/README.md` | Record and index the narrow Peon authority exception before code. |
| `crates/orkworksd/scripts/opencode-session-reporter.js` | Track the captured session's turn and prompt state; post effective attention with ordered high-resolution time. |
| `crates/orkworksd/scripts/opencode-session-reporter.test.mjs` | Run the actual reporter against controlled event sequences and an HTTP receiver. |
| `crates/orkworksd/src/harness/integrations/opencode.rs` | Replace obsolete source-text assertions with the relevant packaging/source identity check. |
| `crates/orkworksd/src/session_application.rs` | Select OpenCode pending-wait authority for Peon merges without changing Codex's selection. |
| `crates/orkworksd/src/metadata.rs` | Preserve the selected hook source instead of always writing `codex_hook`. |
| `docs/agents/harness-integration-contracts.md` | Document question-event coverage, source evidence, and limits. |

### Task 1: Record the prompt-authority decision

**Files:**
- Create: `docs/adr/0065-opencode-prompt-attention-authority.md`
- Modify: `docs/adr/README.md`

**Interfaces:** This task defines the invariant for Task 3: Peon preserves only an active OpenCode `agent`-sourced `waiting_for_input`, while ordinary OpenCode Working/Idle and every other harness retain their existing rules.

- [ ] **Step 1: Write the ADR before code.** Use the repository ADR template and state the context (15-second Peon overwrite), decision (OpenCode waiting-only exception), and consequences (a lost resolution may leave Needs You stale; no cross-session authority). Keep ADR 0027's single owner for status writes and ADR 0005's source ladder intact.
- [ ] **Step 2: Add ADR 0065 to the index.** Give it `accepted` status, since this is the approved written design. Do not add a root `AGENTS.md` inline ADR bullet because the design and harness contract contain independent prose.
- [ ] **Step 3: Review the doc diff.** Run `rtk git diff --check` and inspect `rtk git diff -- docs/adr/0065-opencode-prompt-attention-authority.md docs/adr/README.md`; confirm the ADR neither grants authority to OpenCode Working/Idle nor changes Codex.
- [ ] **Step 4: Commit.** Run `rtk git add docs/adr/0065-opencode-prompt-attention-authority.md docs/adr/README.md` then `rtk git commit -m 'docs: record OpenCode prompt attention authority'`.

### Task 2: Make the reporter project prompt request state

**Files:**
- Create: `crates/orkworksd/scripts/opencode-session-reporter.test.mjs`
- Modify: `crates/orkworksd/scripts/opencode-session-reporter.js`
- Modify: `crates/orkworksd/src/harness/integrations/opencode.rs`

**Interfaces:** Consumes OpenCode plugin events `{ type, properties }`. Produces existing `POST /sessions/:id/attention` JSON `{ status, observedAt, message? }`; no route-schema change.

- [ ] **Step 1: Write the first failing reporter test.** Import the actual reporter source as an ESM data URL from the `.mjs` test file. Start a local `node:http` server and record its real POST bodies. Set `ORKWORKS_PORT` and `ORKWORKS_SESSION_ID`, send `session.created` for `ses_1`, `session.status: busy`, then `question.asked` with `{ id: 'que_1', sessionID: 'ses_1', questions: [] }`. Assert attention statuses are `['working', 'waiting_for_input']`, with an answer-request message. The old reporter already reports Working but fails to report the question.

  ```js
  import assert from 'node:assert/strict';
  import { once } from 'node:events';
  import { readFile } from 'node:fs/promises';
  import { createServer } from 'node:http';
  import test from 'node:test';

  const source = await readFile(new URL('./opencode-session-reporter.js', import.meta.url), 'utf8');
  const { OrkWorksSessionReporter } = await import(`data:text/javascript;base64,${Buffer.from(source).toString('base64')}`);

  test('question ask makes the captured session need an answer', async (t) => {
    const posts = [];
    const server = createServer(async (req, res) => {
      const body = JSON.parse(Buffer.concat(await Array.fromAsync(req)).toString());
      if (req.url.endsWith('/attention')) posts.push(body);
      res.writeHead(200).end();
    });
    server.listen(0, '127.0.0.1');
    await once(server, 'listening');
    t.after(() => server.close());
    const previousPort = process.env.ORKWORKS_PORT;
    const previousSession = process.env.ORKWORKS_SESSION_ID;
    process.env.ORKWORKS_PORT = String(server.address().port);
    process.env.ORKWORKS_SESSION_ID = 'ork_1';
    t.after(() => {
      if (previousPort === undefined) delete process.env.ORKWORKS_PORT;
      else process.env.ORKWORKS_PORT = previousPort;
      if (previousSession === undefined) delete process.env.ORKWORKS_SESSION_ID;
      else process.env.ORKWORKS_SESSION_ID = previousSession;
    });
    const reporter = await OrkWorksSessionReporter();
    await reporter.event({ event: { type: 'session.created', properties: { info: { id: 'ses_1' } } } });
    posts.length = 0;
    await reporter.event({ event: { type: 'session.status', properties: { sessionID: 'ses_1', status: { type: 'busy' } } } });
    await reporter.event({ event: { type: 'question.asked', properties: { id: 'que_1', sessionID: 'ses_1', questions: [] } } });
    assert.deepEqual(posts.map((post) => post.status), ['working', 'waiting_for_input']);
    assert.match(posts[1].message, /answer/);
  });
  ```
- [ ] **Step 2: Run the red test.** Run `rtk proxy node --test crates/orkworksd/scripts/opencode-session-reporter.test.mjs`; confirm the failure is the missing question transition, not an import or server setup error.
- [ ] **Step 3: Implement the smallest prompt state projection.** Maintain separate `Set`s for permission and question IDs, `turnStatus = 'idle'`, and the last emitted `{status,message}`. Map asks by `properties.id` and replies/rejections by matching `properties.requestID`; reject empty or foreign IDs. Derive `waiting_for_input` whenever either set is nonempty, use distinct generic permission/question/mixed messages, and post when status or message changes. `session.status` busy and `session.idle` update `turnStatus` without clearing pending sets. New `session.created` resets state and posts idle; duplicate same-ID creation preserves state.

  ```js
  const permissions = new Set();
  const questions = new Set();
  let turnStatus = 'idle';
  let lastReport = null;
  const effective = () => {
    if (permissions.size && questions.size) return { status: 'waiting_for_input', message: 'OpenCode needs an answer or permission decision' };
    if (permissions.size) return { status: 'waiting_for_input', message: 'OpenCode is asking for a permission decision' };
    if (questions.size) return { status: 'waiting_for_input', message: 'OpenCode is asking for an answer' };
    return { status: turnStatus, message: undefined };
  };
  const reportIfChanged = async () => {
    const next = effective();
    if (next.status === lastReport?.status && next.message === lastReport?.message) return;
    lastReport = next;
    await postAttention(next);
  };
  ```
- [ ] **Step 4: Run green and expand one behavior at a time.** Add a failing event-sequence case before each corresponding change for new-session idle reset, question reply/reject, permission reply, overlapping types, same-type multiple IDs, duplicate/unknown resolution, foreign/malformed events, and busy/idle while pending. Assert the HTTP payloads, including message changes, rather than checking source text.
- [ ] **Step 5: Pin microsecond ordering.** With a controlled `globalThis.performance`, make one event occur at a hand-checked time within the same millisecond as an input-boundary fixture; assert the payload retains its nonzero microsecond part and successive updates strictly increase. Add cases for several events in one millisecond, a backward wall-clock step, and a failed POST followed by a later report. Then replace the millisecond-padding formatter with `Math.max(Math.floor((performance.timeOrigin + performance.now()) * 1000), lastMicros + 1)` and format exactly six fractional UTC digits.

  ```js
  let lastMicros = 0;
  const nextObservedAt = () => {
    const micros = Math.max(Math.floor((performance.timeOrigin + performance.now()) * 1000), lastMicros + 1);
    lastMicros = micros;
    const fraction = String(micros % 1_000_000).padStart(6, '0');
    return new Date(Math.floor(micros / 1000)).toISOString().replace(/\.\d{3}Z$/, `.${fraction}Z`);
  };
  ```

  For the reordering case, invoke `reporter.event({ event: first })` and
  `reporter.event({ event: second })` without awaiting the first, delay the
  first HTTP response in the test server, then assert the two payload
  timestamps are strictly ordered in event-call order. The existing sidecar
  stale-hook guard decides the winner if network delivery reverses them.
- [ ] **Step 6: Update the Rust integration source checks.** Remove the assertion requiring the old `.$1000Z` padding. Keep the installed-source identity assertion, and prefer the behavioral Node test over new string-containment checks for event semantics. Run `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml plugin_source_` and the Node test.
- [ ] **Step 7: Commit.** Run `rtk git diff --check`, stage only the three Task 2 files, and commit with `rtk git commit -m 'fix: track OpenCode prompt attention lifecycle'`.

### Task 3: Preserve live OpenCode prompt status during Peon merges

**Files:**
- Modify: `crates/orkworksd/src/session_application.rs`
- Modify: `crates/orkworksd/src/metadata.rs`
- Modify: `docs/agents/harness-integration-contracts.md`

**Interfaces:** `merge_peon_inference_with_history_preserving_hook_status` gains a preserved source argument (`&str`) alongside status and confidence. The OpenCode caller passes `agent`; the Codex caller passes `codex_hook` and retains its existing after-input `process` provenance behavior.

- [ ] **Step 1: Write a failing sidecar test.** Beside `active_codex_hook_keeps_peon_waiting_inference_from_becoming_needs_you` in `session_application.rs`, create a live active OpenCode handle with `active_work_hook = true`, `observed_status = waiting_for_input`, `attention = needs_you`, `metadata_source = agent`, and a 60-second-old metadata file. Feed a Peon `working` inference with a new summary. Assert durable and live status remain `waiting_for_input`/`needs_you`, source remains `agent`, and the summary updates. The old code fails because Peon may overwrite the stale agent status.

  ```rust
  let result = SessionApplication::new(state.clone()).persist_peon_observation(
      id,
      Some(&inference),
      None,
      Some("Still waiting for an OpenCode answer"),
      "later",
  );
  assert!(result.inference_persisted);
  let stored = state.workspace.lock().unwrap().as_ref().unwrap().metadata.read_session(id).unwrap();
  assert_eq!(stored.observed_status.as_deref(), Some("waiting_for_input"));
  assert_eq!(stored.attention.as_deref(), Some("needs_you"));
  assert_eq!(stored.metadata_source, "agent");
  assert_eq!(stored.summary.as_deref(), Some("Still waiting for an OpenCode answer"));
  ```
- [ ] **Step 2: Run the red test.** Run `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml active_opencode_prompt_`; confirm the status/source assertion fails for the expected reason.
- [ ] **Step 3: Generalize the preserving merge seam.** Change the existing preserved tuple from `(status, confidence)` to `(status, confidence, source)` in `metadata.rs`; use the supplied source where it currently hardcodes `codex_hook`, while preserving the existing `process` exception for Codex. In `session_application.rs`, retain the Codex predicate and add a separate OpenCode predicate requiring live + active + `active_work_hook` + OpenCode harness ID + `agent` source + `waiting_for_input`. Pass `agent` only for that case. Keep all status writes under the existing metadata/runtime merge owner and lock order.

  ```rust
  // metadata.rs: the preserved tuple carries its accepted provenance.
  hook_status: Option<(&str, f64, &str)>,
  if let Some((status, confidence, source)) = hook_status {
      meta.observed_status = Some(status.to_string());
      if meta.lifecycle == "alive" {
          meta.attention = canonical_attention(Some(status));
      }
      if meta.metadata_source != "process" {
          meta.metadata_source = source.to_string();
          meta.metadata_confidence = confidence;
      }
  }
  ```

  The selection returns `(status, confidence, preserved_source)`; use `"codex_hook"` for the existing Codex branch and `"agent"` only for the OpenCode pending-wait branch.
- [ ] **Step 4: Prove the negative boundaries.** Add failing-then-green cases showing OpenCode `working` and `idle` remain Peon-overwritable after 15 seconds; an inactive, terminal, or non-attention-capable OpenCode session gets no exception; a user source remains higher priority; and the existing Codex authority tests still retain `codex_hook`/`process` provenance. Run the focused Rust tests after each case.
- [ ] **Step 5: Update the harness contract.** Name the verified 1.18.18/1.18.32 question events and request-ID fields, describe pending-prompt precedence and the narrow Peon exception, and retain `Limited` coverage until live OpenCode attention delivery is observed. State that a missed resolution or plugin reload cannot be reconstructed reliably.
- [ ] **Step 6: Commit.** Format the touched Rust files with `rtk proxy rustfmt --edition 2021 crates/orkworksd/src/session_application.rs crates/orkworksd/src/metadata.rs`, then run `rtk cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check` and `rtk git diff --check`; stage only Task 3 files and commit with `rtk git commit -m 'fix: preserve pending OpenCode prompt authority'`.

### Task 4: Verify and prepare the PR

**Files:** No new source files. Update issue #631 and the PR description with evidence and limitations.

**Interfaces:** The combined implementation must preserve the existing `/attention` payload and all other harness rules.

- [ ] **Step 1: Run focused behavioral verification.** Run `rtk proxy node --test crates/orkworksd/scripts/opencode-session-reporter.test.mjs`, `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml active_opencode_prompt_`, and the existing Codex authority tests; record counts and failures.
- [ ] **Step 2: Run required sidecar validation.** Run `rtk cargo build --manifest-path crates/orkworksd/Cargo.toml`, `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml`, and `rtk cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check`. Run `rtk git diff --check` and inspect `rtk git status --short`.
- [ ] **Step 3: Review the complete diff.** Use the `requesting-code-review` skill and the repo's explicit `/code-review medium` gate: this change crosses reporter and sidecar authority boundaries. Address findings or record evidence for intentional choices.
- [ ] **Step 4: Open one PR for issue #631.** Push the owned branch, open the PR against `main`, include the spec/ADR, verification commands, limited live-coverage and restart/lost-event limitations, and link #631. Follow `babysitting-pull-requests` for CI and review feedback. Keep #632 separate.
