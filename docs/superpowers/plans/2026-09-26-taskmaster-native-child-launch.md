# Taskmaster Native Child-Launch Boundary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task by task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prove one production-backed Linux/OpenCode child-launch boundary through a narrow Rust adapter and native fixtures, without enabling master-session runner launches.

**Architecture:** A Linux-only Rust adapter creates one generation-owned transient systemd user service and applies canonical worktree, namespace, mount, Landlock, seccomp, credential, descriptor, and cgroup policies before a helper executes. Durable generation records and exact unit/cgroup observations support fail-closed cancellation and restart reconciliation. The qualification artifact can set `native_boundary_proven` only for the pinned tuple; `runner_eligible` remains false until the separate #610 requirements are implemented and reviewed.

**Tech Stack:** Rust 2021, existing `libc` and `sha2` dependencies where sufficient, systemd user services, cgroup v2, Linux namespaces, Landlock ABI 6 or newer, seccomp BPF, native Rust helper fixtures, and GitHub Actions `ubuntu-24.04` x86_64.

**Spec:** [`docs/superpowers/specs/2026-09-25-taskmaster-native-child-launch-boundary-design.md`](../specs/2026-09-25-taskmaster-native-child-launch-boundary-design.md)

## Global Constraints

- Implement only #617's native-boundary prerequisite; do not implement the #610 coordinator, approval flow, tool/model broker, or real child coding work.
- Qualify only Ubuntu 24.04 LTS x86_64 with OpenCode CLI `v1.18.18`, `@opencode-ai/plugin@1.18.18`, `@opencode-ai/sdk@1.18.18`, and the exact code-owned built-in `opencode` definition generation and content digest.
- A native boundary pass sets only `native_boundary_proven`; `runner_eligible` remains false until every separate #610 gate passes.
- All required namespace, mount, Landlock, seccomp, systemd, cgroup, descriptor, capability, and process-inspection controls fail closed when unavailable or ambiguous.
- The systemd user service owns the complete generation cgroup; termination is proven only after both the exact unit is inactive and its cgroup is empty.
- Use the exact canonical worktree identity and an explicit environment/descriptor allowlist. Never inherit user credentials or expose a host socket or network route to the child.
- Apply `PR_SET_NO_NEW_PRIVS=1` after namespace and mount setup and before Landlock and seccomp; drop all child capability sets before exec.
- Fixtures use only a purpose-built helper, never OpenCode or another real coding tool. Native qualification uses the production adapter and policy builder on the exact `ubuntu-24.04` runner without privileged setup or weaker fallbacks.
- Every code change follows test-driven development: add a focused failing test, confirm the expected failure, implement the smallest behavior, then rerun that test.
- If the exact native runner cannot provide the approved controls without host privilege elevation, stop at the no-go gate and return to design review; do not silently select another OS, owner primitive, harness, or weaker policy.

---

## Planning checkpoint: investigated risks and assumptions

**What I was least confident about:** Whether GitHub's exact Ubuntu 24.04 runner provides a user systemd manager with delegated `cpu`, `memory`, and `pids` controllers to the unprivileged workflow user, and whether an unprivileged transient service can supply the unit/cgroup lifecycle required by the spec. The Rust crate has no systemd D-Bus dependency today; it has `libc`, `sha2`, `uuid`, and Tokio. Current provider process ownership is separate from PTY/harness launch and cannot be reused as proof.

**Investigation and disposition:** The current issue #617 explicitly requires the production adapter and native evidence and says unsupported hosts remain unavailable. GitHub documents passwordless `sudo` on Linux VMs, but that is not evidence of unprivileged user-manager delegation ([GitHub-hosted runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)). systemd documents that delegated controllers depend on the service manager's actual delegation and must be available in the unit's subtree ([systemd cgroup delegation](https://github.com/systemd/systemd/blob/main/docs/CGROUP_DELEGATION.md)). The plan therefore puts an unprivileged runner capability probe and transient-unit smoke test first; production sandbox work is blocked until it passes. If it fails, the approved spec's no-go path applies.

**What the project may be missing:** The chosen runner image and distro version alone do not prove the user-level systemd/cgroup facilities, and the native policy is too security-sensitive to infer them from version strings. The approved spec already treats these as required runtime probes. The plan retains that gate and requires the qualification evidence to record the actual runner image, kernel, systemd, cgroup controllers, and Landlock ABI.

**Remaining implementation risks:** The spec intentionally leaves the exact systemd client mechanism, owner process entry point, authenticated owner channel, durable generation-record schema, exact Landlock filesystem rights, CPU sampling interval, and OpenCode binary distribution/digest to implementation proof. Task 1 records these choices in ADR 0065 before production boundary code. No choice may weaken the approved controls; a choice that changes scope returns to spec review.

---

### Task 1: Prove native runner prerequisites and bind implementation architecture

**Files:**
- Create: `crates/orkworksd/src/bin/taskmaster-native-runner-preflight.rs`
- Create: `crates/orkworksd/src/taskmaster/native_runner/host_probe.rs`
- Create: `crates/orkworksd/src/taskmaster/native_runner/host_probe_tests.rs`
- Modify: `crates/orkworksd/src/taskmaster/mod.rs`
- Modify: `.github/workflows/pr-ci.yml`
- Create: `docs/adr/0065-linux-taskmaster-native-child-launch-boundary.md`
- Modify: `docs/adr/README.md`

**Interfaces:**
- Consumes: the approved Ubuntu/OpenCode tuple and required host capabilities in the spec.
- Produces: `HostProbeReport`, an unprivileged probe that checks cgroup v2 controllers, the user systemd manager, transient-service creation, namespace creation, private mount/`/proc`, Landlock ABI and ABI-6 signal scope, and seccomp-filter support. ADR 0065 records the chosen systemd invocation API, owner entry point, authenticated local control channel, durable generation record, and exact failure semantics before implementation code depends on them.

- [ ] Write parser/decision tests for a fully supported host, each missing required control, and indeterminate systemd/cgroup responses; assert every incomplete or ambiguous report is ineligible.
- [ ] Run the focused tests and confirm they fail because the probe report and eligibility function do not exist.
- [ ] Implement the probe with injectable command/syscall results so unit tests do not claim native qualification; do not use `sudo` for any prerequisite or transient-service probe.
- [ ] Add a `ubuntu-24.04` PR CI job that runs the preflight as the workflow user and retains the machine-readable report with runner image, kernel, systemd, controller, and Landlock details.
- [ ] Run the job on the exact candidate image. Confirm an unprivileged transient user service can be started, inspected, stopped, and proven inactive with an empty cgroup; confirm required controllers are delegated.
- [ ] Record the exact systemd client mechanism and owner/control-channel design in ADR 0065, including generation record path/fields and the rule for ambiguous service-manager responses.
- [ ] If any required facility is unavailable, mark this plan stopped at the no-go gate, update #617 with the measured evidence, and return to spec review. Do not start Tasks 2–7 or substitute a privileged/relaxed path.
- [ ] Run `cargo test --locked --manifest-path crates/orkworksd/Cargo.toml taskmaster::native_runner::host_probe` and `cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check` after the probe implementation.

### Task 2: Define generation identity, immutable launch contract, and qualification facts

**Files:**
- Create: `crates/orkworksd/src/taskmaster/native_runner/model.rs`
- Create: `crates/orkworksd/src/taskmaster/native_runner/identity.rs`
- Create: `crates/orkworksd/src/taskmaster/native_runner/record.rs`
- Create: `crates/orkworksd/src/taskmaster/native_runner/contract_tests.rs`
- Modify: `crates/orkworksd/src/taskmaster/native_runner/mod.rs`
- Modify: `crates/orkworksd/src/taskmaster/mod.rs`
- Modify: `crates/orkworksd/src/harness/definition.rs`

**Interfaces:**
- Consumes: `HostProbeReport` and the service-manager/authentication choices from ADR 0065.
- Produces: `LaunchRequest`, `GenerationId`, `CanonicalWorktree`, `HarnessIdentity`, `ResourceLimits`, `GenerationRecord`, and a narrow `NativeLaunchBoundary` contract with `launch`, `status`, `terminate`, and `reconcile` operations. A termination result is proven only when the exact recorded systemd unit is inactive and its exact cgroup is empty. The record contains no bearer token, credential, inherited environment, or raw command output.

- [ ] Write failing tests for empty/stale generation IDs, noncanonical or replaced worktree handles, missing/unbounded resource limits, non-OpenCode definitions, mismatched harness generations, and ambiguous or incomplete termination evidence.
- [ ] Write failing identity tests that parse the code-owned built-ins, locate exactly one `opencode` definition, and verify that both its explicit generation and SHA-256 digest of the canonical serialized definition are required for qualification.
- [ ] Run the focused tests and confirm they fail on missing types/validation.
- [ ] Implement the contract and durable generation record. Include the workspace identity, exact unit name, canonical worktree identity, OpenCode generation/digest, finite limits, and lifecycle evidence required for exact restart reconciliation.
- [ ] Derive the OpenCode identity from `EMBEDDED_BUILTINS`; do not use a user override or resolved custom definition. Keep CLI executable digest and plugin/SDK versions as separate qualification facts.
- [ ] Add tests proving any changed built-in field, definition generation, or executable digest invalidates the previous native qualification identity, while unrelated user overrides cannot create a qualifying identity.
- [ ] Run the focused `taskmaster::native_runner` tests and Rust formatting check.

### Task 3: Implement generation-owned systemd lifecycle and fail-closed reconciliation

**Files:**
- Create: `crates/orkworksd/src/taskmaster/native_runner/supervisor.rs`
- Create: `crates/orkworksd/src/taskmaster/native_runner/owner.rs`
- Create: `crates/orkworksd/src/taskmaster/native_runner/service_manager.rs`
- Create: `crates/orkworksd/src/taskmaster/native_runner/supervisor_tests.rs`
- Modify: `crates/orkworksd/src/main.rs`
- Modify: `crates/orkworksd/src/taskmaster/native_runner/mod.rs`

**Interfaces:**
- Consumes: `NativeLaunchBoundary`, `GenerationRecord`, and ADR 0065's systemd and owner-channel decisions.
- Produces: a generation-specific transient systemd user service running the internal owner mode of `orkworksd`; only that owner can spawn the helper/target. The sidecar can request launch, read status, cancel, and reconcile by exact `GenerationId`, but cannot spawn or signal the target directly.

- [ ] Write fake-service-manager tests for successful start, duplicate generation, stale generation, start timeout, ambiguous start response, owner handshake failure, exact-unit lookup, and refusal to adopt by PID or command line.
- [ ] Write lifecycle tests asserting cancellation, deadline, heartbeat loss, owner failure, and sidecar-channel closure fence the generation and request whole-unit control-group termination.
- [ ] Run the focused tests and confirm they fail for absent supervisor behavior.
- [ ] Implement service creation with `KillMode=control-group`, required delegation and resource properties, unique generation-derived unit names, and one immutable request per unit. Do not implement a fallback owner or reuse a unit for a replacement command.
- [ ] Implement the internal owner entry point and ADR 0065 authenticated local channel. Reject unauthenticated, duplicate, stale, replayed, and mismatched requests before child creation.
- [ ] Persist generation state atomically before launch can be acknowledged. On sidecar restart, query only the recorded unit and generation; retain the reservation as `termination_unproven` until unit inactivity and cgroup emptiness are both observed.
- [ ] Add native helper tests for owner crash and sidecar crash while a descendant is active; assert systemd terminates the full generation and a second launch is refused until reconciliation proves emptiness.
- [ ] Run focused unit tests, the native lifecycle tests on the exact runner, and `cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check`.

### Task 4: Enforce canonical worktree, credential, namespace, and syscall boundaries

**Files:**
- Create: `crates/orkworksd/src/taskmaster/native_runner/worktree.rs`
- Create: `crates/orkworksd/src/taskmaster/native_runner/linux/sandbox.rs`
- Create: `crates/orkworksd/src/taskmaster/native_runner/linux/landlock.rs`
- Create: `crates/orkworksd/src/taskmaster/native_runner/linux/seccomp.rs`
- Create: `crates/orkworksd/src/taskmaster/native_runner/linux/fd_policy.rs`
- Create: `crates/orkworksd/src/taskmaster/native_runner/policy_tests.rs`
- Modify: `crates/orkworksd/src/taskmaster/native_runner/mod.rs`
- Modify: `crates/orkworksd/src/taskmaster/native_runner/owner.rs`

**Interfaces:**
- Consumes: a validated `LaunchRequest`, owned generation cgroup, and native feature report.
- Produces: a sandboxed helper child in the generation's private PID/mount/network namespaces and private mount root, with a fresh read-only `/proc`, explicit FD/environment allowlists, Landlock process/filesystem restrictions, no capabilities, `PR_SET_NO_NEW_PRIVS`, and architecture-checked seccomp inherited by descendants.

- [ ] Write unit tests for directory-handle-derived worktree identity, repository/worktree verification, primary checkout/sibling rejection, symlink and path replacement, `.git`/`commondir`, hard-link, bind-mount alias, and unsupported filesystem cases.
- [ ] Write policy tests for the exact read/write allowlist, private scratch, private `/proc`, no host home/metadata/socket paths, no inherited socket descriptors, cleared environment, and required Landlock ABI/rights; assert missing policy rights prevent spawn.
- [ ] Write syscall-policy tests covering every denied socket syscall, all `io_uring` entry points, architecture checking, and rejection of unsupported compat syscall entry points.
- [ ] Run the focused tests and confirm they fail before implementing policy builders or child setup.
- [ ] Implement namespace/mount setup without host privilege elevation. Build a private root with declared runtime mounts, assigned worktree, private scratch, fresh generation `/proc`, and minimal `/dev`; expose no host `/run`, `/tmp`, home, keyring, agent, or sidecar socket paths.
- [ ] Implement the reviewed bootstrap sequence: establish cgroup and namespaces/mounts; prepare the FD allowlist; drop bounding/effective/permitted/inheritable/ambient capabilities; set `PR_SET_NO_NEW_PRIVS=1`; enforce Landlock; open executable/runtime files under that policy; install seccomp; verify final state; then exec.
- [ ] Pass an explicit environment only. Do not pass `HOME`, `ANTHROPIC_API_KEY`, `CODEX_HOME`, auth files, SSH/cloud/proxy credentials, provider secrets, or arbitrary caller variables.
- [ ] Add native sentinel tests for allowed worktree writes; denied primary/sibling/home/metadata access; post-policy worktree socket creation race; owner and sidecar process visibility split; `/proc` sensitive-read, ptrace, process-vm, pidfd, and signal denial; NNP/capability emptiness; and setuid/file-capability non-escalation.
- [ ] Run policy unit tests, native helper tests on the exact runner, and Rust formatting check. Any namespace or policy setup error must fail before the helper's first instruction.

### Task 5: Enforce aggregate resource, output, cancellation, and clean-exit semantics

**Files:**
- Create: `crates/orkworksd/src/taskmaster/native_runner/limits.rs`
- Create: `crates/orkworksd/src/taskmaster/native_runner/output.rs`
- Create: `crates/orkworksd/src/taskmaster/native_runner/lifecycle.rs`
- Create: `crates/orkworksd/src/taskmaster/native_runner/limit_tests.rs`
- Modify: `crates/orkworksd/src/taskmaster/native_runner/owner.rs`
- Modify: `crates/orkworksd/src/taskmaster/native_runner/supervisor.rs`

**Interfaces:**
- Consumes: generation cgroup handles, finite `ResourceLimits`, and owner control messages.
- Produces: bounded wall-clock, memory/swap, process-count, CPU-rate and cumulative CPU enforcement; bounded stdout/stderr/event capture; and status that distinguishes clean exit, limit breach, cancellation, forced termination, owner failure, and `termination_unproven`.

- [ ] Write tests for invalid/unbounded limits, aggregate cgroup CPU accounting, sampling/deadline overshoot, memory/swap and process-count breach, byte-bound output truncation/termination, and status transitions after cancellation or forced kill.
- [ ] Derive and document a finite CPU sampling interval and maximum measured overshoot using the native helper under parallel CPU load; do not treat `CPUQuota` as a cumulative budget.
- [ ] Run the focused limit/state tests and confirm they fail before enforcement code exists.
- [ ] Apply systemd/cgroup limits to the whole generation before execution. Bind the selected finite limits into the immutable request and record them with the result.
- [ ] Poll aggregate `cpu.stat` usage with a bounded interval and terminate the complete unit when the cumulative CPU limit is exceeded; record maximum observed overshoot.
- [ ] Bound output at collection time and retained-log time. Exceeding any byte ceiling terminates the full generation; raw output is not copied into durable generation records.
- [ ] Treat a helper's success report as untrusted. Return clean completion only after the owner observes a normal zero exit, inactive unit, and empty cgroup; kill requests and forced termination never produce success.
- [ ] Run all focused tests and native resource, cancellation, and forced-termination tests on `ubuntu-24.04`.

### Task 6: Complete the native fixture matrix through the production adapter

**Files:**
- Create: `crates/orkworksd/src/bin/taskmaster-native-boundary-fixture-helper.rs`
- Create: `crates/orkworksd/tests/taskmaster_native_runner_linux.rs`
- Modify: `.github/workflows/pr-ci.yml`
- Modify: `crates/orkworksd/Cargo.toml`
- Modify: `crates/orkworksd/Cargo.lock`

**Interfaces:**
- Consumes: the production adapter, policy builder, service manager, helper protocol, and exact runner preflight from Tasks 1–5.
- Produces: structured, retained per-case native fixture results and environment fingerprint. The helper reports only over its explicit pipe descriptor and contains no coding-tool implementation or credentials.

- [ ] Add the native clean-boundary and allowed/denied-path fixture cases first; run them to confirm the production adapter is exercised and the unsupported-host path fails closed.
- [ ] Add descendant escape, owner/sidecar crash, cancellation-at-start/running/descendant, forced-termination-is-not-success, and foreign-sentinel survival cases.
- [ ] Add resource-limit cases for aggregate memory, process count, wall clock, cumulative CPU, output bytes, and documented CPU overshoot.
- [ ] Add fake home, keyring, API-key, agent/cloud socket, inherited-FD, `/proc`, PID namespace, capability, NNP, setuid/file-capability, seccomp socket-race, and io_uring cases.
- [ ] Ensure every case invokes the same production adapter and policy builder; mocked systemd/cgroup/Landlock tests remain unit tests and cannot count as qualification evidence.
- [ ] Add `ubuntu-24.04` PR CI execution and retain structured output, commands, image/kernel/systemd versions, Landlock ABI/controllers, adapter/helper revisions, and failure reasons as a workflow artifact.
- [ ] Run the complete native matrix without privileged setup. A skipped case, missing artifact, unsupported prerequisite, ambiguous owner state, or failed foreign-sentinel check leaves the candidate unqualified.
- [ ] Run `cargo test --locked --manifest-path crates/orkworksd/Cargo.toml taskmaster::native_runner` and the dedicated native integration test on the exact candidate runner.

### Task 7: Publish exact qualification evidence and synchronize the gate

**Files:**
- Modify: `docs/validation/master-session-runner-confinement.md`
- Modify: `docs/adr/0060-independent-workspace-instances.md`
- Modify: `docs/agents/architecture.md`
- Modify: `docs/superpowers/specs/2026-09-25-taskmaster-native-child-launch-boundary-design.md`
- Modify: GitHub issue #617

**Interfaces:**
- Consumes: retained native fixture artifacts and exact definition/executable/version identities.
- Produces: an auditable qualification row for only Ubuntu 24.04 x86_64 + OpenCode CLI `v1.18.18` + the matching plugin/SDK versions + exact built-in definition generation/digest. `runner_eligible` is explicitly false; the row has no authority to launch work.

- [ ] Write the validation record test/check that rejects a missing fixture, changed definition digest, mismatched image, skipped case, nonempty cgroup, missing sentinel result, or uncertain owner status as unqualified.
- [ ] Pin the tested OpenCode CLI package/release source and capture its executable digest separately from plugin/SDK package versions and the code-owned definition digest.
- [ ] Update the confinement matrix with actual runner facts, per-case results, exact command/job URL, adapter/helper revisions, CPU overshoot, and remaining limits; do not promote a tuple using mocked results.
- [ ] Amend ADR 0060 with the Linux owner/control boundary and its limits. Keep its one-workspace-per-instance decision intact and do not add a peer-instance registry.
- [ ] Update the architecture reference with the new Rust module and data flow; state that normal interactive PTY behavior is unchanged and the master-session launch gate remains closed.
- [ ] Mark the native-boundary design spec's status accepted and link this plan; do not change #610's eligibility or authorize a real child launch.
- [ ] Update #617 with the evidence and close it only after every acceptance criterion passes. If the exact candidate remains unavailable, leave it open with the measured no-go reason and do not claim qualification.
- [ ] Run `bash scripts/doc-check.sh`, `rtk git diff --check`, and the plan/spec coverage review. Run the Rust build/test/fmt commands from `crates/orkworksd/AGENTS.md` before declaring implementation complete.

## Self-review: spec coverage

| Spec requirement | Plan coverage |
| --- | --- |
| Exact Ubuntu/OpenCode/definition tuple and native host prerequisites | Tasks 1, 2, 6, 7 |
| Independent `native_boundary_proven` and `runner_eligible` gates | Tasks 2, 7 |
| Systemd generation owner, sidecar/owner crash, exact-unit reconciliation | Tasks 1, 3, 6 |
| Canonical worktree and Landlock filesystem policy | Task 4 |
| PID/mount/network namespaces, private `/proc`, same-user restrictions, capabilities, NNP | Tasks 4, 6 |
| Credential isolation, explicit env/FD policy, no host network/socket path | Tasks 4, 6 |
| Resource limits, cumulative CPU, output caps, termination proof | Tasks 3, 5, 6 |
| Native-only helper fixtures and no real coding harness | Tasks 1, 6 |
| Retained evidence, validation matrix, ADR 0060, issue #617 | Task 7 |
| No child authority or automatic runner launch | Global constraints and Task 7 |

**Placeholder scan:** No `TBD`, unbounded generic test-writing steps, or undeclared implementation symbols remain. Task 2 defines the contract types before later tasks consume them. The systemd mechanism and owner protocol are an explicit first-task ADR deliverable; production launch code is gated on that decision and the native prerequisite result.
