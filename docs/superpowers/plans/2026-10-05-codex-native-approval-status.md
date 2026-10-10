---
type: "Implementation Plan"
title: "Codex native approval status Implementation Plan"
description: "Implementation plan: Codex native approval status Implementation Plan."
tags: ["orkworks", "plans"]
---

# Codex native approval status Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve attributable Codex approval waits during tool execution without answering approvals or clearing another producer's attention.

**Architecture:** The session runtime owns a private authenticated app-server, the native remote TUI, and a passive observer. A pure reducer correlates validated hooks and bounded native observations; the existing attention owner applies effects under generation and ownership fences. Exact verified compatibility entries gate the native launch before the unchanged direct-launch isolation path.

**Tech Stack:** Rust, Tokio, portable-pty, tokio-tungstenite, serde, existing SHA-256/getrandom and platform process APIs.

**Spec:** `docs/superpowers/specs/2026-10-03-codex-native-approval-status-design.md`; tracked by #690.

## Global Constraints

- The user approved the written design and experimental dependency on 2026-10-05 with “lets go ahead”. This approves implementation; production verification remains required.
- Fixed two-second monotonic grace; duplicate PermissionRequest does not renew a deadline.
- Poll at 100 ms with one in-flight observation cycle, two-second RPC timeout, 1 MiB message limit, and at most 64 loaded IDs. Reject a cycle older than 300 ms from its first request through effect commit.
- Only initialize/initialized, thread/loaded/list, and thread/read(includeTurns=false). Never answer server requests or start/resume/submit turns through the observer.
- Native eligibility runs on the original CommandSpec. Canonical empty new launch, exact resume, ordered -c/--config and --enable/--disable are the only mapped arguments. All other supplied arguments and custom wrappers use unchanged direct isolation before spawning.
- Probe the actual executable's exact version, three-second deadline, 64 KiB per stream; no minimum-version inference. No shipping compatibility entry from spike evidence.
- Fresh 256-bit memory-only capability per attempt, SHA-256 digest only in server args, plaintext only in observer header and dedicated TUI environment. No reporting capabilities in launch probes.
- Runtime-owned children only; authenticated readiness within five seconds, at most three port attempts, two-second graceful owned cleanup. No second-conversation fallback after a child starts.
- Exact accepted authenticated root identity, fresh nonpending baseline before candidate, newly attributable pending edge afterward. Pre-existing pending, reconnect, overlap, additional loaded threads and malformed/partial reads cannot grant clear authority.
- Every accepted attention/source write revokes opaque runtime-local ownership, including identical competing writes. Set/clear and ownership checks share the session lock. Overflow invalidates ownership. Native clear stays disabled if any writer is unfenced.
- Detached renderer preserves runtime; process exit, reset, input, authority loss and generation replacement fence old effects.
- No public HTTP/renderer schema expansion or persisted native status. Hook correlation transports only allowlisted bounded scalars; both reporter platforms need tests.
- All shell commands start with rtk; Node commands use pnpm. Only this worktree/branch is writable. No unrelated sessions or worktrees are changed.

## Evidence and uncertainty checkpoint

Least confidence: actual root/child identity and slow auto-review semantics. Exact 0.160.0 source shows Guardian runs before user-facing requests and does not increment the pending-permission flag; internal Guardian threads are excluded from loaded-thread enumeration. The exact-tag and installed generated schemas match, including optional parentThreadId. Source proves root id equals hook session_id and sessionId; descendants inherit sessionId but have distinct id and parentThreadId. Root source serializes cli/mcp. Independently correlated slow-auto >2 seconds and actual root/subagent prompt isolation remain live gates, not inferred permissions.

Project blind spot: value equality does not establish attention ownership. Existing writers include Peon projection, direct lifecycle/input updates and hook activation. Task 3 must inventory and fence all of them; it may conservatively revoke on extra writes but cannot omit a writer. Existing source includes this requirement already, so no scope extension is needed.

Native auth preflight on installed 0.160.0/macOS arm64 rejected missing/wrong tokens with 401, initialized with the correct token, returned an empty complete loaded list, and stopped its owned server. It started no conversation and does not qualify a shipping entry. Baseline: 112 Codex tests passed, one ignored.

## Routing and review

Sequential tasks: reducer → protocol/launch → backend runtime integration → private hook transport → production evidence. Interfaces, not file boundaries, determine dependencies. One implementation worker at a time; controller owns plan/ledger. Maximum three concurrently live agents including controller; no nested delegation. Fresh task reviewer checks spec and code quality against a packaged diff; final explicit /code-review medium checks the complete diff. Independent read-only source review can run alongside implementation. One review round followed by controller reconciliation before further work. Use the least costly tier adequate for subtle correlation/concurrency work; mid-tier high reasoning is warranted for implementation from prose and review of lifecycle fences.

### Task 1: Pure approval reducer

**Files:** Create `crates/orkworksd/src/runtime/codex_approval.rs`; modify `crates/orkworksd/src/runtime/mod.rs`; tests embedded in the new module.

**Interfaces:** Produces `ApprovalReducer`, `HookEvent::{PreToolUse,PermissionRequest,PostToolUse,Stop,UserPromptSubmit}`, `NativeStatus::{Active,ApprovalPending,UserInputPending,Unavailable}`, `Effect::{ShowWait,ClearWait}`, and `Fence { runtime_generation, identity_revision, turn_revision, input_revision }`. API `new(Fence)`, `hook(HookEvent, Option<&str>, Instant) -> Vec<Effect>`, `observe(NativeStatus, bool, Instant, Instant) -> Vec<Effect>`, `tick(Instant) -> Vec<Effect>`, `invalidate(Fence)`, `disconnected()`, `accepts(&Effect) -> bool`, `assistance_available() -> bool`. Each effect carries an opaque candidate identity and fence; attention ownership is assigned by Task 3, not fabricated by this module. `observe`'s bool denotes independently proven complete singleton root; false locks the batch conservatively. The last two observe arguments are first-RPC time and current receipt time, so freshness is deterministic; no internal clock reads. Observations must also be fresh at commit.

- [x] Write behavioral tests before implementation and run `rtk proxy cargo test --manifest-path crates/orkworksd/Cargo.toml --bin orkworksd codex_approval`. Initial stub methods emit no effects, so deadline and resolution tests fail.

```rust
let now = Instant::now();
let mut reducer = ApprovalReducer::new(Fence::default());
reducer.observe(NativeStatus::Active, true, now, now);
reducer.hook(HookEvent::PreToolUse, Some("tool-1"), now);
reducer.hook(HookEvent::PermissionRequest, None, now);
assert!(reducer.tick(now + Duration::from_millis(1999)).is_empty());
assert!(matches!(reducer.observe(NativeStatus::ApprovalPending, true, now + Duration::from_secs(2), now + Duration::from_secs(2)).as_slice(), [Effect::ShowWait { .. }]));
```

- [x] Implement bounded serial correlation with 64 records/tombstones per turn, fixed receipt deadline, pending-edge attribution, fresh baseline, permanent batch lock until trusted boundary, identified late-duplicate suppression and generation/turn/input invalidation. Anonymous reports cannot inherit a retired invocation identity; they remain conservative. Candidate identity exhaustion permanently disables assistance and causes application fallback, not indefinite suppression. Fresh active with no flags keeps an unconfirmed candidate hidden after deadline. Unavailable/stale status conservatively shows it. Exact eligible completion retires its candidate; a locked older pending batch retains the wait.

```rust
const GRACE: Duration = Duration::from_secs(2);
const MAX_RECORDS: usize = 64;
const FRESHNESS: Duration = Duration::from_millis(300);
// A clear requires an eligible serial chain AND an attributable edge;
// exact PostToolUse can retire only an eligible serial invocation.
```

- [x] Test deadline/duplicate/late flag; slow active tool; pre-existing pending plus new completion; reconnect losing edge; question/idle/error; absent IDs, overlapping pre/permission chains, reordered completions; 64-bound overflow; stale observations; input/reset/Stop invalidation. Re-run the same command, inspect results, commit only owned module changes.

### Task 2: Bounded native client and launch ownership

**Files:** Create `crates/orkworksd/src/runtime/codex_native.rs` and focused child `codex_native/protocol.rs`, `codex_native/launch.rs`; modify `crates/orkworksd/src/harness/probe_cache.rs` for an opaque generation epoch, runtime module registration and `crates/orkworksd/Cargo.toml` to move existing futures-util/tokio-tungstenite dependencies to production use. Tests live next to implementation. Preserve unrelated provider transports.

**Interfaces:** Consumes Task 1 NativeStatus. Produces `eligible(&CommandSpec, &VersionProbeCache) -> Result<Option<NativeLaunchPlan>, String>`, `async NativeLaunchPlan::start(&[(String,String)]) -> Result<OwnedNativeRuntime,String>`, and owned runtime methods `tui_args() -> &[String]`, `tui_environment() -> (&str,&str)`, `resolved_executable() -> &Path`, `revalidate_tui_executable() -> Result<(),String>`, `async observe(&str) -> Result<NativeObservation, NativeError>`, `is_alive() -> bool`, `async shutdown()`. `NativeObservation { status: NativeStatus, complete_singleton_root: bool, started_at: Instant }`. NativeLaunchPlan contains resolved executable, original cwd/config args, exact resume route and VersionProbeEpoch. VersionProbeCache::epoch() returns an opaque VersionProbeEpoch with is_current(); its shared generation is read-only outside the cache. Check the captured epoch before/after awaited probes and before both child spawns. Revalidate the resolved executable immediately before the second PTY child; identity change after a child starts aborts with owned cleanup, never direct fallback. Secret types have no Debug or serialization implementation. Exact reviewed compatibility record contains version, platform, protocol/root proof and linked production evidence; keep the shipping table empty until Task 5 gates pass.

- [x] Write failing argument/eligibility tests: empty new and exact resume keep option order; all unmapped options fall back before children; selected metadata model does not synthesize args; unverified version/platform cannot opt in. Run `rtk proxy cargo test --manifest-path crates/orkworksd/Cargo.toml --bin orkworksd codex_native`.
- [x] Implement explicit complete allowlist parsing and bounded probes on resolved executable. Use the existing VersionProbeCache with a distinct codex-native namespace and executable identity in its key, so permissive integration-probe outputs cannot seed native eligibility; registry generation invalidation and before/after executable identity validation still apply. Native eligibility precedes direct --no-daemon augmentation. Include executable identity revalidation against the startup generation before either child.

```rust
// Empty verified table is intentional until production evidence is complete.
const VERIFIED: &[CompatibilityRecord] = &[];
```

- [x] Write protocol fixture tests for unknown requests, unexpected IDs/replies, message limits, two-second timeout, loaded-list continuation/missing cursor/overflow, exact root mismatch and list changes. Authenticate with Bearer header marked sensitive; a sentinel test must prove request/header Debug does not expose plaintext; require initialized then two complete singleton checks around exact-thread read. Treat unsupported server requests as disconnect, never send a reply. Drain only bounded notifications.

```json
{"method":"thread/loaded/list","params":{"limit":64,"cursor":null}}
{"method":"thread/read","params":{"threadId":"accepted-root","includeTurns":false}}
```

- [x] Implement OS loopback port selection with at most three bounded attempts, fresh secret, owned server spawned with exact execution environment excluding native secret, readiness and child-liveness checks. TUI receives remote endpoint/secret env and exact resume plus ordered shared config. Unix process group cleanup is scoped; Windows uses suspended Job assignment before resume or remains explicitly unsupported with direct fallback until implemented and verified.
- [x] Test wrong/missing auth, competing port, startup failure/cancellation, descendant cleanup, two owners with distinct secret/env/cwd/config. Run module tests and formatter, inspect changes, commit only owned files.

### Task 3: Backend ownership and runtime integration

**Files:** Modify `runtime/session_runtime.rs`, `runtime/terminal_runtime.rs`, `runtime/observed_status.rs`, `session_application.rs`, `metadata.rs`, `http/session_handlers.rs` only if a backend helper needs it. Follow current adapter routing; do not edit renderer schemas. Modify `runtime/codex_approval.rs` for synchronous owner acknowledgements and add focused `runtime/codex_approval_application.rs` if needed to keep state/effect application isolated. A narrow cfg(test)-only synthetic plan factory/re-export and fake TUI extension in runtime/codex_native.rs and codex_native/launch.rs are permitted for actual native PTY lifecycle fixtures; do not change production eligibility or add an environment/record opt-in. The fake TUI branch must precede server environment capture and store only native capability presence, never its value. Narrow launch.rs production ownership is added for explicit awaited server cleanup on failed readiness/revalidation before returning or retrying; Drop stays last-resort cancellation cleanup and errors never start direct conversation fallback. Narrow protocol.rs/record-fixture ownership is added for pinned non-originating initialize: ordinary custom client names mutate upstream global identity and implicit login. Use the exact reviewed non-originating initialization contract, reject unmapped protocols, and add no capabilities/config overrides.

**Interfaces:** Consumes Tasks 1–2. Produces runtime-local tracker state and opaque attention-write ownership returned by atomic reducer set. The runtime owns native server/observer until PTY driver exits; an application tick reads accepted root + generation and applies effect only under workspace→sessions locks with current identity, hook/turn/input revision, candidate, whole tuple/source, freshness and ownership checks. Token revisions use checked addition; exhausted revision never yields another valid token. MetadataStore is a by-value workspace field: audit all production constructors and writers before relying on a per-store revision. The additional stores in session_projection are read-only snapshots; preserve that boundary. The MVP explicitly supports direct session-JSON writers, which bypass a per-store mutex. Production native clear must remain disabled behind a distinct unfenced-writer gate until an approved producer contract resolves this; fixture-only cooperating-writer tests may exercise the conditional clear path. File identity detects completed replacement but does not prove atomic CAS against arbitrary direct writers. Do not add a protocol lock requirement, demote direct JSON, or change its accepted authority in this task. Capture the hook revision before each awaited RPC separately from the Fence so intervening correlation events discard old responses. Extend runtime module registration if a focused application module is added. Add reducer `acknowledge_show(&Effect) -> bool` and `acknowledge_clear(&Effect) -> bool`: issuance alone does not mark shown or retire a visible wait. Only a successful owner mutation under the same lock acknowledges. PersistFailed leaves the original candidate/deadline and resolution proof retryable; each retry requires all original fences, actual current ownership and fresh eligibility. New hook/input/question/pending ambiguity/reconnect/stale status/generation cancels pending clear proof; it cannot be reconstructed from disconnected snapshots. Source/ownership rejection abandons the old effect through explicit invalidation without mutating the competing tuple.

- [x] Inventory every production attention/source writer; record exact paths in the implementation report. Write failing tests for competing identical-value write, user/source priority race, reset/new runtime, delayed hook/RPC and input clear. Tests must prove old ownership cannot clear the new tuple. Add failing persistence regressions: failed ShowWait followed by still-pending observation retries the same candidate without renewing deadline; failed ClearWait followed by fresh eligible Active retries, but question/new hook/input/reconnect/identical competing writer cancels clear. Modify existing reducer test helpers to acknowledge successful simulated owner writes so their visible-wait assertions remain meaningful.

```rust
let first = owner.accepted_write();
owner.accepted_write(); // identical values from a competing producer
assert!(!owner.owns(first));
```

- [x] Centralize ownership revocation at accepted attention/source mutation boundaries; fence every live/durable writer. Conservative extra revocation is allowed. Persisted records do not serialize tokens.
- [x] Preserve existing activation-only whole-tuple clear and user protection. Produce a narrow private report_hook(state,id,report_token,generation,first_receipt,NativeHookReport) callback with closed Accepted/Retry/Rejected outcomes. Validate capability, alive Codex generation, exact previously accepted authenticated root, fingerprint, scalar bounds and ordering. NativeHookReport carries root_id, optional turn/tool IDs, closed HookEvent, observed_at UTC and fingerprint only. Activation persistence Retry does not renew original first-receipt deadline; accepted completion retains reducer proof until clear acknowledgement. Document exact callback signatures, fields/Serde names, bounds and outcomes for Task4. No token goes into its envelope. Temporary root-unbound or activation persistence failures return Retry without granting native authority; invalid auth/identity/fingerprint is Rejected. Transport wiring remains Task4; do not silently grant authority based on caller assertions.
- [x] Route eligible original commands to native preparation; construct session credentials/mailbox once before either child. Keep direct isolate_session unchanged for ineligible routes. Owned-server failure ends native PTY; PTY exit stops observer then server exactly once. Native RPCs run in a separate bounded owner/observer task, not awaited inside a PTY-driver branch; native loss must not stall input/output. The driver holds a stop/cleanup-ack control handle and awaits bounded teardown before report-capability revocation. Cancellation checks surround every asynchronous startup step. Renderer detach is unrelated to ownership.
- [x] Run focused fake fixtures for runtime startup/end in either order, detached attachment, workspace shutdown and exact resume. Private reporter/relay and actual Windows test wiring belongs to Task4. Actual effective-config parity belongs to the Task5 live gate; no shipping entry exists yet. Run full sidecar formatter/build/tests/clippy, record actual output, commit changes.

### Task 4: Authenticated hook relay and reporters

**Files:** Modify runtime/codex_hook_report_relay.rs, harness/integrations/mod.rs (reporter tests), crates/orkworksd/scripts/report-harness-event.sh, crates/orkworksd/scripts/report-harness-event.ps1, crates/orkworksd/scripts/report-harness-event.test.sh and .github/workflows/pr-ci.yml (Windows reporter discover/run). Modify harness/integrations/codex.rs only if generated event arguments require changes. Narrow callback adaptation in codex_approval_application.rs and terminal_runtime.rs environment filtering for the new native marker are permitted after Task3 ownership ends; preserve its atomicity/authentication contract. No public schema changes.

**Interfaces:** Consume the exact NativeHookReport/report_hook/HookReportOutcome API documented in Task3 report. Separate closed scalar `approval` envelope from unchanged strict identity `report` envelope. Relay owns bounded original first-receipt bookkeeping across Retry; retry records must not starve identity envelopes or unvisited records in a bounded scan. Backend independently validates token/generation/root/fingerprint; relay never supplies asserted trust. UUID mailbox filename order is not chronological. Capture one microsecond-precision UTC observedAt at native hook receipt (including capture-only events); reuse it throughout enqueue attempts. Equal timestamps remain ambiguous. Only allowlisted bounded event/turn/tool/root/timestamp/fingerprint scalars, never raw input or reporting capabilities.

- [x] Write failing reporter/relay fixtures proving scalar allowlist, ordering ambiguity, capability/identity/fingerprint rejection, activation persistence retry with unchanged first receipt, envelope size/scan bounds and no secret/free-text capture.
- [x] Prevent ambient ORKWORKS_CODEX_NATIVE_APPROVAL inheritance into direct sessions; only the owned native execution path injects it. Add a focused environment-filter/direct fallback regression. Only route PermissionRequest privately when verified owned runtime marker is present AND atomic enqueue succeeds; otherwise preserve existing immediate attention HTTP fallback. Direct Pre/Post remain capture-only. Native Pre/Post supply correlation only. Stop/UserPromptSubmit keep existing trusted HTTP transition and fence old private reports; no fabricated working signal or extra authority.
- [x] Validate or conservatively invalidate multiple/out-of-order/tied private events; missing/ambiguous IDs cannot grant clear authority. Retry retains unchanged record identity and bounded first receipt; consumed/rejected records follow existing safe mailbox removal. Do not weaken identity envelope validation.
- [x] Extend existing cfg(windows) PowerShell subprocess fixtures for actual native scalar success/fallback/secret exclusion, and discover/run the exact fixture in Windows PR CI. Static assertions do not establish execution; local pwsh is unavailable and must be reported honestly.
- [x] Run focused POSIX/relay/Rust tests, required full sidecar checks after changes, OpenCode reporter regression and Windows CI when available; commit owned files. Keep VERIFIED empty and independent unfenced-writer native-clear gate disabled. No real Codex/model conversation.

### Task 5: Production gates, documentation and PR

**Files:** A cfg(test)-only ignored installed diagnostic may use the private injection in runtime/session_runtime.rs and a test-only explicit installed-plan constructor in codex_native/launch.rs; any helper stays in a dedicated test module. Narrow cfg(test)-only redacted observation/effect metrics in codex_approval_application.rs/codex_approval.rs are permitted if existing state cannot establish the operator checks; keep them per-fixture and expose no identifiers or secret-bearing Debug. A cfg(test)-only OwnedNativeRuntime::disconnect_observer in codex_native.rs and per-runtime watch/control path in session_runtime.rs may drop only the passive client and pause its observations while retaining native server/TUI liveness, normal disconnection revocation and stop/cleanup. The diagnostic reserves Ctrl-\ to toggle that pause; resume uses ordinary conservative reconnect semantics. No additional protocol method or production pause field. A per-fixture cfg(test) final-drain acknowledgment after persistence/trim may supplement Ended and credential absence, because Ended precedes final draining; fake HOME and owned directories must outlive callbacks, including failed-cleanup retention. The installed diagnostic record pins version 0.160.0 on macOS/aarch64 and initialize userAgent prefix `codex_cli_rs/0.160.0`, verified by the read-only non-originating preflight; existing prefix-or-prefix-plus-space policy remains unchanged. No production environment bypass or shipping table change. Controller owns docs: update ADR 0076 status/approval record, ADR index and design implementation status; add version-specific verification evidence under `docs/superpowers/verification/2026-10-05-codex-native-approval-status.md`; populate native compatibility table only for a fully verified version/platform.

- [x] Record the approved architecture before production implementation; retain explicit unverified rollout gates and unchanged unsupported fallback.
- [ ] Execute authenticated installed-binary checks and actual effective-config comparisons. Require fresh manual >2s approval, one approval click, working before long-tool completion and observer-disconnect prompt usability. Require independently correlated automatic Pre/Permission/Post with no click and review >2s. Require root/subagent/overlap evidence excluding independent prompt clears. Native diagnostic tests do not launch another ordinary coding task.
- [x] Prepare and review an explicitly ignored user-run installed diagnostic using the actual owned runtime/PTY and Task4 relay. Start no real model conversation from the controller. Isolate its metadata/hooks/config without altering existing sessions or global integration; preserve login/config authority and document any difference that prevents parity proof. Never record capabilities, prompts, raw frames or native IDs as evidence. Compile/test the setup/cleanup with fake fixtures; distinguish unexecuted operator gates from verified results. If any live gate cannot be completed safely without the user, present the concrete prepared command and missing action; continue independent verification. No shipping table entry and no #690 fixed claim until all required checks for that entry pass. Unsupported OS/config keeps direct launch.
- [ ] Perform fresh correctness and completeness reviews against the complete diff, explicit `/code-review medium`, reconcile findings, and run docs/worktree currency checks. Check Taskmaster API for an exact matching active recommendation; accept/complete only after verified disposition using the authenticated session API.
- [ ] Open one PR referencing #690, approved design, test evidence, compatibility entries and any remaining release gates. Follow required CI/review/bounded PR babysitting. Merge only with passing checks and required review gate, then guarded worktree cleanup; keep issue open if behavior gates are incomplete.

Final-review constraint (2026-10-06): #763 requires a reviewed owned-listener
contract before any production or installed-diagnostic bearer delivery.
The local diagnostic is compiled/fake-tested preparation only and is blocked
from installed execution. Add a fail-closed guard and accepting-competitor
regression; retain narrowly controlled cfg(test) fake fixtures. No new
upstream handoff or readiness contract is implemented under this plan.
