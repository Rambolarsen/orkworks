# Task 6 report — production process-ownership seam audit

Status: audit complete; production integration is intentionally not gated in.
Tasks 3–5 do not provide a passing native mechanism on every supported Unix
target, so this task made no runtime, recovery, replacement, or ADR changes.

## Scope and gate ruling

This audit compares production launch and cleanup behavior with the Task 1–5
fixture contracts in
`docs/superpowers/specs/2026-09-15-process-ownership-proof-design.md`.
The selected fixture direction is an owner outside Electron's kill domain that
authorizes and registers every process root, freezes admission on owner loss,
and returns an authenticated complete-exit or unresolved receipt.

The gate remains closed:

- Task 3 accepted the Windows Job Object fixture evidence only on its hosted
  Windows run. It does not establish a sidecar-wide boundary or a Unix
  mechanism.
- Task 4 rejected the macOS/portable-Unix process-group and registered-root
  candidates; no deployable Unix mechanism was demonstrated.
- Task 5 strengthened the fixture race/cleanup/foreign-owner matrix, but its
  launch-dependent rows were skipped on macOS and Linux/Windows native runtime
  evidence was not produced in this checkout.

Consequently, the production files listed in the Task 6 brief were inspected
but left unchanged. `ProcessJob` is insufficient as the production boundary.
No ADR draft was required: the evidence records an unresolved prerequisite,
and ADR 0056/Task 7 owns the later mechanism decision.

## Exact production seam audit

The required static audit command was:

```text
/opt/homebrew/bin/rg -n "Command::spawn|portable_pty|spawn\(" crates/orkworksd/src apps/desktop/electron
```

The production process seams found were:

| Production seam | Exact location | Routed through proven owner boundary? | Finding |
| --- | --- | --- | --- |
| Electron launches `orkworksd` | `apps/desktop/electron/main.ts:574-588`, direct `spawn` at `:580` | No | Electron owns the child directly; there is no external supervisor, authenticated launch ticket, generation-bound registration, parent-loss channel, or completion receipt. `ORKWORKS_OPEN_PLAN_TOKEN` is the sidecar API token, not an ownership ticket. |
| Electron lifecycle termination/retry | `apps/desktop/electron/sidecarLifecycle.ts:5-20,87-110,175-239,242-276` | No | The process interface exposes only `kill()`. Obsolete generation events are ignored by the in-memory `isCurrent` check, but an obsolete process that survives `kill()` has no owner receipt or unresolved state. Recovery is bounded retry, not descendant cleanup. |
| Electron quit/signal cleanup | `apps/desktop/electron/main.ts:1260-1277` | No | `before-quit`, SIGTERM, and SIGINT call `dispose()`/`kill()` without waiting for process exit, a 5 s + 5 s owner cleanup protocol, survivor reporting, or forced-Electron-termination recovery. |
| Session PTY root | `crates/orkworksd/src/runtime/session_runtime.rs:817-888`, direct `pair.slave.spawn_command(cmd)` at `:882` | No | The PTY is created and the child executes before any owner registration. The reporting token is installed after spawn. `SessionRuntime` retains a killer and wait task (`:948-984`), not a crash-surviving owner. Startup cancellation uses direct `child.kill()`/`wait()` (`:763-804`). |
| Provider/process runner root | `crates/orkworksd/src/providers.rs:847-908`, direct callback `cmd.spawn()` at `:892-894` | No | The generic callback is a bypassable spawn seam. Unix sets a process group (`:913-914`) and Windows creates a per-invocation `ProcessJob` (`:916-920`), but neither is a sidecar-wide owner adapter. |
| Native inference version probe and execution | `crates/orkworksd/src/providers/inference.rs:307-323`; `crates/orkworksd/src/providers.rs:1838-1866` | No | Both the compatibility probe and actual CLI inference eventually use the same local `ProcessRunner`; neither receives a supervisor ticket or participates in an owner generation/barrier. Ollama's HTTP path is not a process root. |
| Custom inference callback | `crates/orkworksd/src/taskmaster/runtime/inference.rs:238-270`, direct `command.spawn()` at `:256-261` | Partially: local invocation runner only; not the required sidecar-wide boundary | The callback is executed inside `PreparedCustomInference::run_with_spawn` (`crates/orkworksd/src/providers/custom_inference.rs:159-173`), which routes the invocation through `ProcessRunner`; on Windows that runner creates/attaches a per-invocation `ProcessJob` before resume (`crates/orkworksd/src/providers.rs:916-945`). This protects that provider invocation only. It does not register with a sidecar-wide owner and does not cover PTY, discovery, the sidecar, or other inference roots. Trust, definition, and runtime-generation revalidation still does not freeze admission on owner loss. |
| Provider model discovery | `crates/orkworksd/src/providers.rs:2184-2205`, direct `spawn()` at `:2191-2196` | No | A declared list-models command is launched outside `ProcessRunner`/`ProcessJob`, with local pipe/timeout cleanup only. |
| Codex app-server model discovery | `crates/orkworksd/src/providers.rs:2275-2300`, direct `spawn()` at `:2283-2289` | No | The app-server probe is another direct process root with local kill/wait behavior and no owner registration. |
| Harness version probe | `crates/orkworksd/src/harness/detect.rs:99-145`, direct `tokio::process::Command.spawn()` at `:125` | No | `kill_on_drop(true)` bounds this short probe, but it is not connected to the process-owner protocol. It is not a PTY or inference root, but it is a production spawn seam from the required static audit. |

The static results in `crates/orkworksd/src/procfs.rs` and the harness
integration `.spawn()` helpers under `#[cfg(test)]` are test-only and were not
counted as production seams. `providers/custom_inference.rs:150-153` is also a
test-only direct-spawn convenience; its production callback is the
`taskmaster/runtime/inference.rs` seam above.

## Boundary, cleanup, and stale-state findings

### Facts

- `main.ts:580` establishes a normal Electron child relationship. The sidecar
  announces only a dynamic localhost port (`crates/orkworksd/src/main.rs:282-291`);
  no owner channel or rendezvous protocol exists.
- `sidecarLifecycle.ts` has an in-memory numeric generation (`:57-60,175-205`)
  for stale event suppression. It is not an OS identity, supervisor generation,
  or cleanup receipt.
- `ProcessJob` is private to the provider module
  (`crates/orkworksd/src/providers/windows_process.rs:18-97`). It creates a
  kill-on-close Job Object and attaches/resumes one suspended provider root.
  It does not contain the PTY root, sidecar process, discovery probes, custom
  inference, or roots launched after the per-invocation job is created. It has
  no authenticated ticket, cross-root admission gate, Electron parent-loss
  detection, rendezvous, complete-exit receipt, or Unix implementation.
- Provider cleanup is local to each invocation's write, read, wait, and timeout
  error paths. On Unix it kills `-(pid)` (`providers.rs:974-1001`) and polls
  for up to one second; on Windows it terminates the per-invocation Job Object
  and falls back to `child.kill()` (`:987-991`). The source comment explicitly
  acknowledges that a supervisor would change the semantics. This is useful
  local cleanup, not a sidecar-wide owner receipt or crash-surviving boundary.
- `AppState.session_pids` is a `HashMap<String, u32>` used to probe a PTY
  child's current working directory (`crates/orkworksd/src/main.rs:183-190`).
  It is removed when session tracking is cleared
  (`crates/orkworksd/src/session_application.rs:1952-1973`). It stores no
  birth identity, retained handle, owner generation, or descendant set.
- On workspace open, persisted `running`/`creating` sessions absent from the
  current sidecar's in-memory session map are immediately passed to
  `reconcile_orphaned_session` (`session_application.rs:2249-2267`). That
  function unconditionally derives a terminal status and marks the session
  offline (`crates/orkworksd/src/metadata.rs:673-708`). It does not query a
  surviving owner or distinguish unresolved liveness from an empty set.
- The existing Electron backend restoration coordinator and sidecar lifecycle
  generation checks protect against stale responses/events, but they do not
  prove that the old OS process tree exited before replacement.

### Inferences

- A sidecar crash can leave a PTY child or provider descendant outside the
  sidecar's in-memory handles. Current startup reconciliation will classify
  metadata as ended while that process may still exist; this is a false-empty
  cleanup/recovery basis under the fixture contract.
- A numeric PID or process-group kill cannot prove ownership after a crash or
  PID/PGID reuse. The current Unix provider cleanup is safe only as a
  best-effort local invocation operation while the `Child` is still held; it is
  not evidence for restart adoption or foreign-owner isolation.
- The current `open-workspace` path starts a replacement through
  `sidecarLifecycle.start` (`main.ts:1198-1224`), but the requested Task 6
  proof must precede any replacement over a potentially surviving old root.
  Implementing that barrier is deliberately outside this audit.

### Open risks / blockers

- No production native owner is available on macOS/portable Unix, and no
  cross-platform production adapter has been selected. A `process_ownership.rs`
  adapter and `processSupervisor.ts` must not be invented from the fixture
  abstractions before the platform mechanism is demonstrated.
- Direct process roots remain bypassable in provider discovery and the custom
  inference callback's caller-supplied spawn seam even aside from the PTY and
  main sidecar roots. Custom inference does use `ProcessRunner` and, on
  Windows, its per-invocation `ProcessJob`; a future adapter must still make
  the sidecar-wide owner mandatory for every root in the table, not only one
  invocation.
- No production implementation proves registration-before-execute, owner-loss
  freeze/drain, five-second graceful plus five-second termination deadlines,
  unresolved survivors, generation-bound adoption, or forced Electron
  termination followed by immediate relaunch.
- Existing provider timeout tests cover local cleanup behavior, not the
  sidecar-crash and foreign-owner contracts. The baseline command below also
  has an unrelated provider prompt-write failure and should not be attributed
  to this audit.

## Task 1–5 fixture contract mapping

| Fixture requirement | Production status |
| --- | --- |
| Authenticated one-use launch tickets bound to generation/role/executable/request nonce | **Absent.** Sidecar API token, trust checks, and runtime generations are not launch tickets. |
| Registration before release/execute; registration failure means no child execution | **Absent.** PTY `spawn_command` and provider `cmd.spawn` execute without owner registration. |
| Independent native identity and observation | **Absent.** Production records a PTY PID for cwd probing and uses live `Child` handles locally; no owner-controlled identity/observation set exists. |
| Owner-loss admission freeze, in-flight drain, and launch-race closure | **Absent.** No parent-loss channel reaches sidecar admission or provider/custom inference. |
| PTY separate session, provider roots, descendants, daemon/reparent/non-reporting cases | **Unproved.** Existing process groups and per-provider Windows Jobs do not cover all roots or the crash-surviving owner boundary. |
| Bounded graceful cleanup, forced termination, complete-exit acknowledgement, unresolved survivor receipt | **Absent.** Current write/read/wait error paths and invocation timeouts use direct kill/wait or local one-second polling; Electron uses `kill()` without acknowledgement. |
| Foreign-owner safety and no broad PID/name/cwd cleanup | **Not implemented for this boundary.** Current code does not attempt cross-generation process cleanup; metadata reconciliation can falsely classify liveness but does not provide an ownership proof. |
| Authenticated rendezvous and adoption only after a complete-exit receipt | **Absent.** No surviving supervisor, rendezvous, or receipt exists. |
| Non-inheritable owner endpoint and no leaked control capability | **Absent.** There is no owner endpoint. Electron stdio and the sidecar HTTP token are unrelated to this contract. |
| Stale PID/metadata handling: unresolved is not empty; persisted PID alone is insufficient | **Gap.** `session_pids` is diagnostic-only and ephemeral; orphan reconciliation marks missing in-memory sessions ended without owner evidence. |
| Two-generation replacement and inference barrier while older roots survive | **Partial only for stale state.** In-memory lifecycle/backend/runtime generations reject stale responses and config, but do not block OS admission on surviving roots. |
| Forced Electron parent termination, including background workspaces, then immediate relaunch | **Unproved/absent.** No external owner survives Electron, and no production test or receipt gates relaunch. |

## Verification

Host: macOS Darwin 25.6.0 arm64 (`aarch64-apple-darwin`). All commands were
run from `/Users/froomiebot/workspace/orkworks/.worktrees/545-process-ownership-proof`.

Exact commands and results:

```text
/opt/homebrew/bin/rg -n "Command::spawn|portable_pty|spawn\(" crates/orkworksd/src apps/desktop/electron
exit 0; direct production seams are listed above; test-only results excluded

cargo test --manifest-path crates/orkworksd/Cargo.toml
exit 101; 1266 passed, 1 failed, 3 ignored, 0 measured
failure: providers::tests::process_runner_cleans_up_provider_that_closes_stdin_during_prompt_write
at crates/orkworksd/src/providers.rs:3665: expected broken-pipe prompt-write error, got "timed out"
elapsed: 74.55s

cargo test --manifest-path crates/orkworksd/Cargo.toml 2>&1 | tail -100
exit 101; 1258 passed, 9 failed, 3 ignored, 0 measured
the 9 failures were PermissionDenied setup failures in provider/server-backed
tests under the default sandbox; no harness/detect test was among the failures
elapsed: 74.05s

cargo test --manifest-path crates/orkworksd/Cargo.toml 2>&1 | tail -120
exit 101; 1266 passed, 1 failed, 3 ignored, 0 measured
run with the sandbox restriction lifted; same ProcessRunner prompt-write
failure above, no harness/detect failure
elapsed: 75.12s

cargo test --manifest-path crates/orkworksd/Cargo.toml harness::detect::tests -- --nocapture
exit 0; 21 passed, 0 failed, 0 ignored, 0 measured; 1249 filtered out
elapsed: 3.11s

cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check
exit 0

git diff --check
exit 0

cd apps/desktop && node --experimental-strip-types --test tests/sidecarLifecycle.test.ts tests/backendRestoration.test.ts
exit 0; 24 passed, 0 failed, 0 skipped, 0 todo
elapsed: 91.40ms
```

The failed Rust test is an existing baseline failure observed without any
production or fixture changes; it exercises the local ProcessRunner prompt
write path, not the unimplemented owner boundary. The focused harness/detect
subset passed separately (21/21), so no harness/detect failure occurred in
these runs. The documented Electron lifecycle/restoration tests passed
separately (24/24); no npm install or dependency download was attempted in
this fix round.

## Changed files

Before editing, the requested worktree was clean on
`issue-545-process-ownership-proof` (ahead 8 of its remote). This task changed
only:

- `.superpowers/sdd/2026-09-15-process-ownership-proof/task-6-report.md`
- `docs/superpowers/evidence/2026-09-15-process-ownership-proof.md`

No files under `apps/desktop/electron/` or `crates/orkworksd/src/` were
modified. No runtime recovery, multi-workspace replacement, production launch
adapter, supervisor, or ADR draft was added.
