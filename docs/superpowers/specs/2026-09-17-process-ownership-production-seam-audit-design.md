# Process Ownership Production Seam Audit

Status: proposed design; no runtime implementation is authorized by this document
Date: 2026-09-17
Tracking: [issue #545](https://github.com/Rambolarsen/orkworks/issues/545)
Related: [ADR 0056](../../adr/0056-one-sidecar-per-open-workspace.md),
[multi-workspace specification](../../../specs/multi-workspace.md),
[process-ownership evidence](../evidence/2026-09-15-process-ownership-proof.md)

## Purpose

PR #568 proves the fixture protocol and native Windows Job Object behavior for
the scenarios it covers. It does not prove that every production child process
created by `orkworksd` enters a surviving owner boundary, nor that the boundary
survives sidecar failure. This design records the production seam inventory and
the evidence required before unavailable-runtime cleanup or multi-workspace
recovery can be implemented.

The design is deliberately an audit and mechanism-selection prerequisite. It
does not accept ADR 0056, change the sidecar launch path, or claim that the
current per-child Windows helper provides sidecar-wide ownership.

## Findings from the production inventory

The inventory distinguishes operating-system child processes from threads and
async tasks. `std::thread::spawn`, `tokio::spawn`, and Tokio task groups are
execution concurrency, not process-ownership seams.

| Production family | Current launch seam | Current containment/cleanup | Design gap |
| --- | --- | --- | --- |
| Native provider and CLI execution | `providers::ProcessRunner::run_prepared_with_spawn` | Unix process group; Windows per-child `ProcessJob`; bounded pipes and timeout | The sidecar owns the job/group. A sidecar crash does not establish a surviving owner for every child. |
| Custom inference | `providers::custom_inference::PreparedCustomInference::run_with_spawn` | Reuses `ProcessRunner` after a captured authorization check | The authorization check is not a crash-surviving ownership boundary. |
| Taskmaster inference | `taskmaster::runtime::inference` invokes the custom transport | Reuses custom inference and its runner | Inference cancellation/lease state is not proof that an OS descendant exited. |
| Provider model discovery | `ProviderManager::discover_models` | Direct child, bounded output and timeout; direct kill on timeout | Direct spawn is outside the sidecar-wide ownership protocol and must be classified or routed. |
| Codex app-server model discovery | `ProviderManager::discover_codex_models` | Direct child and bounded protocol read | Same ownership gap; the child may outlive the request and is not registered with a surviving owner. |
| Tool-version probing | `harness::detect::probe_tool_version` | Tokio child with `kill_on_drop`, bounded reads and outer timeout | Drop cleanup is local to the sidecar task; it is not evidence after sidecar failure. |
| Git and shell helpers | `plan_handoff`, session-application Git probes, and harness integration helpers | Short-lived synchronous children | They need an explicit policy: enter the owner seam, or be documented as bounded non-session helpers with proof that they cannot become persistent owned descendants. |

Tests and fixture-only subprocesses are excluded from the production inventory,
but their seams must remain usable as adapters for the native evidence harness.

## Proposed seam

The next implementation should expose one deep sidecar-facing ownership
interface for all child-process launches that can be associated with a workspace
runtime. Callers should provide role, workspace generation, executable intent,
and bounded lifetime policy; callers should not provide a PID, process name, or
path as an ownership claim.

The interface must hide platform-specific registration, handle retention,
identity capture, descendant enumeration, bounded termination, and complete-exit
acknowledgement. A caller should receive either an admitted child handle bound to
the generation or a fail-closed admission error. Cleanup should return a
classified result distinguishing confirmed empty ownership from unresolved
observation or termination.

The seam is a design target, not an approved API. Its implementation must be
selected only after the platform experiments below establish which owner can
outlive the sidecar:

1. Electron creates or retains the application-owner channel before a child can
   execute.
2. A surviving supervisor or OS-owned containment object receives only
   generation-bound, authenticated launch requests.
3. Registration failure prevents execution; an unregistered child is never
   treated as owned after the fact.
4. The surviving owner enumerates and terminates only its registered descendants
   and emits an authenticated complete-exit acknowledgement.
5. Relaunch adoption requires that acknowledgement for the old generation; a
   released metadata lease or persisted PID is insufficient.

## Platform candidates and evidence gates

No platform mechanism is selected by this document. The implementation plan
must evaluate candidates against the same interface and evidence contract.

- Windows candidates include a supervisor-retained Job Object with explicit
  owner-channel loss handling. The existing per-child Job helper is evidence for
  its fixture only and is not accepted as the sidecar-wide owner.
- macOS candidates include an OS-managed or detached supervisor arrangement
  whose ownership and cleanup survive sidecar death. A process group alone is
  not accepted without native evidence for descendants that daemonize or become
  reparented.
- Linux candidates include a portable supervisor or an OS-native scope where
  the supported environment provides one. Portable Linux runtime evidence is
  required; a CI-only assumption is not sufficient.

Each candidate must pass, on its native platform where applicable:

- registration failure before target execution;
- forged, stale, cross-generation and foreign-owner admission rejection;
- sidecar crash with PTY-like and inference-like descendants still alive;
- bounded cleanup of descendants without touching a foreign sentinel;
- injected observation and termination failures that remain unresolved;
- forced Electron termination with workspace A focused and B in the
  background, followed by immediate relaunch;
- no hidden B process or lease, no automatic session resume, and only the last
  focused A reopening;
- complete-exit acknowledgement before replacement generation adoption.

The evidence must record mechanism, owner lifetime, admission ordering,
identity/containment observation, cleanup deadlines, survivor behavior, and
known limitations. Unsupported or unrun rows remain explicitly unsupported.

## Scope boundaries

This design does not:

- implement a supervisor, Job Object, launchd integration, systemd scope, or
  Unix process-group change;
- alter Electron lifecycle, sidecar startup, workspace leases, or session
  restoration;
- infer ownership from PID, process name, executable path, or released locks;
- claim that direct model discovery, version probes, Git helpers, or shell
  helpers are safe after a sidecar crash;
- accept multi-workspace runtime behavior before ADR 0056 and issue #545 are
  satisfied.

## Exit criteria for the next plan

The implementation plan may proceed only when it names one owner mechanism per
supported platform, names the exact production spawn seam used by every family
above, and maps each required evidence row to a native fixture or an explicit
unsupported result. Any mechanism that cannot prove surviving ownership must be
returned to spec review rather than implemented as a best-effort cleanup path.
