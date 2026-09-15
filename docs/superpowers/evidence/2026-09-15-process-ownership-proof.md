# Process ownership proof — Task 6 production seam evidence

Status: Task 6 audit complete; production integration remains gated. This
record is durable evidence for the fixture findings and the production seam
audit. It does not implement runtime recovery, multi-workspace replacement, a
supervisor, or an ADR decision.

## Decision boundary

The Task 1–5 fixture direction is a native owner outside Electron's kill
domain. It must authorize every process root, register the root before
release/execute, freeze admission on owner-channel loss, clean only registered
roots within bounded graceful and forced phases, and return an authenticated
complete-exit or unresolved receipt. Persisted PIDs, names, paths, released
leases, sidecar-held PTY handles, and unproven process groups are not ownership
authority.

Task 3's Windows Job Object mechanism is accepted as fixture evidence after
the hosted native run. Task 4 rejected the macOS/portable-Unix candidates.
Task 5 accepted the strengthened fixture race/cleanup/foreign-owner semantics
with its documented launch-dependent platform limits. These results do not
yet authorize production integration because there is no demonstrated
cross-platform production boundary.

## Fixture evidence ledger

| Evidence | Mechanism | Host / architecture / mode | Exact command | Parent termination / identities / containment | Result and limits |
| --- | --- | --- | --- | --- | --- |
| Task 1 protocol | Authenticated one-use ticket, generation/role/executable/request-nonce binding, receipt validation | Fixture scope; platform-neutral contract | `cargo test --manifest-path crates/process-ownership-fixture/Cargo.toml --test protocol` (recorded in Task 1 report) | Protocol-level identity and authorization only; no OS containment | Accepted contract fixture; not a native process proof. |
| Task 2 adapter | Ticketed admission and registration adapter | Fixture scope; platform-neutral adapter | Task 2 focused commands recorded in `task-2-report.md` | Adapter seam only; no kernel-bound owner | Accepted as a contract seam with parked platform/cleanup gaps. |
| Task 3 Windows | Private unnamed Job Object, kill-on-close, suspended registration, retained process handles and creation-time identities | Hosted Windows Server 2025 x64 MSVC, native runtime; GitHub Actions run `34995897012`, job `104471969730` | `cargo test --locked --manifest-path crates/process-ownership-fixture/Cargo.toml --test windows_job` | Forced Electron-like parent termination; 8 discovered, 8 passed; independent Job Object census and retained native identities | Accepted Windows native fixture mechanism. It covers the fixture topology, not production or Unix. |
| Task 4 Unix candidates | `ProcessGroup`, `RegisteredRoot`, and `Launchd` candidates | macOS Darwin 25.6.0 arm64; fixture host | `cargo test --manifest-path crates/process-ownership-fixture/Cargo.toml --test unix_candidates -- --nocapture` | No candidate launched; no macOS PTY EPERM or native launchd run | Rejected/unsupported: no kernel-bound identity proof against PID/PGID reuse; launchd lifecycle remained untested. The six rejection tests passed. |
| Task 5 cleanup matrix | Foreign advisory lease, generation barriers, owner-loss race, bounded cleanup and receipts | macOS Darwin 25.6.0 arm64; 4 applicable rows, 14 launch-dependent rows skipped | `cargo test --manifest-path crates/process-ownership-fixture/Cargo.toml --test cleanup_matrix -- --nocapture` | 5 s graceful plus 5 s forced phase in the fixture; survivors remain unresolved; foreign state is observed independently | 4 applicable tests passed. Launch-dependent native rows were skipped because the Task 4 host adapter rejects the native launch path. |
| Task 5 full fixture | Same as above | macOS Darwin 25.6.0 arm64 | `cargo test --manifest-path crates/process-ownership-fixture/Cargo.toml` (twice) | No native Unix/Windows runtime claim | 47 integration tests passed on both runs; platform limits and two parked proof gaps remain in `task-5-report.md`. |

### Accepted and rejected mechanisms

- Accepted for fixture evidence: Windows Job Object ownership with suspended
  registration, retained native handles/creation identities, independent
  census, breakaway rejection, non-inheritable handles, and forced parent
  termination. The hosted run is the source for the 8/8 native result.
- Rejected for production selection: portable Unix process groups and
  registered-root/PID cleanup, because they cannot prove ownership against
  reuse or unobserved descendants; macOS `launchd`, because its lifecycle was
  not tested as a complete owner boundary.
- Not selected: a fixture-only generic adapter as a production implementation.
  It does not supply a native, cross-platform, crash-surviving owner by itself.

### Parked fixture gaps

- Task 5 parks proof that cleanup acknowledgement is serialized against an
  in-flight paused launch; the cleanup thread currently waits for owner loss
  to complete.
- Task 5 parks the encoded framing bound: survivor/reason limits count raw
  bytes rather than escaped JSON bytes, and escaped-identity coverage remains
  required.
- Native macOS and portable-Linux owner mechanisms remain unresolved. No
  launch-dependent Task 4/5 row is converted into a pass by the macOS fixture
  result.

## Production seam status

| Production surface | Exact location | Status against fixture boundary |
| --- | --- | --- |
| Electron sidecar launch | `apps/desktop/electron/main.ts:574-588`, direct `spawn` at `:580` | **Open blocker.** Direct Electron child; no external owner, launch ticket, registration, parent-loss channel, or receipt. |
| Electron lifecycle and cleanup | `apps/desktop/electron/sidecarLifecycle.ts:5-20,87-110,175-239,242-276`; `main.ts:1260-1277` | **Open blocker.** In-memory stale-event generations and one-shot `kill()`/retry do not prove descendant cleanup or forced-parent recovery. |
| PTY root | `crates/orkworksd/src/runtime/session_runtime.rs:817-888`, `pair.slave.spawn_command(cmd)` at `:882` | **Open blocker.** The PTY child executes before owner registration; sidecar memory retains a killer/wait task only. |
| Provider runner | `crates/orkworksd/src/providers.rs:847-908`, direct callback `cmd.spawn()` at `:892-894` | **Open blocker.** Unix process group and the Windows per-invocation Job are local invocation controls, not sidecar-wide ownership. |
| Custom inference | `crates/orkworksd/src/taskmaster/runtime/inference.rs:238-270`; `providers/custom_inference.rs:159-173` | **Partial local protection.** It uses `ProcessRunner`, and `providers.rs:916-945` attaches a Windows `ProcessJob` for that invocation. It still lacks sidecar-wide registration/admission and does not cover PTY, discovery, or other roots. |
| Native inference | `crates/orkworksd/src/providers/inference.rs:307-323`; `providers.rs:1838-1866` | **Open blocker.** Version probe and CLI execution use local `ProcessRunner`; no surviving owner generation/barrier. Ollama HTTP is not a process root. |
| Provider discovery | `crates/orkworksd/src/providers.rs:2184-2205,2275-2300` | **Open blocker.** List-models and Codex app-server discovery spawn directly outside the owner boundary. |
| Harness detection | `crates/orkworksd/src/harness/detect.rs:99-145`, spawn at `:125` | **Open blocker for complete inventory.** `kill_on_drop` bounds the short probe, but it is not registered with a sidecar owner. |
| Stale PTY metadata | `crates/orkworksd/src/main.rs:183-190`; `session_application.rs:2249-2267`; `metadata.rs:673-708` | **Open blocker.** Ephemeral diagnostic PIDs and missing in-memory handles cause orphan reconciliation to mark metadata ended without owner evidence; unresolved liveness is not represented. |

`ProcessJob` is therefore insufficient as the production boundary: it is
private, per-provider-invocation, Windows-only, and absent from the PTY,
discovery, sidecar, and complete generation lifecycle.

## Facts, inferences, and open risks

### Facts

- Production has no `processSupervisor.ts` or `process_ownership.rs`.
- The sidecar exposes a dynamic localhost port, not an authenticated owner
  rendezvous or complete-exit receipt.
- Current cleanup covers write/read/wait error paths and invocation timeouts
  locally, but not a sidecar crash or an owner that outlives Electron.
- `session_pids` stores only a `u32` PTY PID for cwd probing and is removed when
  session tracking clears; it is not a birth identity or ownership record.
- The required Node lifecycle/restoration tests passed 24/24 on this host.
- The focused harness/detect tests passed 21/21; no harness/detect failure
  occurred in the recorded baseline runs.

### Inferences

- A crashed sidecar can leave a PTY or provider descendant alive while startup
  reconciliation classifies its persisted session as ended. That is a
  false-empty basis for cleanup/recovery under the fixture contract.
- A local Windows Job or Unix process group can protect one invocation while
  its owning sidecar is alive, but cannot prove whole-workspace ownership or
  cleanup after the sidecar/Electron parent disappears.
- Existing runtime/backend generations reject stale responses but do not block
  a replacement generation on proof that old OS roots have exited.

### Open risks

- No production adapter can be added without first selecting and demonstrating
  a native mechanism for macOS/portable Unix as well as Windows.
- Every direct production root in the seam table must be routed through one
  mandatory owner adapter; routing only `ProcessRunner` would leave bypasses.
- Task 5's two fixture gaps and the native platform limits remain prerequisites
  for a complete evidence/ADR decision.

## Verification limits and exact commands

Task 6 audit host: macOS Darwin 25.6.0 arm64 (`aarch64-apple-darwin`), Rust
1.96.0, Cargo dev profile. The following commands were run from the provided
worktree.

```text
/opt/homebrew/bin/rg -n "Command::spawn|portable_pty|spawn\(" crates/orkworksd/src apps/desktop/electron
exit 0; direct production seams enumerated above

cargo test --manifest-path crates/orkworksd/Cargo.toml
exit 101; 1266 passed, 1 failed, 3 ignored
failure: providers::tests::process_runner_cleans_up_provider_that_closes_stdin_during_prompt_write
at crates/orkworksd/src/providers.rs:3665: expected broken-pipe prompt-write error, got "timed out"
elapsed: 74.55s

set -o pipefail; cargo test --manifest-path crates/orkworksd/Cargo.toml 2>&1 | tail -100
exit 101; 1258 passed, 9 failed, 3 ignored
the 9 failures were PermissionDenied setup failures in provider/server-backed
tests under the default sandbox; no harness/detect test failed
elapsed: 74.05s

set -o pipefail; cargo test --manifest-path crates/orkworksd/Cargo.toml 2>&1 | tail -120
exit 101; 1266 passed, 1 failed, 3 ignored
run with sandbox restriction lifted; same ProcessRunner prompt-write failure
elapsed: 75.12s

cargo test --manifest-path crates/orkworksd/Cargo.toml harness::detect::tests -- --nocapture
exit 0; 21 passed, 0 failed, 0 ignored, 1249 filtered out
elapsed: 3.11s

cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check
exit 0

cd apps/desktop && node --experimental-strip-types --test tests/sidecarLifecycle.test.ts tests/backendRestoration.test.ts
exit 0; 24 passed, 0 failed, 0 skipped, 0 todo
elapsed: 91.40ms

git diff --check
exit 0 after the audit changes
```

The full elevated baseline is the reliable count for this host: one existing
ProcessRunner prompt-write test fails. The non-elevated 9-failure result is
retained as a sandbox limit, not merged into the code-failure count. No npm
install or dependency download was attempted in this fix round. No production
runtime test can prove the missing owner boundary; the 24 Node tests cover the
existing lifecycle/restoration controller only.

## Changed files in Task 6 fix round 1

- `docs/superpowers/evidence/2026-09-15-process-ownership-proof.md`
- `.superpowers/sdd/2026-09-15-process-ownership-proof/task-6-report.md`

No files under `apps/desktop/electron/` or `crates/orkworksd/src/` changed.
