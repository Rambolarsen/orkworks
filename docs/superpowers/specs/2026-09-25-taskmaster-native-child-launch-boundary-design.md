# Taskmaster native child-launch boundary

- Status: proposed
- Date: 2026-09-25
- Tracking issue: [#617](https://github.com/Rambolarsen/orkworks/issues/617)
- Parent gate: [#610](https://github.com/Rambolarsen/orkworks/issues/610)
- Related design: [master-session parallel runner](2026-09-25-master-session-parallel-runner-design.md)
- Related decision: [ADR 0060](../../adr/0060-independent-workspace-instances.md)
- Related decision: [ADR 0064](../../adr/0064-bounded-taskmaster-coordinator.md)

## Purpose

Define the smallest production-backed native process boundary that can be
proven for one exact OS and harness version. This work addresses the launch
confinement prerequisite in #617. It does not authorize or enable master
session child launches. The runner remains closed until its separate broker,
model-request, approval, lifecycle, and clean-completion requirements in #610
are implemented and reviewed.

The current interactive PTY path sets a working directory and forwards parts
of the host environment. It does not confine the child to its working tree,
own its whole descendant tree, isolate credentials, or prove bounded cleanup.
The existing provider Job Object applies to inference processes, not harness
launches. See the current [native confinement record](../../validation/master-session-runner-confinement.md).

## Decision proposal

### Exact candidate slice

Start native implementation and proof for:

- **OS:** Ubuntu 24.04 LTS, x86_64, native GitHub Actions runner. At design
  time the published `ubuntu-24.04` image version `20260920.314.1` reports
  kernel `6.17.0-1022-azure` and systemd `255.4-1ubuntu8.17`; each CI run
  must record its resolved image and runtime feature probes
  ([runner image inventory](https://github.com/actions/runner-images/blob/main/images/ubuntu/Ubuntu2404-Readme.md)).
- **Host prerequisites:** cgroup v2 with delegated required controllers,
  namespaces creatable without host privilege elevation, seccomp filter
  support, and Landlock ABI 6 or newer. The runtime probes each prerequisite;
  Ubuntu 24.04 hosts that do not expose the required features remain
  unavailable even when their distribution label matches.
- **Harness:** OpenCode CLI `v1.18.18`, with the OrkWorks integration API
  generation pinned separately to `@opencode-ai/plugin@1.18.18` and
  `@opencode-ai/sdk@1.18.18`.
- **Harness definition:** the code-owned built-in `opencode` definition at an
  exact generation and content digest captured by the native fixture. That
  generation and digest must be pinned in the implementation and CI record
  before this tuple may be marked `native_boundary_proven`.
- **Production boundary:** a Rust child-launch adapter used by the runner
  launch path, with a generation-specific systemd user service as its durable
  owner and Linux kernel controls applied before the harness process executes.

The OpenCode release tag and the repo's plugin/SDK type declarations are
separate version facts; matching version strings do not prove a compatible
tool-mediation contract. The repo's current OpenCode integration reports
session and attention events only. It does not broker tool invocations. The
OpenCode CLI release is pinned to the immutable `v1.18.18` tag; its release
was published on 2026-08-13 ([release record](https://github.com/anomalyco/opencode/releases/tag/v1.18.18)).

This is a **candidate for native-boundary proof**, not a supported automation
pair. The application must not expose a launch action or start a real OpenCode
coding session after this boundary alone passes. Other OS, architecture,
harness, version, or definition generations remain unqualified.

### Two independent gates

Persist and present qualification as two independent facts:

| Gate | Meaning | Required evidence |
| --- | --- | --- |
| `native_boundary_proven` | The exact adapter/OS/harness-definition tuple constrains files, resources, credentials, descendants, cancellation, and owner crash as specified below. | Native helper fixture results through the production adapter, exact runner image/kernel/systemd versions, adapter and fixture revisions, commands, and retained machine-readable output. |
| `runner_eligible` | A user-approved #610 plan can safely execute with that tuple. | `native_boundary_proven`, plus the reviewed immutable-plan gate, server-owned tool and model broker, finite broker budgets, provider credentials retained broker-side and never exposed to the child, authenticated child runtime/report channel, harness clean-exit handshake, and all other #610 requirements. |

`runner_eligible` is false unless every prerequisite passes. A native boundary
pass cannot set it true by itself. UI availability, API launch authorization,
and persisted capability issuance must all consult the latter gate.

## Scope

This spec covers the child process boundary and its native fixture suite. It
does not design or implement the full coordinator or allow actual child work.
The boundary must expose a narrow launch/terminate/status interface so that a
later broker can use it without directly spawning or signaling harness
processes.

Out of scope:

- implementing the #610 coordinator, user approval flow, tool broker, or
  model/provider broker;
- launching a real coding harness in CI or in native fixtures;
- enabling network access from the child before brokered model transport
  exists;
- qualifying another OS, architecture, harness, or harness generation;
- inferring success from a process exit, hook, child report, or kill request;
- adding general interactive-session sandboxing or changing normal PTY
  behavior.

## Proposed boundary

### Ownership and generation lifecycle

The production Rust adapter requests a uniquely named transient systemd user
service for each launch generation. A narrow native owner process runs inside
that service and is the only component permitted to create the harness child.
The user service manager owns the unit and its control group independently of
the OrkWorks sidecar process. The owner accepts one launch request bound to an
unpredictable generation ID, a canonical worktree identity, the immutable
harness definition identity, and finite limits. It rejects a duplicate or
stale generation and never accepts a replacement command over an existing
generation.

The owner monitors an authenticated local control channel to the sidecar.
Channel closure, missed bounded heartbeat, explicit cancellation, deadline,
limit breach, owner failure, or sidecar crash fences the generation and asks
systemd to terminate the unit's complete control group. The adapter then waits
for both an inactive unit and an empty generation cgroup. Until both are
observed, status is `termination_unproven`, the generation remains reserved,
and no retry or replacement may launch. The owner service uses control-group
termination on stop, so an owner crash does not leave the harness descendants
running. Restart reconciliation looks up only the exact recorded unit and
generation; it never scans for or adopts a process based on PID or command
line.

Creating the service, owner, and native policy is one fail-closed operation.
The adapter must establish all restrictions and verify the assigned cgroup
before `exec` of the target. Any unsupported user manager, missing controller,
permission error, cgroup mismatch, policy application error, owner handshake
failure, or ambiguous response prevents launch. A sidecar restart may
reconcile a known generation, but cannot relaunch it.

### Filesystem and working-tree boundary

The adapter takes a stable directory handle for the assigned worktree and
derives policy from that handle, not a caller-provided path string. It
canonicalizes and verifies repository/worktree identity before allocation;
rejects symlink or reparse escapes, mount/path aliases, the primary checkout,
and any target whose ownership is ambiguous. It then applies a Landlock
policy in the owner before executing the child:

- write access is limited to the assigned worktree and private scratch;
- read/execute access is limited to the worktree plus explicitly required,
  immutable runtime locations and the OpenCode executable/runtime;
- read-only access to the generation's fresh `/proc` mount is permitted for
  process observation inside its private PID namespace; no host `/proc` mount
  or host process view is exposed, and the mount provides no write or execute
  access;
- the primary checkout, sibling worktrees, home directories, credential
  stores, OrkWorks metadata, and unrelated files are denied;
- Git administrative metadata, including the linked worktree's `.git` target
  and common repository metadata, is read-only to the child;
- inherited file descriptors are closed except for a minimal explicit
  stdio/control allowlist; no pre-opened file, socket, PTY master, keyring, or
  agent connection may bypass path policy.

Landlock applies to the enforcing process and its future children, but open
descriptors created before enforcement remain relevant. The owner must
therefore construct its descriptor allowlist before launch and enforce
Landlock before opening the harness executable or any allowed runtime file.
The Landlock ABI is detected at runtime. The exact rights needed by this
policy are mandatory; a kernel that lacks any required right is unavailable,
not a weaker pass. The kernel documentation describes filesystem and network
rules, inheritance by future children, ABI probing, and the limits of
descriptor-based enforcement ([Landlock documentation](https://docs.kernel.org/userspace-api/landlock.html)).

The allocator rejects worktrees containing regular files with multiple hard
links unless implementation can prove those links are wholly contained in the
assigned tree. The fixture suite includes an outside hard-link sentinel and
must prove that the boundary neither mutates nor permits creating an escape
link. Symlink swaps, hard links, Git `commondir`, `.git` file indirection,
bind mounts, and path replacement during policy setup are adversarial cases,
not ordinary-path-only checks.

### Network and credentials

The native boundary denies child network access. Landlock's port rules alone
are not treated as a full network sandbox or destination allowlist. The owner
must prevent inherited connected sockets and host Unix sockets, and must
place the child in an isolated network namespace with no external route or
host service access. Until the #610 server-owned model broker exists, no
provider connection is permitted. This means this prerequisite does not
demonstrate that OpenCode can perform useful coding work; it demonstrates only
that the production launch boundary contains the designated child.

The child receives an empty private home and private XDG config/data/cache
directories under generation-owned scratch. The launcher passes a closed,
explicit environment allowlist; it does not inherit `HOME`, provider API keys,
`ANTHROPIC_API_KEY`, `CODEX_HOME`, OpenCode auth files, SSH agent variables,
cloud credentials, proxy credentials, or arbitrary caller environment. The
fixture supplies fake home, keyring, agent socket, and credential sentinels
and proves the helper cannot read or use them. Test credentials are never real
user credentials.

Later broker integration must provide scoped model access without placing
provider secrets in the child environment or private home. It must also prove
that every OpenCode tool and model path is mediated and cannot bypass the
server-owned broker. The repo's pinned OpenCode plugin contract is presently
session/event reporting only; plugin hooks and custom tools do not, by
themselves, prove that every built-in tool path is brokered
([OpenCode plugin docs](https://opencode.ai/docs/plugins/),
[repo harness contract](../../agents/harness-integration-contracts.md)).

### Same-user process isolation

The owner and the child run under the desktop user's account, so filesystem
and environment isolation alone are insufficient. Before child `exec`, the
launcher must place the child in a private PID and mount namespace and mount a
fresh `/proc` instance for that PID namespace. It must establish these
namespaces without host privilege elevation, using an unprivileged user
namespace where required. If user-namespace creation is disabled or the
required mount cannot be established, launch fails closed. It must also apply
Landlock in the child process so Landlock's domain hierarchy restricts `ptrace` and
related process-inspection operations, and enable `LANDLOCK_SCOPE_SIGNAL` so
the child cannot signal the owner, sidecar, or unrelated same-user processes.
The owner remains inside the generation's private PID/mount namespace but
outside the child's Landlock domain; the sidecar remains outside the private
PID namespace. The child and its descendants inherit the restriction.
Namespace creation, private `/proc`, Landlock process scoping, and dropped
capabilities are mandatory parts of the production adapter; if the host cannot
establish them, launch fails closed. Namespaces provide additional process-view isolation;
Landlock and cgroup enforcement remain mandatory controls rather than being
replaced by namespaces.

The Landlock feature probe must include `LANDLOCK_SCOPE_SIGNAL` (ABI 6 or
newer), plus the filesystem rights required by the policy. The native target
does not require ABI-9 `LANDLOCK_ACCESS_FS_RESOLVE_UNIX`: host pathname sockets
are excluded from a private mount tree before Landlock is applied. The
launcher enters a new private root assembled from declared runtime mounts,
the assigned worktree, private scratch, a fresh `/proc` for the generation
PID namespace, and a minimal private `/dev`; it contains no host `/run`,
`/tmp`, home, keyring, agent, or sidecar socket paths. Existing pathname
socket nodes in exposed runtime or worktree inputs are rejected, but this scan
is defense in depth: the worktree remains host-backed and may change after
the scan. No host directory or socket descriptor is inherited by the child.
A seccomp filter installed after namespace and Landlock setup and before
`exec` denies `socket`,
`socketpair`, `connect`, `bind`, `listen`, `accept`, `accept4`, `sendto`,
`recvfrom`, `sendmsg`, `recvmsg`, `sendmmsg`, and `recvmmsg` so a post-setup
pathname socket cannot be reached and the child cannot create a socket server
reachable from the host. The filter also denies `io_uring_setup`,
`io_uring_enter`, and `io_uring_register` so socket operations cannot bypass
syscall checks through io_uring. It is architecture-aware and rejects
unsupported compat syscall entrypoints. The child receives only explicit
pipe/PTY descriptors. Any future broker channel must use a separately scoped
inherited pipe, not a socket exception. The seccomp filter is one kernel
control within the layered boundary, and its restrictions must be inherited by
every child and exec'd descendant ([seccomp filter documentation](https://docs.kernel.org/userspace-api/seccomp_filter.html)).
If seccomp, the required ABI-6 controls, namespace setup, or mount-tree policy
are unavailable, launch fails closed.

The launcher uses this fail-closed bootstrap order: create the generation
cgroup; establish the required user, PID, mount, and network namespaces; build
and enter the private mount root; set up the fresh `/proc`; prepare the exact
inherited-FD allowlist; drop every bounding, effective, permitted, inheritable,
and ambient capability; set `PR_SET_NO_NEW_PRIVS=1`; enforce the required
Landlock rules; open the executable and required runtime files under that
policy; install the architecture-checked seccomp filter; verify the final
security state; then `exec` the target. Namespace and mount setup must finish
before capability dropping and `no_new_privs` so unprivileged namespace setup
cannot depend on any later privilege gain. All setup occurs without host
privilege elevation. Any failure or failed verification aborts the launch
before `exec`. `no_new_privs` is inherited across fork, clone, and exec and
prevents setuid/setgid bits and file capabilities from granting privilege at
exec; it is also required to install
seccomp filters without `CAP_SYS_ADMIN` in the process namespace
([kernel no_new_privs documentation](https://docs.kernel.org/userspace-api/no_new_privs.html),
[seccomp filter documentation](https://docs.kernel.org/userspace-api/seccomp_filter.html)).
The capability check covers every capability set in the child user namespace;
the child receives no capability exception for the adapter's setup needs.

The runner-image inventory currently reports kernel 6.17, while ABI 9's
pathname Unix-socket right maps to Linux 7.1 in the Landlock ABI reference;
therefore that right cannot be a prerequisite for this candidate. Seccomp
syscall denial plus an empty inherited socket-FD set closes the pathname
socket race without relying on ABI 9. The kernel ABI is probed directly at
runtime rather than inferred from the version string. Ptrace restrictions
follow Landlock domain hierarchy; native tests must prove same-UID denial
separately from PID-namespace hiding
([Landlock process scoping](https://docs.kernel.org/userspace-api/landlock.html),
[Landlock ABI/kernel mapping](https://man7.org/linux/man-pages/man7/landlock.7.html),
[`/proc` PID namespace support](https://docs.kernel.org/filesystems/proc.html)).

The adapter must not grant the child `CAP_SYS_PTRACE`, `CAP_KILL` in the host
user namespace, or a `PR_SET_PTRACER` exception that weakens this boundary.
The private `/proc` view must not expose the sidecar or processes outside the
generation. Every required namespace, mount, Landlock, capability, and
process-inspection control is checked before `exec`; an unavailable control
rejects launch.

### Resource ceilings

The native adapter enforces limits at the generation cgroup, not per PID:

- finite wall-clock deadline, independently enforced by systemd and the
  generation owner;
- memory maximum and swap policy, with the selected values bound to the
  approved launch request;
- process-count maximum;
- CPU rate ceiling plus cumulative CPU accounting, with an owner-enforced
  total CPU ceiling and a measured, documented maximum overshoot;
- bounded stdout, stderr, terminal/event output, and retained logs; exceeding
  any bound terminates the complete generation.

systemd/cgroup controls (`MemoryMax`, `TasksMax`, CPU controls and cgroup
accounting) are candidate mechanisms, not yet production evidence
([systemd resource controls](https://www.freedesktop.org/software/systemd/man/latest/systemd.resource-control.html),
[Linux cgroup v2](https://docs.kernel.org/admin-guide/cgroup-v2.html)).
`CPUQuota` is a rate limit, not a cumulative CPU-time budget; implementation
must read aggregate cgroup CPU usage and enforce the total with a bounded
sampling interval. Token, cost, and tool-invocation ceilings belong to the
future server-owned broker and are not proven by this native launch work.

## Native fixture suite

Fixtures invoke the same production adapter and policy builder as a future
runner launch. They use a purpose-built native helper, never OpenCode or any
real coding tool. The helper reports structured observations to the fixture
controller over an explicitly allowed pipe descriptor; CI retains exact
machine-readable results and the environment fingerprint.

The required cases are:

1. **Clean boundary success:** helper reads an allowed runtime file, writes a
   file within the assigned worktree and private scratch, and exits zero; the
   adapter observes normal helper exit and an empty cgroup. This proves
   adapter operation, not successful OpenCode task completion.
2. **Path denial:** reads and writes outside the worktree are denied,
   including primary checkout, sibling worktree, home, `/tmp` outside private
   scratch, OrkWorks metadata, Git common metadata writes, absolute-path
   escapes, `..`, symlink swaps, bind-mount aliases, `.git`/`commondir`, and
   hard-link sentinel cases.
3. **Descendant ownership:** helper forks descendants and attempts a detached
   child; the cgroup and policy apply to all descendants, and termination
   leaves no descendant alive.
4. **Resource limits:** aggregate memory, process count, wall time, cumulative
   CPU, and output bounds trigger the expected reason and whole-generation
   termination. CPU overshoot is reported and stays within the implementation
   contract.
5. **Credential isolation:** fake home files, fake API keys, fake keychain,
   SSH agent and cloud credential sockets, and inherited host descriptors are
   inaccessible; no fake credential use reaches the sentinel receiver.
6. **Same-user process isolation:** use two distinct sentinels. A same-UID
   owner sentinel runs inside the private PID namespace, outside the helper's
   Landlock domain, and is visible in the helper's read-only private `/proc`;
   `/proc` sensitive reads, `ptrace`, `process_vm_readv`/`process_vm_writev`,
   `pidfd_getfd`, and signal attempts against it must be denied by the
   process-access policy.
   A second same-UID sidecar sentinel stays outside the private PID namespace;
   it is absent from the helper's `/proc` and cannot be signaled or inspected
   from the helper. Both sentinels remain alive and unchanged. Access to
   processes within the helper's own generation remains contained to that
   generation. The helper reports `NoNewPrivs: 1` from `/proc/self/status`,
   empty effective, permitted, inheritable, ambient, and bounding capability
   sets, and denial of privilege gain from a fixture-only setuid or
   file-capability executable. The helper cannot weaken Landlock or gain
   host-user-namespace capabilities.
7. **Socket and network denial:** attempts to create/connect/listen/accept
   AF_UNIX, AF_INET, or AF_INET6 sockets fail under the production seccomp
   filter; io_uring setup/entry also fails. A same-UID host helper creates a
   pathname Unix socket inside the worktree *after* policy setup, and the
   child still cannot connect to it. Conversely, the child cannot create a
   socket in the worktree for the host helper to connect to. The generation's
   network namespace has no external route or host loopback access, and no
   socket FD was inherited.
8. **Cancellation:** cancellation at launch, during helper work, and during
   descendant work revokes the generation and proves an empty cgroup before
   releasing its reservation.
9. **Forced termination is not success:** force-kill the helper after it has
   reported apparent completion and during active work; both outcomes remain
   failed/termination states, never a clean-exit receipt, until the adapter
   observes the required exit and empty-cgroup conditions.
10. **Owner/sidecar crash:** kill the sidecar and separately kill the owner
   while a descendant runs; systemd reaps the entire unit and reconciliation
   proves empty cgroup without duplicate launch.
11. **Foreign sentinel survival:** processes and files outside the generation
   remain unchanged and alive after clean exit, every limit breach, cancel,
   and crash.
12. **Fail-closed setup:** absent user manager, missing cgroup controller or
    Landlock right, insufficient delegation, malformed paths, stale
    generation, policy-install failure, and uncertain systemd responses all
    reject before helper execution or retain an orphaned reservation until
    reconciled.

Use Linux namespace user-mode/kernel capabilities only where the production
path itself uses them. Tests must run on the exact native CI image; mocked
systemd/cgroup or Landlock implementations are unit tests only and cannot
qualify the slice. No privileged escape-hatch, host bind mount, or relaxed
fallback may turn an unsupported host green.

## Evidence and status record

For each candidate tuple, record:

- OS image, kernel release/configuration, architecture, systemd version, and
  detected cgroup/Landlock ABI/controllers;
- OpenCode CLI version and executable digest; plugin and SDK package versions
  separately; exact code-owned built-in harness definition generation and
  digest (missing or changed definition identity prevents qualification);
- production adapter revision, helper/fixture revision, exact command and
  native CI job URL;
- per-case pass/fail result, termination reason, cgroup-empty observation,
  maximum CPU overshoot, and foreign-sentinel result;
- unresolved limitations and the qualification fields updated.

A missing result, skipped fixture, opaque cleanup, green mocked test,
unsupported kernel, changed harness generation, or ambiguous owner state is a
failure for that exact tuple. The validation matrix must distinguish
`native_boundary_proven` from `runner_eligible`; neither an issue closure nor
an ADR amendment may conflate them.

After the complete native evidence is reviewed, update #617, the validation
record, and ADR 0060 with the exact proof and remaining limits. Reopen #610's
launch gate only after the broker, approval, process-result, and clean-exit
requirements have their own reviewed implementation and evidence. No design
or native boundary pass alone opens that gate.

## Current facts, inferences, and open questions

**Facts:** The current PTY path is not runner confinement; provider process
ownership fixtures do not cover interactive harness launches. The repo's
OpenCode plugin integration is event reporting, not complete tool mediation.
Landlock enforcement is inherited by future children, but pre-opened
descriptors matter. A cgroup CPU rate limit does not cap total CPU consumed.

**Inference:** Ubuntu 24.04 x86_64 with OpenCode `v1.18.18` is a reasonable
first implementation candidate because the repo already pins the matching
OpenCode plugin/SDK type generation and Ubuntu offers the proposed Linux
primitives. This selection is for bounded proof work only; it is not a claim
that the CLI can run safely under this boundary or that the broker contract is
solved.

**Open questions for the implementation plan:**

- Can the native CI image provide a user systemd manager with the required
  delegated cgroup v2 controllers without privileged setup? If not, choose a
  different Linux owner primitive before implementation; do not add a weaker
  path.
- Which exact Landlock ABI-6 filesystem/signal rights and network namespace
  setup are required for the supported kernel floor and all descendants?
- How will systemd's owner/unit result plus the cgroup population prove
  generation emptiness across owner, sidecar, and user-manager failure?
- What CPU sampling/termination interval gives a defensible cumulative budget
  and maximum overshoot under aggregate parallel CPU use?
- Does the #610 broker design provide a complete OpenCode built-in tool and
  provider mediation seam? If not, OpenCode remains `native_boundary_proven`
  at most and never `runner_eligible`.
- What exact OpenCode CLI binary digest and package distribution will CI
  install for `v1.18.18`, independently from the plugin/SDK npm package
  versions?

These are implementation proof questions, not permission to expand the OS or
harness scope. Any answer requiring a second OS/harness, broader network,
host credentials, or a relaxed policy returns to design review.
