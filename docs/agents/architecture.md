---
type: Architecture Reference
title: OrkWorks architecture
description: Electron, Rust sidecar, runtime, API, and metadata-protocol architecture.
tags: [architecture, electron, rust, sidecar, metadata, api]
status: stable
---

# Architecture

When changing runtime behavior, also read the relevant [runtime contracts](runtime-contracts.md).
Load the sections that constrain the task; this reference is not a whole-file reading requirement.

```text
orkworks/
├─ apps/desktop/          # Electron + React/TypeScript + CSS Grid + xterm.js
├─ crates/orkworksd/      # Rust sidecar (Axum HTTP/WS, PTY via portable-pty)
├─ docs/
│  ├─ adr/                # Architecture Decision Records
│  └─ agents/             # Agent-facing docs (this file, domain-entities, apm)
├─ skills/                # Repo-level agent skills
└─ specs/                 # Authoritative product specs
```

## Electron ↔ Rust sidecar communication

`electron/main.ts` spawns `orkworksd` as a child process and discovers its port by reading stdout for the line `ORKWORKSD_PORT=<n>`. The app icon is platform-aware: macOS uses `icon.png`/`icon-dark.png` (squircle background baked in) via `app.dock.setIcon()`; Windows uses `icon.ico`/`icon-dark.ico` (transparent background, multi-resolution) via `BrowserWindow.setIcon()`. Both swap on `nativeTheme` change. The port is dynamic — there is no fixed localhost port. The frontend gets the URL via the preload bridge: `window.orkworks.getBackendUrl()`.

### Sidecar lifecycle and runtime recovery

`electron/sidecarLifecycle.ts` owns sidecar process startup, port discovery, and
failure recovery. Each launch is a monotonically numbered generation with its
own readiness promise. Only the current generation may publish a port, change
the active lifecycle state, or notify the renderer; exits, errors, timers, and
stdout from a replaced generation are ignored. Spawn errors, exits before the
port line, and the 10-second readiness timeout reject that generation's
readiness and clear the active port. Once a process has announced a port, a
later process failure invalidates the port and reports the failure to the main
process instead of leaving API callers waiting on a stale promise.

Sidecar readiness is followed by a separate generation-owned restoration gate
in `electron/backendRestoration.ts`. After the port is known, Electron main
restores the selected workspace as the critical prerequisite for readiness.
Once that succeeds, `get-backend-url` resolves and the renderer receives the
`ready` lifecycle event; the renderer can therefore adopt the restored
workspace and recover its sessions even if a secondary settings replay is
unavailable. Persisted retention and provider settings are then applied as
abortable best-effort steps in the same generation. Provider replay shares the
Electron settings mutation queue with user saves, so a slow startup catalog
request cannot overwrite newer settings. Their failures are logged without
invalidating workspace readiness, and replacement/disposal aborts any
in-flight step. A restoration timeout or workspace-restoration failure still
rejects readiness and publishes an unavailable state. Initial startup remains in
the picker with no sidecar, health probing, or backend recovery UI, even when
history contains remembered workspace paths. Those paths are choices for an
explicit open, not authority to restore a runtime automatically. A workspace
path is remembered only after restoration publishes readiness. Stale
restoration work is aborted so an older workspace cannot become ready afterward.

Installation history uses `fs-ext` for a bounded, nonblocking OS advisory lock
on a retained `.workspace-memory.lock` file in Electron `userData`. Never unlink
or rename this lock file: contenders must lock the same inode. Process exit
releases ownership; time alone cannot evict a live writer. Under the lock,
mutations reread history, reject revision overflow, flush a bounded temporary
record, atomically replace the target, and verify it by read-back. Corrupt input
is preserved and history-write failures do not roll back a ready workspace.
`pnpm build:native` runs the native dependency installer and verifies this addon
in the installed Electron runtime before dev/build/dist. A failed load forces a
targeted rebuild against the installed Electron version and a second load check;
a stale rebuild marker alone cannot establish ABI compatibility. After those
commands, `pnpm rebuild fs-ext` restores the host Node ABI for tests. The separate
`pnpm test:native` regression exercises host-built addon reuse with a stale
Electron marker, actual advisory locking, and repeated startup; it restores the
host ABI before the ordinary Node tests and must not run concurrently with them.

Recovery is explicit through the serialized workspace coordinator. A startup
failure reports its readiness/restoration diagnostic; Retry uses the same
cleanup-and-open path. An unexpected sidecar exit cancels restoration and
generation-bound requests, advances the backend fence, and marks the workspace
unresolved. Cleanup timeout/failure remains sticky and blocks replacement;
process exit alone cannot prove descendant cleanup. Crash-recovery adoption and
replacement remain subject to ADR 0060's native ownership evidence gate;
closing #545 did not establish that proof. There is no automatic sidecar
relaunch in the current source.

Process launch, restoration candidate, and workspace-operation generations have
different lifetimes and need not share numeric IDs. Main advances its request
fence and aborts old work when admission closes, a launch starts, or an
unexpected exit occurs. Restoration callbacks additionally require the current
unaborted candidate; the workspace coordinator rechecks its operation generation
after readiness and awaited history persistence before publishing ready.
Provider replay checks cancellation when its queued settings write begins and
after catalog decoding before persisting. Rejected-workspace history cleanup
checks cancellation after its await before changing the current path or history
diagnostic. These checks also cover operations that finish despite abort.

The preload bridge exposes `onBackendLifecycle` and `retryBackend`. Lifecycle
events are the narrow union `picker` (no backend), `starting`, `retrying`, `ready` (with a validated
port), `failed` (with a stable failure message), and `exhausted` (with a stable
failure message). Preload canonicalizes exact event shapes and replays the
latest main-process snapshot for late subscribers, while preserving live-event
ordering and returning an unsubscribe function. The renderer maps these events
to its backend status, stops session polling unless the backend is connected
and workspace restoration is complete, and shows a Retry panel for
unreachable or exhausted states without discarding existing workspace/session
state.


`workspaceSessionController` owns session-list publication. Each refresh captures
request, foreground, admission, source and polling identities and uses one local predicate
for both successful lists and unavailable status; disposal invalidates publication.
A new stream snapshot invalidates pending REST publication even when the stream
was already healthy. Those identities remain independent. App's polling gate additionally covers the
renderer switch attempt before a backend lifecycle event reaches the controller,
and requires a connected backend and restored workspace. Controller admission
checks therefore do not replace that gate. Creation rechecks its foreground and
admission identities after backend discovery and before sending the request, so
a pending lookup cannot dispatch into a replacement workspace or disposed
controller. The same identities still guard response publication; requests
already sent are not treated as canceled. Review-document, model-refresh and
Settings-save guards retain their own selection/request/lifecycle semantics.

React render exceptions are handled in the renderer by `ErrorBoundary`.
Electron main has an independent fallback for main-frame `did-fail-load` and
`render-process-gone` events, with bounded, sanitized console diagnostics. It
loads a resource-free inline recovery document for those failures; the
document has one user-triggered Retry action that returns to the captured
development URL or exact packaged `file:` document. A recovery-load guard
prevents repeated automatic fallback navigation and resets when the original
document begins or finishes loading, or when recovery navigation fails. This
path does not depend on React or the preload bridge, so it remains available
when the normal renderer document is not.

## Packaging and release

Packaged desktop updates use the production `electron-updater` dependency in
Electron main. Main explicitly flags packaged preload through a window argument;
development update methods remain IPC-inert. The updater's own network session
blocks cross-channel metadata fallback. Release identity excludes the public
download event's local `downloadedFile` completion path. Both macOS and Windows
installation fail closed before verification, session queries, confirmation,
shutdown, installer invocation, or recovery. NSIS schedules quit before asynchronous
spawn failure is known and offers no supported latch reset/retry handshake; macOS
cannot prove native verification without arming installation. Windows signature
verification accepts the release pipeline's exact certificate SimpleName, while
rejecting mismatch or skipped verification. The adapter retains fresh metadata
revalidation; neither matching metadata nor signature proof enables installation.
A sidecar stopped for installation remains owned until its exit is confirmed,
including after timeout. See [desktop updates](../user/updates.md) for limitations.

Taskmaster's independently updated reference knowledge and separate analysis
model follow [ADR 0054](../adr/0054-taskmaster-honors-managed-cli-policy.md) and the
[knowledge specification](../../specs/taskmaster-knowledge.md). Electron main
owns feed/cache/starter verification and sends retained signed activation proof;
the sidecar independently verifies that proof and owns bounded analysis and
evidence validation. Knowledge cannot change execution authority.
Brain-backed analysis requires server-owned, reverified durable knowledge proof
under [ADR 0081](../adr/0081-signed-taskmaster-knowledge-activation-proof.md).
Current source packages the generated `brain-3` signed starter with the complete readiness
rubric and unchanged public pin from the approved production publication. Admission checks
manual requests, scheduling and worker entry before provider preparation, context
collection or usage reservation. Missing, corrupt, unsupported or mismatched proof
remains unavailable; unsigned policy fields, legacy snapshots and executable
approval cannot enable it. Manual admission and scheduling still retry
lease-protected stale run-status recovery, and queued workers whose proof becomes
unavailable are interrupted without dispatch. Production regressions cover the
actual starter, authenticated activation, all four native/custom manual/background
dispatch paths, offline restart and corruption revocation. Deterministic observation
recommendations and explicit Fix with AI remain available independently.
See [knowledge publication operations](taskmaster-knowledge-publication.md) for
public artifact identities, promotion and trust rotation.
CLI analysis reuses installed logins through fixed recommendation profiles and
honors administrator-managed policies; required hooks/context/routing are not
bypassed. OrkWorks' context limits constrain its own collection and request.

`GET/POST /settings/taskmaster` and `POST /settings/taskmaster/knowledge` use
the existing Electron-only sidecar authority. Taskmaster settings and durable
evaluation accounting are global under `~/.orkworks/taskmaster/`, with explicit
workspace overrides; Peon settings remain independent. The Recommendations
settings panel uses narrow preload methods rather than receiving that authority.
Electron sends exactly `activationFormatVersion`, `bundleEnvelopeBase64`, and
`manifestEnvelopeBase64`, retaining the original signed envelope bytes. The
knowledge route reads bounded raw bytes before strict JSON/signature validation;
its body limit and Electron's knowledge request limit are 6 MiB inclusive.
Other resource and response limits are unchanged. Each decoded envelope remains
bounded at 2 MiB. A standalone signed bundle or decoded bundle is ineligible.

The sidecar derives its namespace from the embedded trusted key's SPKI SHA-256
and atomically persists the proof at
`~/.orkworks/taskmaster/knowledge/<fingerprint>/activation.json` under the existing
persistence locks. Legacy `knowledge.json` and other keys' namespaces remain
untouched and cannot set its sequence floor. Startup and durable reload bound,
reread and independently reverify the proof; an unreadable, missing or invalid
record clears its in-memory authority even without a generation change. Lower
sequences and equal-sequence different identities are refused. A matching bundle
attested by another valid manifest is idempotent and does not advance generation.
Changed knowledge durably advances generation before publishing the proof, so a
crash between writes invalidates old work. Unique synced temporary files use the
existing platform-safe atomic replacement primitive.

The packaged resources are `knowledge/starter.json` (an activation record retaining
both original signed envelopes) and `knowledge/public-key.pem`. The Rust verifier
embeds that same public key. The generated starter is independently eligible
without contacting the feed; a key rotation ships its signed starter and trust pin
together. This describes current source, not older published installers.

The custom inference foundation follows [ADR 0055](../adr/0055-json-taskmaster-inference-adapters.md).
`harness/inference.rs` validates inert JSON capabilities;
`taskmaster/inference_trust.rs` and its `identity` submodule resolve executable
identity without running it and persist fingerprint-bound grants separately in
`inference-trust.json` beneath the Taskmaster global root. Grant operations
share Taskmaster's process and filesystem persistence locks, reject stale
revisions, and fail closed on malformed data. `taskmaster/inference_approval.rs`
holds the harness mutation lock while re-resolving identities and changing
grants. Dedicated `GET/POST /settings/taskmaster/inference` handlers require
Electron-only authority and strict revision-bound requests. Electron's
`inferenceTrust.ts` re-inspects before native confirmation; its narrow preload
methods expose inspect, approve, and revoke without exposing credentials or
arbitrary URLs. Recommendations settings lists these capabilities independently
of Peon. `providers/custom_inference.rs` implements the isolated command transport:
single-pass literal argv expansion, private stdin/file input ownership, inherited
CLI login configuration with OrkWorks capabilities stripped, and strict bounded
UTF-8/JSON response validation. It reuses the existing process runner with opt-in
strict stdout decoding; existing Peon and native transports retain their decoding.
The runtime's captured-identity invocation now calls this transport, but the
scheduler does not activate that path yet. `taskmaster/provider_catalog.rs`
projects selection availability independently of Peon; privileged settings and
the pre-context evaluator gate share that projection. Custom definitions report
approval required, unavailable, or ready after current executable approval, including
when they override a built-in ID. An explicit inference clear cannot fall back
to the legacy native profile. The Recommendations picker preserves unavailable
stored selections, shows provider-specific model suggestions, and allows
manual IDs. Codex and Ollama can refresh live models only on an explicit user
request; Claude Code uses its built-in static catalog. Taskmaster refresh uses
the Taskmaster draft provider and Ollama URL, a separate Electron-authorized
route, and the shared provider-discovery implementation where safe. It never
calls Peon's HTTP discovery endpoint, performs inference, applies settings, or
changes Peon state. Custom Taskmaster providers keep static suggestions and
manual entry; Taskmaster does not execute custom model-list commands. Peon's
existing model refresh remains unchanged. Taskmaster discovery selects a typed,
code-owned operation from the validated built-in provider: Codex always uses the
fixed app-server model-list operation and Ollama queries the supplied draft URL.
Neither path resolves a mutable provider definition to choose a model-list
command, including when a built-in ID has a user-defined `models` command
override. The model-refresh and run-status protocol decision is recorded in
[ADR 0071](../adr/0071-taskmaster-model-refresh-and-run-status.md).
Native readiness permits presentation/discovery metadata overrides but rejects
launch or Peon execution overrides unless an explicit custom inference command is
declared and trusted. Ollama does not advertise reasoning-effort support; a stored
unsupported effort is not ready for evaluation. Settings reject unsupported effort
instead of silently discarding it. Model and effort string bounds are shared with
the custom transport: nonempty, at most 256 UTF-8 bytes, no control characters,
and no model trimming or prefix rewriting.
Ordinary Taskmaster analysis uses the latest 16 observations whose evidence
contains Unicode letters or numbers. Filtering precedes the bound, and prompt
construction and knowledge retrieval share that window. The prompt includes
kind and observation time without treating historical friction as unresolved.
The stored observation history remains unchanged. Assessment applies the same
filter with a newest-128 window. Each mode uses its selected window for prompt
construction, Brain retrieval and repository relevance signals. The bounded
repository collector ranks current root/scoped guidance and manifests before
historical plans/reports, then uses signal overlap and deterministic path order.
It selects a contiguous relevant excerpt within the existing bounded file read;
access exclusions and the evidence hash of the bounded bytes remain unchanged.

Taskmaster status exposes the latest analysis outcome per workspace separately
from provider/configuration availability. Recommendations shows queued or
running state and the latest result; Settings includes provider/model,
timestamps, and useful failure details. Outcomes persist across restarts, with
an abandoned running attempt reported as interrupted. A later success clears or
supersedes the workspace's prior analysis error; knowledge-update errors remain
separate. Each workspace run record contains an optional `activeAttempt` and
the `latestOutcome`; the status projection shows the active attempt first, then
the latest outcome, then idle. Background checks that do not dispatch inference
do not replace the last outcome. Evaluator exits either finalize dispatched
work or clear the active attempt for pre-evaluation skips; context collection
and prompt-construction failures are failed outcomes even if the provider was
not invoked. Recovery clears queued attempts and marks running attempts
interrupted only after acquiring the same installation-wide analysis lease used
by evaluators; if another instance holds the lease, recovery waits for the next
workspace open or evaluation admission. Status queries are read-only. Each
workspace retains its active attempt, latest terminal outcome, and optional
latest manual-request disposition; total record count grows with the number of workspaces used, with
global retention and cleanup outside this increment. The workspace run record
is the sole displayed source for analysis outcomes; the legacy installation-wide
`lastError` remains readable for old ledger files but is no longer written or
displayed as an analysis error. Both UI surfaces call the narrow
`getTaskmasterRunStatus()` preload method, backed by Electron-authenticated
`GET /taskmaster/run-status`; Settings polls every five seconds and on window
focus, and Recommendations polls alongside its recommendation list. The status
does not enumerate or coordinate other OrkWorks instances.
New ordinary success outcomes include fixed-category created/suppressed proposal
counts. A separate `latestManualRequest` records confirmed cache hits or
pre-dispatch skips without overwriting the last actual outcome; background work
does not replace that disposition. Legacy count-less outcomes remain generic.

Manual workflow assessment uses separate `taskmaster/assessment.rs` scheduling,
`assessment_model.rs` strict decoding, `assessment_report.rs` publication, and
`assessment_store.rs` capped atomic workspace storage. The narrow
`requestTaskmasterAssessment()` preload call reaches authenticated empty-body
`POST /taskmaster/assess-workflow`. It shares the installation analysis lease and
selected provider/context, requires signed assessment capability, and bypasses
background quota/cooldown. Its separate run-status projection exposes only a
currently revalidated report. Manual assessment first supplies repository-led
workflow seeds and a permitted catalog, then decodes one strict discovery selection
before at most one final call through the same captured provider/model/effort.
`assessment_context.rs` owns bounded prefixes, component-by-component safe opens,
aggregate evidence limits and manifest reconstruction. Manifests and CI lead seed
selection; scoped instructions accompany selected targets rather than independently
consuming the seed budget. Discovery considers grounded problems or opportunities,
then the final assessment challenges the hypothesis before a proposal. Every selected target
includes its governing ancestor AGENTS.md/CLAUDE.md chain under those same limits;
inaccessible or incompletely enumerated instructions prevent a proposal. Workspace
and native relative paths must be valid UTF-8 and reconstruct without replacement
characters or normalization. `assessment_discovery.rs`
rejects arbitrary paths and unknown references. Progress exposes only fixed
`discovering`, `reading_evidence` and `assessing` phases.
One explicitly decoded format-1/2 `assessment.json` beneath workspace metadata
holds at most 64 KiB, including any pending recommendation upsert. Format 2 stores
a bounded input manifest with catalog digest/completeness, seed/follow-up prefix
and excerpt identities, exact selection and discovery digest, without uncited
contents, catalog listings, discovery prose or full prompts. Every consumer
re-enumerates current permission data and reconstructs that selection; stale
uncited inputs and catalog changes fence reuse and recommendations. Legacy reports
are not reusable, legacy proposed recommendations are superseded on state-changing
admission, and accepted/executing legacy records block new assessment until resolved
through existing authorized lifecycle routes. Ordinary analysis is unchanged.
Startup and immediate commit-error recovery replay the stable upsert before
publishing success; pending consumers fail closed.

`assessment_access.rs` shares current-input checks across list/detail, manual
admission, acceptance, and terminal handoff. Access-setting writes reconcile all
known affected workspace stores before persisting the narrower policy. Request
and startup reconciliation use the same settings-persistence/workspace lock order
and validate the selected workspace identity before reading or replacing metadata.
Redacted
completion packets keep audit IDs, fingerprints, approvals, and lifecycle history
while losing source-derived display text; their provenance rejects new actions
and packet mutations. The authenticated target session can still record completion
of an already-delivered accepted fix, with a generic completion event that cannot
restore redacted prose; an existing packet's matching completion retry remains
idempotent. Other input changes retain audit snapshots. A terminal
handoff reserves queue capacity before revalidation through synchronous enqueue,
then awaits delivery acknowledgement without configuration/workspace locks.
Already-delivered prompts finalize their audit outcome after a policy change;
undelivered redacted reservations roll back and become superseded. Assessment
handoff events use fixed text rather than retaining a generated title.
See [ADR 0085](../adr/0085-taskmaster-assessment-and-analysis-outcomes.md).

`taskmaster/runtime/inference.rs` now captures an immutable identity containing
the approved definition/path, trust generation, runtime generation, selected
settings, workspace key and full verified knowledge identity. Its short
synchronous action guard re-resolves and
revalidates those values while holding the harness document lease and Taskmaster
persistence/runtime locks. `PersistenceGuard` lets trust inspection reuse the
held persistence lease without recursively acquiring its mutex. The guard offers
a stable cache identity. `invoke_custom_inference` prepares only from captured
fields and supplies the process runner's fallible spawn callback; the guard
remains held through actual `Command::spawn`, then releases before prompt I/O,
output capture and process waiting. Native/Peon calls retain their direct-spawn
wrapper. Private prompt files remain owned through denial, failure or completion.
The invocation itself does not reserve usage or commit recommendations.
The evaluator now binds the custom identity to its snapshot before collecting
context; its internal catalog transport kind prevents failed custom capture from
falling through to native dispatch. Versioned SHA-256 cache keys include settings,
generation, prompt and the captured trust identity or native profile/revision.
Native catalog entries carry a closed code-owned Codex, Claude or Ollama profile
and the inspected harness-document revision. Production Taskmaster invocation
uses that bound profile's fixed definition, never a mutable Peon provider lookup;
existing CLI policy, login handling, version probes and response decoders remain
unchanged. Native profiles are not deserializable from user JSON. Binding,
reservation, diagnostics and final commit reject changed revisions while holding
the harness lease. This deliberately invalidates native evaluations even for
unrelated document edits. Immediately before native dispatch, the sidecar also
holds the harness lease and Taskmaster persistence/runtime locks while it checks
the captured evaluation generation, effective settings, reverified full proof
identity and native document revision; for a manual run it nests the current workspace and
active-recommendation check before starting the provider. Process dispatch keeps
that admission through child setup and spawn. Ollama revalidates on every
synchronous poll of the HTTP send future until response headers arrive, releasing
the locks between pending polls and before reading the response body. A snapshot
with neither identity fails closed.
An explicitly defined custom `ollama` harness shadows the native HTTP catalog
entry; it follows custom readiness/trust rules and never falls back to HTTP.
`reserve_snapshot` checks
custom trust and writes usage under the same mutation locks, reusing the native
ledger rules without recursively locking. `with_current_evaluation` checks that
the identity belongs to the supplied settings/generation/workspace, then guards
the existing live workspace-instance, current-evidence and recommendation writes.
Evaluation diagnostic updates use the same identity checks, so revoked output
cannot replace or clear the current error status. Reservations, native/custom
spawn and HTTP polling, cache/recommendation acceptance and rollup continuations
all compare the full verified identity: exact bundle-envelope digest, format,
signed policy, version, sequence and trusted-key fingerprint. Prompt page ranking
and truncation keep this identity separate from selected pages; full signed
hierarchy, applicability, provenance and manifest attestation remain in the proof.
Manual reservation admission takes the workspace lock inside the existing
harness → persistence/runtime-data boundary and retains it through the write.

`HarnessStore::with_locked_snapshot` and mutations share a retained
`harnesses.json.lock` OS file lease, not just an instance-local mutex, so a
second OrkWorks store/process cannot edit the definition during a guarded action.
Lock order is store mutex → harness file lease → Taskmaster process/file locks
→ runtime data. The lease coordinates cooperating OrkWorks mutations; it does
not freeze arbitrary direct filesystem edits or updates to an installed executable.
Do not wait for model completion inside this guard: revocation must remain able
to proceed while a model call is running.

Approval records executable trust; custom scheduling additionally requires enabled
analysis, a selected model, compatible effort, and a usage reservation. Approved
custom entries now report ready and use the integrated guarded evaluation path.
Native Rust process fixtures cover custom transport, activation and revocation
without requiring a shell. A focused Windows PR CI job exercises transport,
activation and trust. Native Windows verification remains a release requirement
and has not been performed in this checkout; locally passing portable fixtures
are not a substitute for that run. Guarded-spawn lock stress fixtures remain
Unix-only; the portable activation fixture covers revocation during a live call.
Built-in Codex/Claude CLI process fixtures also remain Unix-only; this job is
custom-adapter coverage, not a claim of complete native CLI validation on Windows.

Desktop packaging lives under `apps/desktop/`. `electron-builder.yml` defines the product metadata and `extraResources` layout, while `scripts/package-release.mjs` maps the current host platform/arch to the matching Rust target triple, stages the built `orkworksd` binary into `crates/orkworksd/target/release/`, and invokes `electron-builder` with the matching CLI arch flag. CI runs the same path from `.github/workflows/release.yml`, with separate macOS x64 and arm64 jobs so the packaged sidecar always matches the bundled Electron arch. In development, `pnpm dev` builds the debug sidecar before starting Electron, preventing the app from launching a stale Rust binary after sidecar changes.

The tag-driven release workflow uses the protected GitHub `release` environment.
For a credential-backed run, electron-builder signs the macOS app and nested
Rust sidecar, notarizes and staples the macOS artifacts, and signs the Windows
NSIS output with Authenticode. The workflow then verifies the generated
metadata/checksums and performs native platform checks before upload. Signing
credentials are injected only in the trusted release jobs; they are not stored
in source control or packaged into the application. The base64 App Store
Connect Team Key is decoded to a mode-600 file under the macOS runner's
temporary directory, passed to electron-builder by path, and removed after
packaging. Windows signing currently uses base64 `.pfx`/`.p12` secrets;
managed signing requires separate workflow integration and is not wired here.

Runtime updater network/download and restart operations remain Electron-main
work owned by issue #511. The signing wiring here does not implement or claim
installed older-build update testing.

## Preload bridge (security boundary)

The [selected-session Git changes viewer](../../specs/session-git-changes.md)
uses session-scoped authenticated `git-change-bases`, `git-changes` and opaque
snapshot/file routes in `http/git_changes_handlers.rs`. `git_changes.rs` owns
bounded comparison captures in private temporary Git storage; original
repositories are never written. `git_changes_fs.rs` anchors worktree traversal
without following symlinks. Built-in EOL policy in `git_changes_eol.rs` resolves
only bounded control copies in the private repository, preserving index fallback;
source attribute/ignore files never enter libgit2 filesystem readers. The Git-context cache owns ephemeral snapshot and
capture-admission handles; workspace adoption replaces them to invalidate old
requests, independently of ordinary Git-context polling. Electron main exposes
only the Changes-specific preload request and retains the desktop credential
and backend-generation fence. See [ADR 0091](../adr/0091-read-only-session-git-changes.md).

Electron runs with `nodeIntegration: false` and `contextIsolation: true` (ADR 0009). The renderer cannot call Node APIs directly. All privileged operations go through `electron/preload.ts`, which exposes `window.orkworks` with backend discovery, workspace memory, layout memory, menu-command, panel-visibility, and app-settings methods. Adding new capabilities requires extending the preload, not relaxing context isolation. `titleBarStyle: 'hiddenInset'` is set on macOS so the web content extends into the title bar area; the renderer reads `window.orkworks.platform` (exposed synchronously by the preload) to apply a `data-platform` attribute on `<html>`, which CSS uses to add traffic-light clearance (`padding-left: 80px`) on darwin only.

Leaving `sandbox` unset on `webPreferences` means Electron runs `preload.ts` under its sandboxed preload loader, which only resolves Node/Electron built-ins — a plain per-file `tsc` require of any other local `electron/*.ts` module fails at runtime (`module not found`) even though it type-checks and compiles cleanly. `scripts/build-preload.mjs` (config in `scripts/preloadBuildConfig.mjs`) runs esbuild after `tsc` in the `dev`, `build`, and `dist` npm scripts to bundle `electron/preload.ts` and its local imports into a single self-contained `dist-electron/preload.js`, keeping `electron` external so Electron's own binding still resolves it. The rest of `electron/*.ts` (main-process code, not sandboxed) keeps its plain per-file `tsc` output and may freely `require()` sibling modules.

`electron/layoutMemory.ts` retains the legacy Dockview `layout.json` compatibility API in the Electron user data directory. The fixed-shell renderer neither reads nor writes this record and does not restore the old panel graph.

The fixed shell uses Electron-owned `shell-layout.json` and
`workspace-navigation.json` records through `electron/shellLayoutMemory.ts`
and `electron/workspaceNavigationMemory.ts`. The layout record stores only
validated Sessions/inspector widths, Sessions visibility, density, and an optional
inspector pin preference (missing means unpinned). The fixed right inspector
overlies central content with a subtly translucent background until pinned;
pinning reserves width and uses an opaque background. Narrow windows retain
the temporary page. See [ADR 0083](../adr/0083-inspector-overlay-and-pin.md). The
workspace record stores only the last central Terminal or Review surface for
each retained canonical workspace key; Electron derives that key and its
workspace generation from the ready sidecar lifecycle. Both records use the
retained `.shell-memory.lock`, bounded revisions, atomic replacement and
read-back verification. Failed or stale writes leave the current UI usable;
reset and deletion are ordered barriers, and failed navigation deletion can
be retried through the exact saved canonical key. Workflow presentation state
remains outside these records until its separately gated issue is accepted.
The existing Dockview `layout.json` is retained untouched for downgrade
safety. Issue #779 removed the renderer's legacy layout replay when it installed
the fixed shell.

`electron/settingsMemory.ts` owns app-level settings in Electron `userData`, including hotkey validation, default hotkeys, a persisted `debug.showSessionIds` flag for gating internal session identifiers in the Details panel, persisted menu accelerators, and durable provider settings (`ProviderSettings`). In user-facing copy these provider settings are model provider settings; internal code keeps the existing `ProviderSettings` name. `getSettings()` and successful `saveHotkeys()` responses include a renderer-facing `defaultHotkeys` copy sourced from the main process, so the settings UI can restore defaults without duplicating canonical accelerators. Backend restoration pushes saved retention and provider settings into the sidecar after port discovery. Explicit retention saves return sidecar application status. `electron/providerSettingsSync.ts` handles the `POST /settings/providers` push during startup and workspace-switch restoration. Peon Apply tests and applies a provider selection through its transaction; Save persists the matching applied selection and returns the updated settings without a provider application status. Provider model lists are fetched on demand from `POST /settings/providers/:id/models` and cached in memory; the renderer reads them via the `getProviderModels` preload method. The discovery response and IPC cache preserve optional `modelOptions` metadata alongside model IDs. Ordinary Codex session creation accepts optional `reasoningEffort` with an explicit model, revalidates it against the actual launch executable’s live model catalog, and appends its native CLI override before spawn. Image-session normalization preserves the same option; omission preserves tool defaults. Interactive effort discovery is independent of background-inference model commands. Peon verification preserves the compatibility `models` ID list and also returns `modelOptions` metadata. Codex populates that metadata from the installed CLI's `app-server --stdio` `model/list` response, including model-specific reasoning-effort choices and defaults; if a refresh cannot provide a catalog after provider verification, Electron reuses the last successful catalog and marks it stale, while Codex can still apply its default model without an explicit model. Draft provider verification in Settings bypasses that cache through the `verifyPeonProvider` preload bridge and the Peon verification transaction, so unsaved Ollama URLs can be checked before persistence. Provider selection is persisted through `savePeonSelection`; provider labels come from the runtime catalog. The preload contract exposes `saveActiveHarnessesWithIntegrations(ids)` as the typed renderer entry point for the Tools subsection's combined active-tool persistence and integration reconciliation flow. That operation is deliberately an Electron-main orchestration seam, not a renderer-to-sidecar mutation shortcut: the renderer submits only the requested active-tool IDs, Electron main persists them first, plans which coding tools need install/repair/uninstall, and — before making any of those changes — shows one batched native confirmation dialog (`dialog.showMessageBox`, main-process only) listing every planned mutation's tool name, operation, and affected paths, escalating to a warning-styled dialog if any of them execute code automatically. Declining leaves the active-tool selection persisted but skips every planned mutation (reported per-tool as a `confirmation_declined` failure); confirming reconciles each coding tool independently through the existing integration routes as before. The call returns one structured `ActiveHarnessSaveResult` containing the active-harness outcome plus a per-tool partial result for install, repair, uninstall, unsupported skip, failure, or `stale_workspace`. Electron main rejects old-generation results as `stale_workspace` when the workspace path or sidecar generation changes mid-save — including while the confirmation dialog is still open — so a replaced workspace cannot report a successful save for the old one. A second preload entry, `enableHarnessIntegrationImmediate(ids, adapterId, targetId)`, backs the Settings coding-tools card's off-to-on toggle transition: it calls `enableHarnessImmediate` in `electron/activeHarnessIntegration.ts`, which composes the same `persistActiveHarnesses` step with the existing single-group `reconcileGroupedIntegration` pipeline (the same one `saveActiveHarnessesWithIntegrations` uses per group) so flipping one tool on persists and installs/repairs only that tool's integration group immediately, without waiting for the modal-wide Save and without touching any other tool's group. Because this path reuses the shared `confirmMutations` Electron-main step, flipping a tool's toggle off→on pops the native OS confirmation dialog listing the install or repair it is about to perform (tool name, operation, and affected paths) at toggle time rather than deferring that prompt to a later Save click — but only when the immediate reconcile plans a mutation; already-healthy, unsupported, and failed status lookups exit before confirmation with no dialog. This is intentional, not a regression, and prompts to defer it to Save should be rejected. Turning a toggle off remains draft-only; its cleanup mutation and retry both wait for Save. See `docs/superpowers/specs/2026-08-29-active-coding-tool-hook-toggle-design.md` for the full rationale, including why the per-row Reconcile button introduced right before this change was removed again in favor of the toggle itself.


The Settings modal merges successful responses into their owning section, so a
late response cannot restore another section's older snapshot. Independent
section save scopes fence UI status by latest request and committed settings by
latest successful request; an earlier success still commits when its successor
fails. Closing or unmounting invalidates those scopes. Hotkey draft revisions
preserve edits made during a pending save. Peon Apply/Save also check their
mutation, verification and modal identities before publishing UI state or
releasing busy state; Electron's existing Peon transaction serializes Apply/Save
mutations independently of these renderer checks.

Electron-main denies renderer popup and navigation requests, delegates valid HTTP(S) links to the OS default browser, and keeps same-origin Vite reloads inside Electron during development. xterm's OSC-8 terminal links use the narrow `openExternalLink` preload bridge, so they bypass renderer `window.open()` while still receiving Electron-main URL validation. Renderer load/process failures log bounded diagnostics and show a resource-free local recovery document; its retry returns to the captured original app URL, with only that exact packaged `file:` target allowlisted.

`inspectSessionApp(workspacePath, sessionId)` and
`openSessionApp(workspacePath, sessionId)` expose only bounded native desktop
opening. Electron reads current session/tool data from its own ready sidecar;
`electron/sessionAppOpener.ts` validates the exact native UUID and built-in tool,
checks app registration and Claude capability/lifecycle/directory, and revalidates
identity and lifecycle after asynchronous probing before dispatch. Main binds
requests to workspace generation and aborts stale work. Claude uses fixed argv with a 15-second timeout, a 256 KiB output bound and an
environment without `ORKWORKS_` capabilities. Unix executes the tool directly;
Windows uses the absolute OS command interpreter with only a fixed Claude help
or UUID-validated handoff command, with AutoRun/delayed expansion disabled, to
support npm command shims. General links remain HTTP(S)-only. See the
[native application contract](harness-integration-contracts.md#native-application-opening).

### Desktop skill management

[ADR 0097](../adr/0097-desktop-skill-management.md) places skill discovery and
qualified APM execution in Electron main. `skillInventory.ts` inventories known
Workspace/Personal roots; `skillInstall.ts` owns private previews, input
fingerprints, single-use review IDs and per-destination install exclusion. Narrow
preload operations expose inventory/detail, native local-source selection,
preview and confirmed install. The renderer never receives a general filesystem
or command-execution API. Source and installed-profile coverage is recorded in
[skill management qualification](../validation/skills-management.md). Installed
skills do not establish native loading/use or change session lifecycle authority.

## Frontend → backend API

[ADR 0092](../adr/0092-workspace-update-stream.md) adds one workspace-scoped
read-only update socket owned by `workspaceSessionController.ts`. An initial
complete session snapshot is followed by complete changed rows, removals and
ordering changes. Stream messages share a versioned instance/workspace cursor;
a gap or reconnect requires a new snapshot. The sidecar reuses the complete REST
query, including the live Peon/observation overlay, and never infers removal from
an incomplete query. Metadata watches and runtime/mutation signals coalesce
projections to two-second starts; thirty-second reconciliation covers Git/cwd,
clock and external changes. Native watcher paths are canonicalized, and directory
replacement/overflow rearms watches. Stream failure restores existing polling.
The source API does not imply these changes are in a published installer.

`recommendationSource.ts` shares one workspace-scoped recommendation list across
the shell Actions overview and mounted recommendations panel. Stream invalidation
and local actions refresh that source; disconnected compatibility mode has one
five-second fallback timer. The panel refreshes authenticated analysis status
independently of list failures. Terminal sockets and PTY ownership are unchanged.

`apps/desktop/src/api.ts` defines TypeScript types and fetch wrappers for the REST API. `workspaceSessionController.ts` owns the single renderer polling loop, workspace switching, session refresh/merge, pending-create reconciliation, active-session restoration, session lifecycle actions, terminal pruning, and stale/disposed operation rejection. `App.tsx` retains React state and view wiring, passes controller callbacks into the UI, and enables or disables polling with the application lifecycle. `SettingsModal.tsx` owns its own durable snapshot versus draft, subsection-local save/revert behavior, provider verification, and partial-save status directly via component state; the Coding tools subsection saves through the Electron-main orchestration seam above, Hotkeys owns its own Save/Cancel/Restore lifecycle, Providers keep Apply/Save, and Retention plus Debug stay immediate or field-local. `settingsController.ts` itself is now scoped to `mergeIntegrationOperationFailures`, the pre-grouped-to-grouped result merge for coding-tool integration operation failures. Electron remains authoritative for defaults and persistence. Session state flows: Rust structs → JSON API → `SessionInfo`/`WorkspaceInfo` TS types → controller callbacks → React state → components. The payload exposes canonical `harnessId`, `modelProviderId`, and `modelId` fields alongside legacy fields during the migration window. Its session state is the canonical `creating → alive → stopping → dead` lifecycle, with alive-only attention (`working`, `idle`, `needs_you`, `blocked`, `failed`, or `capped`); `connectivity`, `terminalOutcome`, `lastActivityAt`, `lastOutputAt`, and `resumeOptions` provide supporting runtime and history context. `lastActivityAt` tracks meaningful situation changes for task history, while `lastOutputAt` tracks raw PTY output; session-list recency selects the newest valid timestamp. PTY lifetime is session-runtime-owned in the sidecar; the terminal WebSocket is an attach/detach transport rather than the thing that keeps the PTY alive.

Key endpoints: `POST /workspace`, `POST /workspace/active-session`, `PUT /workspace/active-harnesses`, `GET/POST /sessions`, `DELETE /sessions/:id`, `DELETE /sessions/:id/forget`, `POST /sessions/:id/resume`, `POST /sessions/:id/harness-session`, `POST /sessions/:id/attention`, `POST /sessions/:id/plan-path`, `POST /sessions/:id/debug-injection`, `GET /sessions/:id/plan-content`, `POST /sessions/:id/request-plan-review`, `POST /taskmaster/analyze`, `POST /taskmaster/recommendations/:id/accept`, `POST /taskmaster/recommendations/:id/complete`, `POST /taskmaster/recommendations/:id/completion-packet`, `GET /workspace/integrations/:harness_id/status`, `POST /workspace/integrations/:harness_id/install`, `POST /workspace/integrations/:harness_id/uninstall`, `GET /sessions/:id/terminal-output`, `GET /sessions/:id/summary-log`, `GET/POST /sessions/:id/workflow-observations`, `GET /providers`, `GET /providers/:id/models`, `POST /settings/providers`, `POST /settings/providers/ollama/verify`, `POST /settings/retention`, `GET/POST /harnesses`, `POST /harnesses/:id/duplicate`, `POST /harnesses/:id/remove-profile`, `PUT/DELETE /harnesses/:id`, and `WS /sessions/:id/terminal`.

### Harness configuration and provider projection

The sidecar persists `~/.orkworks/harnesses.json` as version 3. The document
contains declarative `custom` definitions and sparse built-in `overrides`; the
compatibility-profile map is sidecar-owned state and is never accepted from
editable harness JSON. Registry resolution produces an immutable snapshot with
the effective definition, origin, stored override, and read-only compatibility
metadata. The renderer flattens the nested wire entry into
`HarnessConfigEntry`; Electron main keeps its own `ElectronHarnessConfig` and
integration contract types at the preload boundary.

`GET /harnesses` returns an opaque `documentRevision` with the resolved entries.
`POST /harnesses/:id/duplicate` returns a proposed ID/name and editable,
server-resolved definition without persisting it or installing a hook. Create,
replace, built-in patch, profile removal, reset, and delete operations carry
`expectedRevision`; stale writes return `409` with
`harness_config_revision_changed`. Custom JSON is strict, bounded to 256 KiB,
rejects duplicate or unknown fields, and cannot name integration/session-signal
bindings, hook paths, reporter commands, or executable assets. The sidecar is
authoritative; the Settings editor performs matching local diagnostics and
keeps an invalid draft visible rather than sending it.

Harness-backed model providers are projected dynamically from the same resolved
registry. A custom coding tool therefore gets a stable provider ID and keeps
its provider settings independently from the source tool. Removing a custom
tool also removes its projected provider and clears a removed Peon selection
without redirecting to another provider.

The editor does not own integration lifecycle. It shows compatibility profiles
and derived signals outside editable JSON, while the existing Electron-main
confirmation boundary still owns active-tool persistence and hook install,
repair, and removal. Shared integrations are keyed by adapter/target, so a
Copilot hook remains installed while any active compatible coding tool consumes
it; a custom copy can be disabled while its parent remains active, or vice
versa.

`GET /sessions/:id/summary-log` returns `{ "entries": [{ "timestamp", "summary", "source", "confidence", "recommendationId?" }] }` in append order, where `confidence` is nullable and `recommendationId` links Taskmaster handoff events back to their persisted recommendation; a missing workspace, session log, or checkpoint returns `{ "entries": [] }`. Internal event types and status fields are excluded. `apps/desktop/src/api.ts`'s `getSummaryLog` fetches it, and `SessionDetailPanel` renders the checkpoint history as a "Task history" section (ADR 0029) — distinct from the session `label`, whose `labelSource` provenance allows automatic Peon/Codex title updates while preserving user labels. (design, not yet implemented — see issue #313) ADR 0042 plans to remove this route and "Task history" section in favor of the current-summary snapshot and workflow-observation contract described below.

`GET /sessions/:id/workflow-observations` returns `{ "observations": [{ "id", "sequence", "observedAt", "kind", "description", "evidence", "problemArea?", "reportedImpact", "source", "confidence" }] }`, ascending by `sequence`; a missing workspace or unknown session returns `{ "observations": [] }`. This exposes the raw evidence records the `POST` route on the same path writes into `workflow_observations::WorkflowObservationStore` (see "Metadata protocol" above); it has no request body or auth requirement since it is a read of already-workspace-scoped data. `apps/desktop/src/api.ts`'s `getSessionWorkflowObservations` fetches it, and `SessionDetailPanel` renders it as a "Workflow observations" list gated behind the existing `showDebugMetadata` debug setting, alongside "Peon diagnostics" — this is the only viewer for individual observation records; `RecommendationsPanel`'s "Evidence diagnostics" only shows aggregated parse/validation diagnostics, not the underlying accepted evidence.

For Codex, an identity report that also carries `ORKWORKS_REPORT_TOKEN` triggers a bounded, read-only lookup of `$CODEX_HOME/state_5.sqlite` (or `~/.codex/state_5.sqlite`), selecting `threads.name` then `threads.title` by exact native ID and storing the result as a Codex label when the current label is still automatic. Missing or unsupported data preserves the current label.

Codex runtime startup probes the configured executable's `--help` with bounded
output and a three-second deadline. When it advertises `--no-daemon`, both new
launches and exact resume receive that flag before their existing arguments.
This avoids Codex 0.159.2's shared app-server retaining the first launch's
OrkWorks environment across conversations. Successful older help output without
the option preserves the original arguments; failed or oversized probes abort
startup. Existing sessions require relaunch. Global Codex configuration and
its shared daemon are untouched. See [ADR 0072](../adr/0072-codex-session-process-isolation.md).

The approved [native approval design](../superpowers/specs/2026-10-03-codex-native-approval-status-design.md)
([ADR 0076](../adr/0076-codex-owned-native-approval-observer.md)) adds a gated
runtime path. Its exact version/platform compatibility table is empty, so
current sessions retain the direct isolation above. Before any enabled native
launch, argument and executable checks must succeed; the runtime then owns a
private authenticated app-server, remote TUI and separate bounded passive
observer. `futures-util` and `tokio-tungstenite` support that production client.
The observer cannot answer approvals or start turns. Renderer detachment does
not end ownership; native child exit or session shutdown tears down the owned
runtime. A fixed two-second hook grace and fresh, complete singleton-root
observations feed conditional attention effects without adding public schemas.

Native startup and the installed diagnostic have a closed listener-ownership
gate before bearer delivery. Child liveness and an expected initialize shape
do not attest a listener after a released-port race.
[#763](https://github.com/Rambolarsen/orkworks/issues/763) tracks the required
reviewed handoff/identity contract; controlled fake fixtures cannot enable it.
The owner-approved dormant macOS arm64 `listener_identity` helper proves an
already-connected IPv4 stream against the retained owned leader's exact accepted
socket. Its SDK adapter, bounded inventory, blocking worker, and same-stream
guard do not activate startup or qualify the native TUI's independent reconnects.
All-client support and installed evidence remain required.

Native clearing has a separate unfenced-writer gate. The current MVP contract
permits direct agent JSON writes, which sidecar revisions and completed-file
identity checks cannot serialize with a final check/rename. The contract
approved in [#761](https://github.com/Rambolarsen/orkworks/issues/761) routes
active metadata mutations for native-enabled Codex sessions through a
versioned sidecar API. The launch adapter advertises that protocol only for a
migrated integration. Direct JSON reads remain available; active direct JSON
writes would be unsupported for those sessions. The sidecar keeps separate
work-metadata and attention source/confidence fields, records
`attentionOrigin`, rejects stale revisions, persists an attention-specific
update time, and serializes the full
read/check/modify/write transaction with native clears against the single
session record. This changes the current write contract for eligible sessions.
The approval covers the written contract only; it does not authorize runtime
changes under #761. Producer migration, installed behavioral verification,
and the separate #690/#763 gates remain required, so production clearing stays
disabled. [#788](https://github.com/Rambolarsen/orkworks/issues/788) owns the
transactional API/writer boundary and behavioral tests; dependent [#789](https://github.com/Rambolarsen/orkworks/issues/789)
owns Codex producer migration and handshake verification. See the
[approved mediated metadata design](../superpowers/specs/2026-10-08-codex-native-attention-layer-design.md)
and [verification record](../superpowers/verification/2026-10-05-codex-native-approval-status.md).

Codex session identity stays bound to the conversation OrkWorks launched:
differing Codex hook IDs are ignored, except for an authenticated root
`SessionStart` with `source=clear` after OrkWorks records that explicit reset.
Internal Codex CLI subagents remain within their parent OrkWorks session and
receive no separate OrkWorks session ID. The report token authenticates the
OrkWorks session but is inherited by child processes, so it does not verify
operating-system process identity. Resume requires the exact ID and a matching
`threads` row whose `rollout_path` exists in the supported local Codex store.
Missing or unsaved IDs never fall back to `codex resume --last`; see
[ADR 0068](../adr/0068-codex-subagents-share-owning-session-identity.md).

Every spawned PTY session receives `ORKWORKS_SESSION_ID` and `ORKWORKS_PORT` in its environment, so an in-session hook can address the sidecar without any config look-up. Harness-native session IDs are reported through `POST /sessions/:id/harness-session`, which writes `resume.harnessSessionId` plus source/confidence/captured-at metadata. Deterministic supported sources such as the installable OpenCode plugin (issue #110; reads the same two env vars via `process.env` inside OpenCode's plugin host) and Claude hook JSON outrank Peon inference; interactive status probes remain user-triggered. Attention is deterministic where the harness exposes a needs-input-shaped event: `POST /sessions/:id/attention` accepts hook-reported statuses (hook-authoritative `working` only when the binding grants the Attention capability). OpenCode's plugin reports `permission.asked` and `question.asked` as `waiting_for_input`; `permission.replied`, `question.replied`, and `question.rejected` resolve a matching request ID and report the effective state, which remains waiting while another request is pending and otherwise returns to the latest turn state. `session.status` busy and `session.idle` update that turn state but cannot clear pending requests (issue #104).

Other harness native IDs use the HTTP route above. For each Codex runtime, the
sidecar creates a private temporary mailbox and passes that path to the Codex
hook process. The reporter publishes native-ID reports there when the mailbox
is available, for cases where Codex's command network sandbox blocks loopback;
it does not enable network access. The runtime validates reports through the
existing in-memory report token (re-checked immediately before each report is
applied), hook fingerprint, and Codex identity-reset rules, then removes them,
draining once more before terminal cleanup so a short-lived session doesn't
lose a report queued right before exit. It pins the mailbox directory and uses
handle-bound operations on Unix; on Windows, a retained directory handle
prevents path replacement while reports are processed. The mailbox directory
itself is only ever removed on Windows, where the held handle makes that safe;
on Unix, removing it by path would be a TOCTOU against a child that controls
that path for the mailbox's whole lifetime, so it is leaked instead. The
report token is not written into mailbox files, and the mailbox path is
stripped from every other child's forwarded environment so it cannot leak
across sessions.
Child processes inherit the mailbox capability, so it does not prove process
identity. See [ADR 0069](../adr/0069-codex-session-id-hook-report-mailbox.md).

`POST /sessions/:id/attention` accepts `{status, message?, planPath?, cwd?}` from a harness's own notification mechanism. `planPath` is optional: a string sets it, `null` clears it, and omission preserves it. When no harness path is present, sidecar output persistence may associate a validated printed path beneath `docs/superpowers/plans/` or `specs/`. `GET /sessions/:id/plan-content` returns the validated Markdown content only to Electron main; `POST /sessions/:id/request-plan-review` accepts only the selected session ID and injects the fixed, user-approved prompt after PTY acceptance. A terminal plan link uses the narrower authenticated `POST /sessions/:id/select-terminal-plan` handoff: renderer passes only that session ID plus the exact clicked terminal text, while Rust resolves and persists an anchored reference in the launch worktree or a linked worktree from the same Git family. Electron generates a per-sidecar secret for these endpoints, passes it in the sidecar environment, and excludes it from PTY child-process environments. The renderer never receives filesystem paths or supplies arbitrary terminal text; see [ADR 0025](../adr/0025-authenticated-session-plan-handoff.md), [ADR 0034](../adr/0034-user-approved-session-review-prompt.md), and [ADR 0039](../adr/0039-terminal-plan-link-selection.md).

`GET/POST /workspace/integrations/:harness_id/{status,install,uninstall}` back both the explicit per-tool Settings affordance and Electron main's combined Tools Save orchestration. The renderer never calls those mutation routes directly for the combined save path: it invokes `saveActiveHarnessesWithIntegrations(ids)`, and Electron main performs the ordered `PUT /workspace/active-harnesses` plus per-tool status/install/uninstall sequence. The retired Gemini handler remains available to read and preserve existing owned settings, but Gemini is no longer offered as a coding tool for new sessions. Antigravity CLI has no compiled signal or integration binding. Installation writes an idempotent, ownership-marked artifact into the tool's own workspace-local config (e.g. `.claude/settings.local.json`, never `settings.json`) and never runs at session spawn; see [ADR 0026](../adr/0026-resolved-harness-capability-registry.md). OpenCode is the first integration whose owned artifact is a whole file rather than a JSON fragment merged into a shared document — `.opencode/plugins/orkworks-session-reporter.js` reports the native OpenCode session ID via a `session.created` plugin hook (issue #110) plus attention events mapped per the evidence register (issue #104) — so its ownership check compares the file's bytes directly instead of a JSON marker field. The shared reporter script (`report-harness-event.sh`/`.ps1`) is copied to `~/.orkworks/hook-scripts/` on install, so installed commands remain stable across app updates and AppImage mount changes. For the `claude-code` marker specifically, it also extracts `cwd` from Claude Code's hook JSON payload (alongside the `session_id` it already captures) and forwards it on the same attention POST — see [ADR 0032](../adr/0032-harness-reported-cwd-via-hook-payload.md).

`POST /sessions/:id/debug-injection` accepts `{attention, message?}` from the dev-only Details-panel picker (gated behind `showDebugMetadata`) and writes through the same merge path as `report_attention`, tagged `attentionSource: "debug"`, `attentionConfidence: 0.0`. Both merges gate through the shared `source_priority::can_overwrite` ladder (see [ADR 0005](../adr/0005-metadata-source-priority.md)): a `debug`-sourced write is ignored if the session currently carries `user`- or `agent`-sourced metadata (the live-signal tiers), and is otherwise accepted — including over `process`/`peon` state, which is what keeps debug injection usable on live sessions; any subsequent real signal reclaims the session on its next update. Rejects with 400 if the session's `lifecycle` isn't `alive`. `capped` injections route a non-empty `message` to the in-memory `usageLimitResetHint` handle field rather than the persisted `summary`, matching how the `Capped · <hint>` badge reads it; omitting `message` leaves any existing hint untouched.

`POST /sessions` also accepts optional `initialImages` (`name`, `mimeType`,
`dataBase64`) and `expectedWorkspacePath`. Nonempty images require the main-only
secret and a matching workspace identity. Electron's generation-bound
`create-session-with-images` IPC validates bounds and positively negotiates
`GET /session-image-context` version 1 before forwarding. The sidecar decodes
supported images with bounded allocation, publishes private session-owned files
before metadata admission, and retains them until Forget/retention. Unix storage
operations are relative to retained directory handles. Native Codex image starts
pass both image paths and the text in argv, and skip PTY prompt submission because
Codex automatically submits startup images. Other qualified image readers receive
explicit file references through the existing readiness-gated prompt owner.
Unqualified tool/version/platform routes remain disabled. See
[image context](../../specs/session-image-context.md) and
[ADR 0089](../adr/0089-new-session-image-context.md).

`POST /sessions` now accepts `{ harnessId, model, initialPrompt }`. The renderer's inline New agent session composer uses a model dropdown populated by the selected coding tool's resolved model capability, with a coding-tool-default option when no explicit model is selected or discovery is unavailable. Saved IDs absent from that catalog fall back to the coding tool default; pending or late results from another tool cannot supply its model selection. It labels harness choices as coding tools, can fall back to the default shell session if harness metadata is temporarily unavailable, and still sends the selected harness config id so session rows and remembered-session resume behavior remain compatible. The response returns as soon as the `"creating"` metadata record is persisted, before the harness has actually spawned — the PTY spawn runs as a detached background task, mirroring how `POST /sessions/:id/resume` and daemon-restart reconciliation already leave a session observable in `"creating"`. A spawn failure surfaces asynchronously as `status: "error"`, observed via the existing `GET /sessions` poll, rather than as a synchronous error response (issue #302).

If the initial metadata write fails, creation returns an error before startup or
label inference is scheduled and removes the provisional in-memory session.
This applies to text-only and image-backed requests; the composer retains its
draft on synchronous creation failure. The historical missing Codex launches
remain under investigation in [#1009](https://github.com/Rambolarsen/orkworks/issues/1009).

For a new Codex, OpenCode, or Claude Code session with an initial prompt, the runtime waits
for the TUI to enable bracketed paste (`ESC[?2004h`) and for a per-harness
quiet-time settle before writing the prompt as one bracketed paste. Codex
keeps a 200ms settle. OpenCode requires 1.5s since paste-enable: its TUI
enables bracketed paste ~0.1s after spawn but starts reading input only with
its first real render ~1.1s in, and input written in between is silently
discarded (opencode v2.0.32). Claude Code likewise drops input for roughly
the first 0.5s after paste-enable (measured on 2.1.292) and uses the same 1.5s
floor. In an untrusted folder Claude first shows a trust chooser (default
"No, exit") that also enables bracketed paste; the runtime recognizes it and
holds all delivery, including the hard timeout, until it tears down
(`ESC[?2004l`) and the editor re-enables paste. The runtime sends Enter
separately, 200ms after the paste write completes, ahead of later queued input.
This avoids cooked PTY newline conversion, startup input loss, and Codex's
paste-burst Enter suppression. Resize and kill controls bypass waiting prompt
input. Hookless activity tracking is armed after accepted submission, so a
slow startup cannot exhaust its work-signal window before delivery. A response
that arrives before write acknowledgement retains its echo-filtered activity
evidence until acceptance; terminal output itself continues normally. This
retains the existing two-second post-spawn banner grace for hookless activity
inference: a response completed entirely during that grace can leave attention
idle. The separate status-inference policy follow-up is tracked in
[issue #884](https://github.com/Rambolarsen/orkworks/issues/884). Stop and
child exit still cancel pending delivery. Failed initial-prompt writes stop the runtime with an error; a failed paste
never sends the automatic Enter. Queued output is processed before a quiet-time
deadline can dispatch input. Both readiness and quiet waiting have a hard
10-second bound; overrides that never enable paste mode then use plain
prompt-plus-Enter. Other harnesses retain plain input delivery.

`electron/workspaceMemory.ts` persists the last workspace path and recent workspace directories to the Electron user data directory, enabling workspace restore on relaunch. The sidecar persists workspace-scoped state to `~/.orkworks/workspaces/<path-hash>/workspace.json`; Aider's versioned notification-command preference is separately stored at `integrations/aider.json`, so no repository Aider configuration is edited.

## Metadata protocol

The source-owned `session-metadata.py` producer prepares API-only agent reporting
for #789. With `ORKWORKS_SESSION_METADATA_API_VERSION=1`, it requires authenticated
GET, validation-only PATCH, and follow-up GET with the same revision before a
revisioned mutation, preserves the caller's revision, and fails without direct-file fallback. Codex integration reconciliation
installs the helper beside its reporters; a marked `SessionStart` emits the
instructions as native additional context. The agent must run bootstrap through
its actual sandboxed tool context; the host hook does not prove that reachability.
The closed `sessionMetadataWriteProtocol` definition field accepts only
`sidecar-v1` on the source-owned Codex definition; custom definitions and overrides
cannot add it. The embedded definition still omits it, the launch marker is not
advertised, and inherited copies of that marker are filtered. This preparation
changes no current attention mapping and supplies no native-clear eligibility.


The helper also accepts explicit `--http-proxy http://127.0.0.1:<port>` after
bootstrap/get/patch for an owner-selected trusted sandbox relay. It pins the
sidecar origin, ignores ambient proxy variables in direct mode, rejects redirects
and never falls back after a selected proxy fails. The proxy receives the bearer
capability; loopback validation is not identity proof. Automatic callers do not
select this transport. See the [approved proxy design](../superpowers/specs/2026-10-24-sandbox-metadata-proxy-design.md).

The versioned agent API uses `metadata/session_transactions.rs`: checked, durable
revisions are stored in the canonical session JSON, and scoped mutations hold the
retained store's per-session lock across read/check/modify/replace. Internal
sidecar writers mutate the latest record in that lane. Legacy reads hydrate
provenance and age without a write. Separate reader stores share no write locks
or native ownership. Projection I/O and parsing failures return an explicit error.

`GET /sessions/:id/metadata` returns the canonical record and opaque
`metadataRevision`. `PATCH /sessions/:id/metadata` accepts
`{metadataRevision, patch:{attentionState?, agentMetadata?}}` using the live
session report bearer token. Attention and work fields are complete snapshots;
summary and relative plan path are independent scopes. The sidecar owns provenance,
timestamps, and plan resolution. Stale revisions, losing source checks, and
agent changes to user-selected plans return conflict; invalid fields return 400.
`?validateOnly=true` performs the same authentication, revision, schema, and path
checks without a write or source mutation.

Attention provenance and age are independent of work metadata. Every accepted
attention write revokes prior native ownership; work/summary/plan-only writes
preserve it. Peon arbitrates attention and work independently, retaining the
strict greater-than-15-second agent freshness boundary. Live native-owned and
validated Codex permission waits remain protected while their runtime ownership
is current. Session details attribute the attention tuple to its own source.
Producer migration in [#789](https://github.com/Rambolarsen/orkworks/issues/789)
remains pending, and all native approval rollout gates remain closed.

- `~/.orkworks/workspaces/<hash>/sessions/<id>.json` — session state, including the current-summary snapshot (`summary`, `summarySource`, `summaryConfidence`, `summaryObservedAt`, all four updated or cleared together by the metadata API — ADR 0042). Broader summary handoff work remains tracked in #313
- `~/.orkworks/workspaces/<hash>/events/<id>.ndjson` — append-only event log with durable, exact consecutive-deduplicated summary checkpoints and accepted provenance
- `~/.orkworks/workspaces/<hash>/events/<id>.terminal` — recent raw terminal replay, bounded on append to the newest 1,000 lines and 1 MiB; existing oversized dormant files remain unchanged until their next append
- `~/.orkworks/workspaces/<hash>/events/<id>.terminal-size` — the PTY's `cols`x`rows`, used to render dead-session terminal replay at its recorded size instead of the current panel width. Written authoritatively at the moment a session reaches a terminal status (`killed`/`ended`/`error`), and best-effort after a successful live resize when the requested grid differs from the known durable grid. This also persists an initial grid when a successful same-size resize occurs before that grid is durable, so a daemon restart mid-session can leave a usable last-known size for orphan reconciliation (`metadata::reconcile_orphaned_session`), which has no in-memory runtime handle to read a size from and never reaches the terminal-status transition itself. Still absent for sessions that ended before this file existed and for sessions that never lived long enough to make a grid durable before an untimely daemon restart — both cases fall back to fit-to-container replay, which can misrender recorded output that used absolute-column cursor addressing computed for a different width than the container happens to fit to.
- `~/.orkworks/workspaces/<hash>/workflow-observations/<session-id>.ndjson` and `~/.orkworks/workspaces/<hash>/workflow-observations/sequence` — bounded (1,000 records/2 MiB per session), sequenced `WorkflowObservation` evidence recorded through one shared module (`workflow_observations.rs`) from the authenticated `POST /sessions/:id/workflow-observations` agent-report route (`http/workflow_observation_handlers.rs`); records contain an optional `problemArea` in addition to the existing `kind`, `description`, `evidence`, impact, source, confidence, and fingerprint fields. New explicit problem areas use sidecar-owned v2 identity; omitted values preserve the legacy v1 description-based fingerprint. This evidence is durable improvement input for Taskmaster and deliberately separate from the current-summary snapshot above (ADR 0042). The route authenticates with a per-session `ORKWORKS_REPORT_TOKEN` bearer capability, generated from OS randomness (`getrandom`) at session start/resume and never persisted, logged, or serialized; session creation/resume fails closed if OS randomness is unavailable rather than spawning with a weak or empty token. Peon-inferred recording and Taskmaster's `improve_workflow` correlation are implemented. Peon's per-scan idempotency keys are generation-scoped, so re-detecting the same friction each scan cycle would otherwise accumulate one stored observation per scan; the retention loop's `observation_spam_cleanup_once` pass therefore trims same-session, same-fingerprint Peon-origin scan spam every 5 minutes (agent reports are deliberate durable evidence and are never trimmed, nor do they participate in the Peon spam grouping): within each session segment it keeps each fingerprint's first and latest occurrence plus re-occurrences spaced more than 30 minutes after the previous kept hit, drops the closer-gap re-detections in between (tombstoning them like bounded-storage eviction so the 15-minute retry window survives trimming; backward clock steps are exempt), and never removes an observation cited in evidence of an Accepted/Executing/RolledUp recommendation — Proposed cards are regenerated by the evaluator from stored observations each pass and carry their evidence as embedded snapshots, so their citations must not pin spam records forever; only acted-on cards constrain the trim. A trim pass also never strands the workspace: one session segment that cannot be rewritten is logged and skipped while the remaining segments are processed.
- `~/.orkworks/workspaces/<hash>/capacity/<id>.json` — capacity per model/harness
- `~/.orkworks/workspaces/<hash>/recommendations/<id>.json` — Taskmaster recommendation state and history, including rollup parents and `rolled_up` member records. A parent stores sorted member IDs/dedupe keys, generation, bounded projected evidence, target surface, and an optional `proposedChange` (null for exact cards, legacy records, and pre-v2 parents); a member stores its current `rolledUpBy` parent ID. An optional `completionPacket` is a versioned, revision-bound evidence projection for one source session and one workspace-local change subject. Packet fingerprints, approvals, idempotency keys, and supersession lineage are validated at the store boundary; replacing a packet clears approval and returns the recommendation to `proposed`, while orphaned target sessions are recoverable without discarding evidence. The sidecar owns validation and writes parent/member transitions as a recoverable graph transaction under the workspace lock. It stages replacements and durable backups in a fsynced manifest, rolls back uncommitted manifests, completes committed manifests during startup/read recovery, and withholds recommendation reads with a diagnostic if neither complete graph can be established.
- `~/.orkworks/workspaces/<hash>/workspace.json` — workspace memory, including the last active session
- `~/.orkworks/workspaces/<hash>/codex-hook-observation.json` — the last Codex hook fingerprint observed executing; Settings reports Codex activation only when it matches the currently installed hook definition
- `~/.orkworks/workspaces/<hash>/integrations/aider.json` — versioned OrkWorks-owned Aider notification-command preference
- `~/.orkworks/harnesses.json` — global harness definitions
- `~/.orkworks/hook-scripts/` — stable copies of harness reporter scripts (e.g. the Claude Code Notification hook), installed hook commands always point here rather than at the packaged/dev source, so they keep working across app updates and packaging schemes whose own paths aren't stable at runtime (Linux AppImage's per-launch mount point, in particular). The workspace-local harness hook configuration that invokes these reporters is gitignored and must not be committed.
- `~/.orkworks/hook-scripts/report-harness-event-diagnostic.json` — the latest redacted Codex reporter trace from the POSIX or Windows PowerShell reporter, atomically replaced per hook event and written with owner-only permissions. It records event/environment-presence flags and POST results, plus a bounded ordered sequence of at most 16 `PreToolUse`/`PermissionRequest`/`PostToolUse` captures containing allowlisted top-level payload key names and only `hook_event_name`, `permission_mode`, `turn_id`, `tool_name`, and bounded `tool_use_id` scalar values. It never stores session IDs, tokens, paths, `tool_input`, arbitrary free text, full payloads, or response bodies. `PreToolUse` and `PostToolUse` are capture-only; attention behavior is unchanged.

Metadata source priority is `user > agent > peon > backend_inference > process > unknown > debug`. Peon reads terminal output and writes inferred metadata; it never types into terminals. Detached runtimes continue draining terminal output, persisting history, and feeding Peon while `orkworksd` stays alive, so losing a renderer terminal attachment alone must not end a session.

`GET /sessions/:id/summary-log` exposes checkpoints in append order as timestamp, summary, source, and nullable confidence; missing data returns `{ "entries": [] }`. The session detail panel renders these as "Task history," distinct from `label` (title), whose `labelSource` provenance separates automatic Peon/Codex titles from explicit user labels (ADR 0063).

Taskmaster consumes normalized metadata and proposes cross-session transitions; v1 requires explicit user approval for every action. The passive `improve_workflow` recommendation needs no approval to display, but cannot focus a terminal, edit a file, or start a session. A user may dismiss it or explicitly accept it to send a generated fix prompt to their active session, scoped to the recommended target surface (ADR 0042, ADR 0048).

## Workflow observations and the current-summary snapshot (partially implemented)

This section documents the authoritative target contract from
[ADR 0042](../adr/0042-workflow-observations-replace-summary-checkpoints.md)
and `specs/orkworks-mvp.md`/`specs/taskmaster.md`; implementation is tracked
by [issue #313](https://github.com/Rambolarsen/orkworks/issues/313) and lands
across Tasks 2–4 of the workflow-observation-feedback-loop plan. The
current-summary snapshot on `SessionMetadata` remains design-only below, while
workflow-observation persistence, Peon recording, and passive Taskmaster
recommendations are implemented.

(design, not yet implemented) `SessionMetadata` gains a first-class
current-summary snapshot: `summary`, `summarySource` (`agent` | `peon`),
`summaryConfidence`, and `summaryObservedAt`, all four updated or cleared
together (see "Current-summary snapshot" in `specs/orkworks-mvp.md`). This
replaces the ADR 0024 durable summary-checkpoint mechanism; no new checkpoint
is appended to `events/<id>.ndjson`, and `GET /sessions/:id/summary-log` and
the desktop's "Task history" section are removed. Old event records
containing the superseded checkpoint fields remain readable.

Implemented separately: `workflow_observations.rs` owns `WorkflowObservation`
records (`id`, `sequence`, `sessionId`, `observedAt`, `kind`, `description`,
optional `problemArea`, `evidence`, `reportedImpact`, `source`, `confidence`, `fingerprint`,
`idempotencyKeyHash`) behind a small interface — `record_observation`,
`workspace_observations`, `delete_session_observations`. The authenticated
explicit-report HTTP adapter (`http/workflow_observation_handlers.rs`) and Peon
inference adapter are live. Records
persist as bounded, session-segmented NDJSON under
`~/.orkworks/workspaces/<hash>/workflow-observations/<session-id>.ndjson`
(newest 1,000 records/2 MiB per session), ordered by a durable
`~/.orkworks/workspaces/<hash>/workflow-observations/sequence` counter;
workspace reconstruction reads the newest 10,000 records across segments.
`DELETE /sessions/:id/forget` and retention delete a session's segment and
every recommendation derived from it.

The workflow-observation and Taskmaster routes this design introduces or repurposes:

- `POST /taskmaster/assess-workflow` — implemented in source. Authenticated manual assessment returns one grounded report or an explicit no-proposal outcome, with stable identity-only admission responses.
- `POST /taskmaster/analyze` — implemented. A user-requested Brain analysis bypasses the background workspace cooldown and installation-wide daily allowance; it returns an active Brain recommendation instead of scheduling another evaluation while one is proposed, accepted, or executing.

- `POST /sessions/:id/workflow-observations` — implemented. Harness-neutral explicit report, authenticated with a per-session, non-persisted `ORKWORKS_REPORT_TOKEN` bearer capability (alongside the existing `ORKWORKS_SESSION_ID`/`ORKWORKS_PORT` env vars, generated from OS randomness at session start/resume and never persisted, logged, or serialized) and an `Idempotency-Key` header; body limited to `kind`/`description`/optional `problemArea`/`evidence`/`reportedImpact`, 8 KiB total, rate-limited to 30/session/60s ahead of the store's own 60/session/minute acceptance cap. Missing `problemArea` preserves v1 identity for compatibility; explicit malformed optional fields are rejected without discarding an otherwise valid legacy report.
- `GET /taskmaster/recommendations` and `GET /taskmaster/recommendations/:id` — implemented; list responses include persisted observation diagnostics. The list contract returns active rollup parents and proposed exact families without an active parent, never a `rolled_up` member. Detail responses retain parent/member relationship fields for audit and handoff; actions against a rolled-up member return the existing invalid-transition response.
- `POST /taskmaster/recommendations/:id/dismiss` — implemented. `improve_workflow` exposes no `refresh` action, since Taskmaster's five-second correlation debounce drives its own reevaluation.
- `POST /taskmaster/recommendations/:id/accept` — implemented (ADR 0048). Takes a caller-supplied `sessionId` (the desktop's current `activeSessionId`, never the backend's best-effort-persisted last-active value) and an optional `prompt` override, and sends a generated fix prompt into that session's live PTY via the accepted-input bookkeeping shared with `request_plan_review`, with its runtime identity and control sender captured during approval. No session is created, resumed, or reconfigured. The recommendation is reserved (`proposed` → `executing`) synchronously before the PTY write, so two concurrent accept requests for the same recommendation can't both write to the terminal; on delivery success `executing` resolves to `accepted` with `targetSessionId` set, on failure it rolls back to `proposed`, and `dismiss` also accepts `executing` as a manual recovery path if a crash ever leaves one stuck there.
- `POST /taskmaster/recommendations/:id/complete` — implemented by the recommendation-aware handoff. It accepts only an optional summary with a bearer `ORKWORKS_REPORT_TOKEN`; the sidecar reverse-resolves the token to the live session, requires that session to equal `targetSessionId`, transitions `accepted` to `completed`, and records the recommendation ID in session history. Packet-backed completion additionally requires the current packet revision, evidence fingerprint, and idempotency key. Repeating the same call is idempotent; callers cannot supply a different session or status.
- `POST /taskmaster/recommendations/:id/completion-packet` — implemented. A bearer-authenticated source session may attach the initial packet or submit a superseding revision for that existing recommendation. The route enforces one matching source session and workspace-local packet validation; it does not create sessions or perform coordinator actions.
- `POST /taskmaster/audit/recommendations` and the type-dispatched accept — implemented. The audit pass classifies every `proposed` exact-family card (`improve_workflow:v1:` dedupe keys only — Brain-created assessment proposals are deliberately outside its scope; `under_eligible` below the two-qualifying-citation floor, `noise` as all-unscoped-Peon citations, `duplicate` against a newer proposed or unextended terminal sibling in the same dedupe family, `stale` beyond the fixed 14-day evidence window) and proposes one `cleanup` card (dedupe key `cleanup:v1`, `requiresApproval`, optional `audit` projection) in place, or returns `null` when everything is healthy. Cleanup accepts require no `sessionId` and execute the audited bulk dismissal — each still-`proposed` entry dismissed with the same watermark transformation `store::dismiss` uses plus an `audit:<criterion>@<card>` reason, changed entries skipped and reported, the card itself completed in the same atomic graph transaction that rolls back whole on failure; `improve_workflow` accepts keep requiring the caller-supplied `sessionId` exactly as above. See "Accept execution and immutability" in `specs/taskmaster.md`.

Taskmaster correlates accepted observations five seconds after the latest
accepted one: a fingerprint cluster of at least two observations, each at
confidence ≥ 0.6 (high-impact observations additionally require confidence
≥ 0.8), produces one `improve_workflow` recommendation embedding immutable
snapshots of its cited observations, deduped as
`improve_workflow:v1:<target-surface>:<observation-fingerprint>` and
resurfaced past a dismissal watermark only on higher impact or two
newly-qualifying observations including a new session. See
`specs/taskmaster.md`'s "Workflow-improvement recommendations" section for
the full eligibility, kind-to-target mapping, and dismissal-watermark rules.

Exact recommendations remain deterministic audit units. After that pass, the
sidecar may run one bounded model rollup over currently proposed exact-family
snapshots: at most 32 families, three representative observations per family,
96 observations, eight source sessions per family, and 128 KiB input. The
model may return at most eight two-to-eight-family clusters with bounded title
and summary text plus one required `proposedChange` (summary, one to three
file targets, and a verification note); the sidecar rejects unknown,
duplicate, overlapping, or cross-target clusters. A parent uses the
stable `rollup:<sha256-hex>` identity over sorted member IDs, while generated
prose and `proposedChange` cannot alter evidence-derived counts or claims or
the identity. `proposedChange` is non-evidence: target paths are validated
against the workspace under the workspace lock, `sensitive` is sidecar-computed,
and the Fix with AI handoff places it inside the untrusted reference block. A
rollup validation failure discards the rollups section as a whole (no partial
rollup) while the rest of the combined response still applies, and the run
status records a rollup-degraded reason. See `specs/taskmaster.md` "Rollup proposed change" and
the ADR 0057 amendment. Exact recommendations remain available when the model
is unavailable.

The recommendation graph is recovered before reads are served. Parent/member
updates are staged and published through the fsynced manifest transaction
described above, so recovery produces either the complete old graph or the
complete new graph. The API exposes `rolled_up`, optional observation
`problemArea`, and `rollupMemberIds`, `rollupMemberDedupeKeys`,
`rollupGeneration`, `rolledUpBy`, and optional `proposedChange`; terminal parent/member history is kept
for audit while only active parents and unparented proposed exact families
appear in the normal actionable list.

## Proposed ordinary-child orchestration

[ADR 0077](../adr/0077-taskmaster-orchestrated-child-sessions.md) and the
[#610 scope alignment](../superpowers/specs/2026-10-04-taskmaster-orchestration-scope-design.md)
describe a proposed replacement for the confined master-runner architecture.
The current coordinator records/store are data-only; no orchestration launch,
run/grant, skill-usage or hierarchy runtime is implemented by these documents.

[ADR 0090](../adr/0090-workspace-hierarchy-sqlite-transactions.md) separately
records the accepted one-workspace SQLite/private-worker DELETE/EXTRA direction
for further hierarchy design and qualification only. Detailed storage contracts
and numerical limits remain proposed; enforcement evidence, producer agreement,
capability qualification and approved executable plans retain their gates.
This acceptance supplies no hierarchy runtime or migration.

Repository-family identity is accepted separately in [ADR 0080](../adr/0080-repository-identity-across-worktrees.md)
and its bounded sidecar registry is tracked by [#810](https://github.com/Rambolarsen/orkworks/issues/810).
That registry supplements the path-only workspace history and lease in ADR
0060; it does not represent a workspace, peer instance, or permission. Each
future run must retain both repository-family and exact source-worktree
bindings. #810 is substrate work only and does not add a route or activate
ordinary-child launch; platform evidence and the separate #610 consumer gates
remain open.

Children would reuse the parent's selected-workspace sidecar, metadata store
and ordinary PTY lifecycle, with approved worktree launch directories. There
are no child sidecars, extra selected workspaces or peer registries. Electron
main retains the existing UI authority, while the verified parent integration
receives a volatile run planning bearer. Only a separate server-held exact-plan
grant permits launch. Research completion revokes its grant and permits the
same parent to synthesize/propose execution; final execution or version-bound
UI research-only/decline-execution finish ends run authority.
All live prior-plan children and unattached reservations count toward the run
cap. Events are bounded hints; adapter proof of same-parent continuation is
required, and a live PTY alone is insufficient.

Allocation intent/reservations are durable before mutation/spawn. Turn readiness
and an explicit version-bound parent result coordinate declared dependencies;
they do not prove quality or user acceptance. Recovery never relaunches a child.
Parent end, workspace changes and sidecar replacement fence volatile authority;
exact UI resume and plan reapproval precede further launches. Children remain
manageable through ordinary controls and edits stay for manual integration.
The initial slice does not automate cleanup. Same-user API/environment replay
and ordinary host access remain limitations; native confinement and
crash-surviving orchestration process ownership are not claimed. ADR 0060's
independent-instance cleanup/replacement proof remains unchanged.

The [baseline handoff](../superpowers/plans/2026-09-26-taskmaster-orchestrated-child-sessions.md)
splits later work into reviewable units. Scope is accepted in PR #747;
component review and version-specific coding-tool permission/delivery/event evidence must pass
before scoped execution plans authorize code.

## Rust sidecar (`crates/orkworksd/src/`)

Single binary. Top-level modules:

- `main.rs` — Axum router, `AppState` / `SessionHandle` / `WorkspaceState` / `PeonState` / `RetentionConfig` struct definitions, `main()`, `health_check()`, `#[cfg(test)] pub(crate) mod test_support` (shared test helpers), and a slim `mod tests` covering route registration and core AppState invariants
- `http/` — HTTP handler submodules (`ErrorResponse` declared in `http/mod.rs`):
  - `harness_handlers.rs` — harness CRUD (`GET/POST /harnesses`, `PUT/DELETE /harnesses/:id`)
  - `integration_handlers.rs` — grouped and legacy integration HTTP adapters: strict mutation-body parsing, typed application calls, compatibility error mapping and serialization
  - `provider_handlers.rs` — provider query handlers (`GET /providers`, `GET /providers/:id/models`, `POST /settings/providers`, `POST /settings/providers/ollama/verify`) plus the Peon verification/apply adapters (`POST /settings/peon/provider/verify`, `POST /settings/peon/test-and-apply`, `GET /settings/peon/applied`). Stale-generation rejections on the verify and test-and-apply routes include the sidecar's current `currentGeneration` in the response body so the desktop transaction can resync and retry instead of staying locked out.
  - `retention_handlers.rs` — retention config handler (`POST /settings/retention`)
  - `session_handlers.rs` — session/workspace HTTP handlers (`POST /workspace`, `GET/POST /sessions`, `DELETE /sessions/:id`, `POST /sessions/:id/resume`, `POST /sessions/:id/harness-session`, etc.) and associated request/response types. `GET /sessions` is a thin blocking-task adapter over `session_projection.rs`. `POST /workspace` acquires the workspace's OS advisory lease before migration or reconciliation; a second sidecar receives `409 Conflict` and cannot mark the owner's sessions orphaned. The owning sidecar reconciles sessions orphaned by a previous daemon run via `metadata::reconcile_orphaned_session`: stale "running"/"creating" sessions are completed to `ended`, and sessions persisted mid-`ending` consume their `pendingTerminalStatus` as the final status so they cannot stay stuck in the ending phase.
  - `workflow_observation_handlers.rs` — the thin, capability-authenticated `POST /sessions/:id/workflow-observations` adapter (ADR 0042): bearer-token auth, the route's own 30-attempts/60s pre-persistence rate limit, `Idempotency-Key` validation, and fixed request-vocabulary enforcement, before mapping onto `workflow_observations::record_observation`. Merged into the router as its own sub-`Router` so `DefaultBodyLimit::max(8 KiB)` scopes to this route only.
- `taskmaster_handlers.rs` — recommendation list/detail/dismiss/accept/complete adapters. `accept` (ADR 0048) sends a generated fix prompt into a caller-specified, already-live session; it starts no session of its own. `complete` accepts only a capability-authenticated optional summary and derives the target session from `ORKWORKS_REPORT_TOKEN`, so agents cannot complete a recommendation on behalf of another session.
- `harness_integration_application.rs` — typed application seam for integration inspection, mutation, workspace listing, unreferenced cleanup and synchronous launch readiness. It coordinates existing adapters, probe/revalidation locks and prompt-authority snapshots without duplicating state. Inspection can demote stale authority; HTTP handlers own transport mapping. See [ADR 0030](../adr/0030-integration-lock-check-await-helper.md).
- `session_application.rs` — typed application seam for workspace opening, session lifecycle commands, attention and plan selection, and delete/forget workflows. It coordinates the existing `AppState` and runtime/metadata modules without owning a second session map; workspace opening retains the `WorkspaceLease` in `WorkspaceState` for the lifetime of the owner; `http/session_handlers.rs` remains responsible for request extraction, authorization, compatibility mapping, and serialization.
- `session_projection.rs` — stateful `GET /sessions` projection. It snapshots live and durable session state, performs capacity/provider write-back and cwd/Git/conflict enrichment, and serializes projection with workspace replacement under `AppState.projection_lock`; the lock order is projection lock, workspace or sessions lock, then provider-manager internal locks. It releases state locks before filesystem, process-cwd, or Git I/O. Its process-local Git-context cache lives in `AppState`, reuses results per resolved worktree root for five seconds from scan start, and retains at most 512 roots. Workspace adoption clears it; expired entries are pruned and a full cache evicts the oldest scan. The dedicated cache lock serializes misses without holding workspace or session locks. Session recommendations and cwd-conflict warnings are derived afresh on each listing. `session_view.rs` remains pure and reusable for field derivation.
- `runtime/` — background-task and PTY submodules:
  - `observed_status.rs` — shared attention field operations and `ProcessTransition` decisions; `SessionApplication` owns application and persistence/live-mirror sequencing for ordinary hook/debug, idle and process-attention changes. Committed input retains its combined freshness/attention critical section. See [ADR 0027](../adr/0027-observed-status-attention-owning-module.md) and [transition observation windows](domain-entities.md#transition-ownership-and-observation-windows).
  - `codex_terminal_status.rs` — bounded OSC 7501 parser for live Codex PTY output. Its best-effort status uses existing transactional attention storage, preserves explicit attention, and supersedes hooks only after activation (ADR 0093).
  - `codex_approval.rs` — pure bounded hook/native correlation reducer; effects carry runtime/identity/turn/input fences and require owner acknowledgement after successful persistence.
  - `codex_approval_application.rs` — private authenticated scalar callback and runtime-local attention ownership; atomic application checks use workspace→sessions lock order, and production clearing remains separately gated for unfenced JSON writers.
  - `codex_hook_report_relay/completion.rs` — dormant POSIX sender-ordered completion queue inside a private runtime relay directory. Publication and acknowledgement share one retained lock and atomic, synced queue state; in-flight/failure state cannot grant clear authority. No production stream is created or adopted after restart.
  - `codex_completion.rs` — dormant bounded hook-only completion tracker; only an exact serial invocation can produce a one-use proof to clear its acknowledged permission wait.
  - `codex_completion_application.rs` — dormant non-native application core; checks live and persisted attention ownership in the canonical storage lane. No production caller or qualified launch exists; activation requires the [approved completion gates](../superpowers/specs/2026-10-23-codex-hook-completion-mitigation-design.md).
  - `codex_native.rs` with `protocol.rs` and `launch.rs` — exact compatibility/argument eligibility, bounded authenticated passive WebSocket reads and owned server/TUI launch/cleanup. The shipping compatibility table remains empty pending live verification.
  - `peon_runtime.rs` — `peon_loop` (continuous Peon observation loop); idle sessions enter an in-memory hold and resume observation only after qualifying user input
  - `retention.rs` — `retention_cleanup_task`, `retention_cleanup_once`
  - `session_runtime.rs` — session-runtime-owned PTY/process startup, bounded PTY/persistence/control backpressure queues (including startup input buffering), output draining, replay state, attachment ownership, child wait/finalization. After direct-child exit, PTY output drains for a bounded grace; Unix polls a cancellable PTY descriptor, and Windows cancels a blocked ConPTY pipe read before releasing the reader. `start_session_runtime` generates a fresh workflow-observation reporting capability before spawning the child (ADR 0042), aborting startup on OS-randomness failure rather than spawning with no or a weak capability; `clear_ended_session_tracking` revokes it.
  - `terminal_http.rs` — `get_terminal_output`, `get_summary_log`, `session_terminal_handler` (WebSocket upgrade / attach entrypoint)
  - `terminal_runtime.rs` — env helpers (`terminal_env_overrides`, `session_env_overrides`, `should_forward_terminal_env`), `TerminalAction` dispatch, `set_session_status`, websocket attach/detach transport that continues observing client closure while a command is backpressured, and the process-local workflow-observation reporting-capability registry (`new_workflow_report_token`, `set_workflow_report_token`, `verify_workflow_report_token`, `workflow_report_session_for_token`, `record_report_attempt`) backing `ORKWORKS_REPORT_TOKEN` (ADR 0042)
- `git.rs` — git2-based context detection (repo root, branch, dirty check and recursive changed-file count including untracked files while excluding ignored files), run against each session's *effective* cwd (see `procfs.rs` and `session_view::resolve_effective_cwds`). A single tree-to-workdir diff with index data produces optional API-only `lineChanges` totals against `HEAD`, including staged/unstaged changes and non-ignored new files, with an empty-tree baseline only for unborn HEAD. Binary files contribute no line totals; failures omit statistics. Session listings reuse this context per discovered worktree root, including different subdirectories, and reuse the bounded five-second projection cache across listings. New-session admission still performs a fresh scan. Session creation scans on a blocking worker before admission, reusing that snapshot for its response and durable Git fields when the workspace path matches. A changed persistence workspace is scanned outside the workspace lock and revalidated before registration; canceling a pending scan admits no session.
- `harness.rs` — neutral command and resume types plus `definition`, `registry`, `store`, `integration`, and `integrations` submodules. The versioned built-in resource and v3 user document resolve once into an immutable `ResolvedHarnessRegistry`; each runtime operation captures one snapshot. The document stores declarative custom definitions, sparse built-in patches, and a sidecar-owned compatibility-profile map. Definitions can be `retired`: they remain resolvable for historical sessions and settings, but are excluded from new-session selection and launch. `definition` owns the strict 256 KiB editable schema, duplicate-key rejection, field-path diagnostics, and compiled-binding exclusion. `integration` supplies canonical workspace confinement, component-by-component no-follow parent creation, Git-local-only checks, optimistic revision-checked configuration transactions, and the cross-platform publication helper; `integrations` is the closed compiled-handler dispatch boundary. Definitions declare launch, resume, model, Peon, capacity, signal, integration, and voice capabilities without adapter code paths. Windows uses target-only `windows-sys`: `ReplaceFileW` for an expected existing target and non-replacing `MoveFileExW` for an expected new target. The final check remains best-effort optimistic concurrency, not a portable CAS guarantee.
- `metadata.rs` — reads/writes session, workspace, and event files under the global metadata root (`~/.orkworks/workspaces/<hash>/`). Raw `events/<id>.terminal` replay is trimmed on append to the newest 1,000 lines and 1 MiB; existing oversized dormant files remain unchanged until append. The PTY's last known `cols`/`rows` is persisted authoritatively at the terminal-status transition and best-effort after a successful live resize when the requested grid differs from the known durable grid (including a successful same-size resize when the grid is not yet durable; writes serialize through `TERMINAL_SIZE_WRITE_LOCK`, and live-resize writes back off during `ending`/`ended`). It is cleared on resume, so dead-session replay renders at the recorded grid instead of fit-to-container; see [ADR 0046](../adr/0046-live-resize-persistence-for-terminal-size-sidecar.md). Accepted Peon inference and attention reports preserve exact summaries as durable NDJSON checkpoints with accepted provenance, omitting only whitespace summaries and exact consecutive duplicates. See [ADR 0024](../adr/0024-bounded-terminal-replay-durable-summary-checkpoints.md). (design, not yet implemented — see issue #313) ADR 0042 plans to replace this with updates to a current-summary snapshot on `SessionMetadata` instead of an appended checkpoint; see "Workflow observations and the current-summary snapshot" below.
- `migration.rs` — one-time migration of legacy `<workspace>/.orkworks/` data into the global store
- `peon.rs` — observer config, ring buffer, in-memory observation state, prompt building, inference parsing/validation, timer-based idle detection (`PEON_IDLE_TIMEOUT`, default 15s), summary normalization (strips "User is/wants/typed" prefixes), and usage-limit detection from terminal output. The source-priority overwrite ladder itself lives in one place: `metadata.rs`'s `source_priority` module (see [ADR 0005](../adr/0005-metadata-source-priority.md) and its 2026-09-03 amendment), which both the Peon merge and the attention merge consult.
- `procfs.rs` — `live_cwds(pids)`: cross-platform (Linux/macOS/Windows) batched probe for running processes' current working directories in one `sysinfo` scan. Backs the pid-probe tier of live session git-context tracking (issue #241, [ADR 0031](../adr/0031-live-session-cwd-via-sysinfo-probe.md)); pids that are gone, denied, or unsupported are simply absent from the result and callers fall back further down the chain. Only actually tracks bare shell sessions — see `resolve_effective_cwds` below for why command-template harness sessions need [ADR 0032](../adr/0032-harness-reported-cwd-via-hook-payload.md)'s harness-reported cwd instead.
- `providers.rs` — provider definitions, applied-settings store, persisted runtime state, fallback runner (`run_inference` skips disabled/capped providers in fallback order), and model listing. `builtin_provider_registry()` contains only ollama (HTTP-based, no harness). Harness-backed provider definitions are projected from the captured resolved registry, so Peon configuration remains with its harness definition rather than being duplicated. `ProcessRunner` starts harness providers through plain `Command::spawn()` with piped stdin/stdout/stderr; it has no Unix fork-time callback, setsid operation, or inherited-file-descriptor sweep. This module still carries the historical `Provider*` names, but today it is modeling the Peon inference tool registry rather than the user-facing coding-tool selector. It exposes `GET /providers` for live runtime state, `GET /providers/:id/models` for available models, and `POST /settings/providers` for durable settings application. The session Peon loop routes through `ProviderManager::run_inference`. Per-provider peon model is configured in Settings.
- `session_types.rs` — `SessionInfo` struct, lifecycle and attention enums, and the renderer-facing session contract
- `session_view.rs` — lifecycle-aware session projection, connectivity and terminal-outcome derivation, conflict detection, and resume-option derivation. `resolve_effective_cwds` centralizes the explicit task-worktree binding → harness-reported → pid-probed → launch-cwd fallback chain (ADR 0032 → ADR 0031 → launch `cwd`) so git-context enrichment and cwd-collision conflict warnings agree on where a session actually is.
- `workflow_observations.rs` — durable, bounded `WorkflowObservation` recording (ADR 0042): validation, idempotency (15-minute replay window via tombstones), sequencing, per-session (1,000 records/2 MiB) and per-workspace (10,000 records) bounds, and a live 60-accepted/session/minute rate cap. Public surface: `open`, `record_observation`, `workspace_observations`, `delete_session_observations`, `diagnostics`. Callers never see file paths or on-disk formats.
- `taskmaster/` — canonical passive recommendation contract, deterministic workflow-improvement evaluator, five-second generation-debounced refresh, and atomic recommendation persistence with dismissal watermarks and orphan/session cleanup.
- `workspace_runtime.rs` — `iso_now`, `orkworks_global_dir` (workspace path hashing to global store location)

For the current Rust domain model itself, see [domain-entities.md](domain-entities.md).

Live terminal attachment replays the newest 256 PTY output chunks. As older
chunks leave that window, `runtime/terminal_replay_modes.rs` retains a bounded
checkpoint of screen-buffer and interaction modes. The first replay chunk
restores that checkpoint before its original bytes, preserving alternate-screen
wheel routing, mouse protocol/encoding, keyboard, focus and bracketed-paste
modes after renderer reload. Partial control sequences retain parser context;
retained mode changes and resets still execute in their original order. This
does not reconstruct evicted screen contents or scrollback, and it does not
persist PTYs or checkpoints across a sidecar restart.

Delayed terminal-status transitions preserve durable `ended` metadata even
while the same live runtime remains `ending`: finalization writes metadata
before updating the live handle, so that interval must not reopen the record.
Read-side lifecycle reconciliation remains necessary, while resume admission
rejects an ending handle. Idle candidates carry their selected runtime generation;
the application rechecks live and persisted lifecycle plus live authority under
its existing locks, so a stale sweep cannot mark a terminal or replacement
runtime idle. Failed finalization writes preserve `ending` and can be retried
through the same operation once storage recovers; no automatic retry is implied.

PTY input runs as one sequential blocking-pool write at a time, while the
session driver continues draining output and observing the independent stop
channel. Input and resize remain ordered in the bounded queue. A command
rejected during runtime exit switches the attachment to output-only draining: queued and
new commands are discarded while trailing output and the runtime's final
`Ended` or `Error` event continue to reach the renderer. Stopping rejects
queued input and signals in-flight cancellation; the native writer retains its
acknowledgement until it actually returns, because async cancellation cannot
abort a blocking OS write. Taskmaster delivery finalization retains its
reservation even if the HTTP request is canceled and checks the original
workspace path and advisory lease before updating its store. Same-workspace
reopens retain that lease; a genuinely replaced lease rejects late finalization.
Approval captures the target runtime sender and generation, and synchronous
input bookkeeping shares the projection gate with resume admission, so an old
approval cannot send input or apply live effects to a replacement runtime. See [ADR 0075](../adr/0075-responsive-pty-input-delivery.md).

The shared provider process runner owns a Windows Job object per invocation
(`providers/windows_process.rs`). It assigns the suspended child before resuming
execution, preventing descendants from escaping timeout cleanup. Termination
and pipe joins have bounded waits; external `taskkill` is unnecessary. Unix
continues using an owned process group. See the 2026-09-12 amendment to
[ADR 0055](../adr/0055-json-taskmaster-inference-adapters.md); native Windows
desktop verification remains tracked by #525.

## Fixed desktop shell

`ApplicationShell` owns fixed React/CSS Grid regions: Sessions, one central
Terminal, Review or temporary utility page, and at most one inspector.
Recommendations uses the central utility page at every width, without an
inspector or pin control; Back restores the underlying view and its invoker.
It remains transient presentation state under [ADR 0084](../adr/0084-recommendations-main-view.md). Dockview is
removed under [ADR 0082](../adr/0082-react-grid-desktop-shell.md), which supersedes
the library choice in [ADR 0078](../adr/0078-fixed-desktop-shell-and-central-navigation.md).
Pointer and keyboard separators resize bounded regions. At medium widths a
utility replaces the central content; compact widths show one page at a time.
Navigation preserves selected-session identity and restores focus to the invoker
when a temporary page closes. Review opens from the selected session's Details,
the content header or the View menu, with an explicit return to that session's
Terminal/history. Return restores an available document invoker, otherwise
terminal focus. Review and Details share one in-flight independent-review request
per session and workspace generation. Validated document bytes remain in renderer
memory with a content digest and workspace/session binding; refresh replaces that snapshot and
clears stale content on failure. Opening, refreshing, closing and restoring Review
never submit a prompt; only Request independent review does. Saved Review
presentation is revalidated through Electron before restoration. Refresh/Retry
and no-op Terminal commands leave navigation recency unchanged. Unreadable
navigation memory surfaces a diagnostic and the existing Electron-confirmed rebuild path; cancelled recovery
preserves the record. Gated Workflow navigation remains a separate delivery slice.

Actions (#805) uses the same inspector or temporary page. It groups canonical
live session attention before the existing ordered workspace recommendations,
with source labels and separate required-session-attention and recommendation
counts in the central toolbar even when Sessions and the inspector are hidden.
Remembered attention stays historical; usage-limit status remains on session
list and detail surfaces. The unfinished Capacity destination is removed; the
standalone dashboard is deferred in the MVP specification.
The current API has no live Done/Stale values, so Actions does not infer them
from ended history or inactivity. Session polling reports availability, and a
workspace/generation-fenced recommendation poll labels partial, loading and
unavailable data instead of inventing zero counts. Show next action focuses the
first current row without selection, unread acknowledgement or execution.
Inspection survives responsive host changes; changed/removed subjects lose
current details and show an unavailable explanation. Open terminal/history and
Open recommendation are separately labeled explicit navigation commands.

`TerminalPanel` hosts the active live PTY through `CenterPanel` and xterm.js.
The renderer terminal registry keeps that runtime and its WebSocket draining
when responsive navigation detaches the terminal DOM. Returning reattaches the
same runtime. Sidecar PTY lifetime remains independent of renderer attachment.
Inactive sessions need no renderer attachment to keep their PTYs running.

Electron owns bounded installation shell preferences and canonical-workspace
navigation memory. The renderer hydrates preferences without writing defaults,
debounces resize saves, and serializes reset before subsequent saves. Legacy
`layout.json` remains untouched and is not restored by the new shell. The native
View menu, sidebar navigation and context header route through the same shell commands.

The fixed sidebar places workspace switch, New session and Recommendations
above independently scrolling session history, with Settings and
connection status anchored below. A compact header above the central content
shows the selected session, Details, and a prominent Resume session action for
an ended session with an available resume option while connected. Resume is a
split button: the main action selects the preferred available strategy and the
arrow opens explicit choices. The shared header/Details chooser confirms every
non-exact choice in a modal warning before dispatch. Cancel is initially focused.
The selected strategy travels through the controller to `POST /sessions/:id/resume`
as an optional `{ strategy }` body. Bodyless legacy callers retain default
selection; supplied choices are revalidated and never silently substituted.
Only exact resume may carry the prior native ID into command interpolation or
seed a new prompt-authority generation. Non-exact resume uses the existing
fresh-launch first-binding behavior and clears stale native-ID provenance;
pre-spawn cancellation restores the original metadata snapshot, including
native-ID provenance. Codex remains exact-only. When the
sidebar is collapsed at medium and wide widths, an icon-only navigation rail
retains New session, Recommendations and Settings, and an expand
button restores its saved width using the existing visibility preference.
Compact widths omit the rail and retain the temporary Sessions page. The
central Workspace menu keeps workspace switching and the full sidebar controls
reachable whenever session history is hidden.
The titlebar retains the product name
and native window dragging/controls; active-workspace history warnings remain
visible there even when the sidebar is hidden. The no-workspace picker displays
connection progress and recovery diagnostics above the shell. At short logical
heights, the sidebar itself scrolls so workspace actions and Settings stay
reachable alongside the independently scrolling history list.
On Windows, Electron uses a hidden title bar with native window controls overlaid
on the 38px app header; CSS reserves the controls area. The application menu is
auto-hidden and remains accessible with Alt. macOS and Linux retain their chrome.

Session details retain read-only coding-tool, provider, model, and provider-state
fields, with session IDs and Peon diagnostics behind debug settings. Session
sorting and attention routing remain lifecycle-aware: only alive sessions receive
live attention, while dead sessions remain historical context.

PTY handles text I/O; native harness voice bypasses PTY entirely. Source behavior
here does not establish availability in published installers. See the
[accepted shell design](../superpowers/specs/2026-10-05-application-shell-navigation-design.md)
for the complete target and remaining accessibility/platform evidence gates.

## Update triggers

Update this file when:

- A new module is added to or removed from `crates/orkworksd/src/`
- `electron/preload.ts` exposes new or removed `window.orkworks` methods
- `apps/desktop/src/api.ts` adds or removes endpoints
- Port-discovery mechanism changes in `electron/main.ts`
- Panel layout changes (new panels, library swap)
- A major npm or Cargo dependency is added or removed

## Session organization

The accepted [archive design](../superpowers/specs/2026-10-18-session-archive.md)
and [ADR 0088](../adr/0088-session-archive-state.md) add Active, Closed and Archived
as organization states, independently of runtime status and attention. The
sidecar owns bounded identity-bound `session-state/<id>.json` records. Missing
legacy records derive Active or Closed; unreadable state disables mutations and
protects retention. Active Archive and Delete close the exact captured runtime
and await cleanup plus persisted ending first. Restore returns to Closed.
Archived history is retained outside automatic age/count cleanup. Archive and
Restore preserve activity timestamps and native tool identity. This is OrkWorks
organization only; native archive synchronization is outside this increment.

Archive publication syncs the staged file and, on Unix, the metadata parent and
state directory entries before success. Windows publishes through a same-directory
write-through replacement. A durability barrier failure is returned to the caller;
it cannot be reported as a successful archive decision.

## Repository agent-readiness implementation

Owner-approved [readiness scoring](../../specs/repository-agent-readiness.md) is
implemented separately from the one-improvement workflow assessment.
`taskmaster/readiness.rs` owns the manual two-call scheduler and input fences;
`readiness_model.rs` owns strict scoring/discovery decoding;
`readiness_rubric.rs` validates the complete signed fourteen-concept descriptor;
`readiness_store.rs` owns capped atomic snapshots, attempt recovery and redaction.
The shared safe collector has an explicit readiness purpose/version while
legacy workflow manifests retain their existing representation.
Assessment failures retain fixed provider/validation categories without raw model
output or provider diagnostics. Readiness adds the failed stage to its existing
error code; Recommendations renders only known local descriptions and keeps
legacy generic failures readable. Changed-input attempts are interrupted, and
failed reruns preserve previously completed snapshots.

Authenticated empty-body `POST /taskmaster/assess-readiness` admits a manual run;
`GET /taskmaster/readiness` projects only the selected workspace's local history
and fixed progress. The existing inference transport/trust, shared analysis
lease and context restrictions apply; no recommendations or repository commands
are generated. Sidecar-computed totals require all fourteen validated ratings.
The renderer compares compatible saved snapshots and never treats a partial
assessment as an overall number. Settings narrowing removes affected historical
scores and evidence, including unopened workspaces, through the existing policy
reconciliation owner.

Runtime eligibility still requires the complete reviewed signed format-2 Brain
rubric, generated starter proof and selected-model quality evidence. These
dependencies remain pending under [#1063](https://github.com/Rambolarsen/orkworks/issues/1063)
and [Brain PR #127](https://github.com/Rambolarsen/brain/pull/127); source wiring
does not establish released-installer availability. See
[ADR 0095](../adr/0095-repository-agent-readiness-history.md).


### Combined readiness and workflow suggestion

[ADR 0096](../adr/0096-readiness-workflow-suggestions.md) adds authenticated,
empty-body `POST /taskmaster/assess-and-suggest`. The readiness owner keeps the
installation-wide analysis lease through scoring and a fresh workflow assessment;
`assessment::run_fresh_under_lease` reuses existing current-input checks and
recommendation publication. Standalone scoring remains independent. The combined
ceiling is four calls, with no retries or wider per-stage evidence allowance.

`taskmaster/readiness_suggestions.rs` owns an atomic, source-free 8-KiB
`readiness-suggestion-run.json` with exact child and recommendation IDs. Existing
readiness history and workflow report formats are unchanged. Recovery reconciles
children first, then the parent, without replaying inference. A failed suggestion
preserves its committed score. Readiness status projects the exact linked
no-proposal report through existing current-input/access checks; standalone scores
do not inherit prior combined results. Fix with AI retains its explicit approval.

Collector version 3 prioritizes current documentation in bounded traversal and
catalog selection before large generic skill trees. Version-2 manifests remain
valid for historical display, but cannot restore as current version-3 inputs.
This changes evidence selection, not privacy permissions or collector bounds.
