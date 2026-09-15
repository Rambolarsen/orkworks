# Process Ownership Proof Design

Status: draft for issue #545; fixture and mechanism-selection work only

Tracking: [issue #545](https://github.com/Rambolarsen/orkworks/issues/545)

## Goal

Prove an ownership boundary that can clean every OrkWorks-owned sidecar,
session PTY, and inference descendant after the sidecar or Electron exits,
without touching a foreign sidecar or sentinel process. The proof must be
strong enough to support the unavailable-runtime cleanup and relaunch rules in
the proposed concurrent-workspace specification.

This work does not implement concurrent workspaces, runtime recovery, close,
quit, or replacement behavior. It produces a native fixture, records the
selected mechanisms and limitations, and updates the implementation plan only
after the evidence is complete.

## Contract under test

The owner boundary must satisfy all of these properties:

1. Electron establishes one owner/supervisor generation before any sidecar,
   PTY, or inference child is allowed to execute.
2. Every owned process is admitted through that boundary. A registration or
   containment failure fails closed before the target executes.
3. Owner-channel loss causes bounded cleanup of only the registered generation.
   Cleanup returns an explicit acknowledgement only after all owned processes
   have exited, or an unresolved-survivor result after the deadline.
4. A foreign sidecar, its metadata lease, and an unrelated sentinel remain
   alive throughout owned cleanup and cleanup failure.
5. A new generation cannot be adopted while the previous generation has an
   unresolved owned survivor.
6. The evidence covers a PTY-shaped child that creates a separate terminal
   session, an inference child, and descendants that outlive the sidecar.
7. The mechanism does not infer ownership from a persisted PID, executable
   name, working directory, or released metadata lease.

## Selected direction

Use a small native owner/supervisor process launched by Electron. Electron
retains the authenticated control channel and the supervisor owns the
generation's containment state. The supervisor, rather than the sidecar,
launches or authorizes every process root that can create a PTY or inference
descendant. The sidecar receives generation-bound launch capabilities and cannot
spawn an unregistered child.

The first implementation artifact is a disposable fixture with the same
control and failure transitions. Production wiring is a later task and cannot
be started until this fixture produces evidence on the required platforms.

### Windows hypothesis

The supervisor creates a private Windows Job Object with
`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` and without breakaway permissions. It
starts the sidecar in the job while suspended, assigns it to the job, and only
then resumes it. Children created by ordinary Win32 process creation inherit
the job; the supervisor retains the job handle after the sidecar exits. On
owner-channel loss it terminates the job, waits for the job to become signaled,
and reports the complete-exit acknowledgement.

The fixture must include a sidecar child that creates both a PTY-like process
and a nested inference process, and must attempt a breakaway path. A breakaway
that succeeds without an explicit supported boundary is a failed proof, not a
warning.

### macOS and portable Unix hypothesis

Do not treat one supervisor process group as sufficient. The existing
`portable-pty` Unix implementation calls `setsid()` for the PTY child, so the
PTY session is outside the supervisor's ordinary process group. The fixture
must therefore validate an explicit supervisor-owned launch/registration
boundary for each process root, with launch identity retained by the live
supervisor and cleanup performed through that boundary.

The experiment will compare:

- a supervisor-owned process-group/session boundary, including a PTY child that
  calls `setsid()` and a child that creates a new process group;
- a supervisor that launches and tracks each process root with native process
  identity and recursively reaps only its registered descendants; and
- the platform service mechanism available on macOS (`launchd`) as a control,
  including its process-group cleanup behavior.

The result must identify which mechanism, if any, survives the PTY/session
case. Linux may additionally use pidfds or cgroup-v2 evidence where available,
but Linux-specific strength cannot be substituted for the macOS result. If the
macOS fixture cannot prove ownership after `setsid()` or intentional
reparenting, the prerequisite remains unresolved and the limitation is brought
back to spec review.

## Fixture protocol

The fixture contains an owner, a sidecar-shaped child, owned PTY/inference
fixtures, and a foreign sentinel. It communicates over private pipes or local
sockets with length-delimited messages:

- `prepare(generation)` creates the ownership domain but launches no target;
- `spawn(role, command, identity)` registers and starts one owned process root;
- `ready(role, identity)` confirms that the target executed only after
  registration;
- `children(role, identities)` reports the descendants created by the fixture;
- `owner_lost` closes the Electron-like control channel;
- `cleanup(deadline)` requests bounded cleanup and returns either
  `acknowledged` with the complete owned identity set exited or
  `unresolved` with the surviving registered identities;
- `foreign_status` confirms that the foreign sidecar and sentinel remain
  alive; and
- `adopt(generation)` is rejected while the prior generation is unresolved.

Every process identity contains a generation, role, native process identity,
and launch acknowledgement. PIDs are diagnostic fields only; they are never
the authority for selecting a cleanup target.

## Evidence matrix

The fixture must pass these scenarios before mechanism selection is recorded:

| Scenario | Required result |
| --- | --- |
| Normal owner loss | Owned sidecar, PTY fixture, inference fixture, and nested descendants exit; foreign sentinel survives. |
| Sidecar crashes first | Supervisor still cleans the owned PTY and inference descendants. |
| Registration failure | Target does not execute and no owned descendant appears. |
| PTY creates a separate session/group | Cleanup still reaches the PTY-shaped tree, or the candidate mechanism is rejected. |
| Owned child ignores graceful cleanup | Bounded escalation reaches only the owned generation; survivor is reported if it remains. |
| Foreign owner holds the workspace lease | Foreign owner and sentinel heartbeat continue; only the attempted owned generation is cleaned. |
| Late obsolete-generation event | It cannot mutate the replacement or initiate cleanup of a different generation. |
| Immediate relaunch | Adoption waits for the previous supervisor's complete-exit acknowledgement. |
| Forced Electron termination | Owner-channel loss cleans all owned generations, including background ones; foreign sentinel survives. |

Run the fixture on Windows and macOS, plus portable Linux coverage. Record OS,
architecture, build mode, mechanism, test command, raw process identities,
cleanup latency, and any survivor. Do not record user credentials or private
prompts.

## Boundaries and limitations

- The owner boundary is process containment, not a filesystem sandbox. It does
  not make Git or workspace path operations atomic.
- A metadata lease is still required for single-writer workspace ownership,
  but it is not evidence that processes exited.
- Persisted PIDs are never used for adoption or cleanup.
- The fixture must distinguish an unavailable owner from a confirmed empty
  owner. Failure to observe liveness is unresolved, not zero children.
- No unavailable-runtime cleanup or automatic recovery may report success until
  the native evidence and the corresponding ADR 0056 update are complete.

## References

- [Concurrent workspaces](../../../specs/multi-workspace.md)
- [ADR 0056](../../adr/0056-one-sidecar-per-open-workspace.md)
- [Multi-workspace validation contract](../../validation/multi-workspace.md)
- [Windows Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects)
- [`launchd.plist(5)`](https://keith.github.io/xcode-man-pages/launchd.plist.5.html)
- [`PR_SET_PDEATHSIG(2)`](https://man7.org/linux/man-pages/man2/pr_set_pdeathsig.2const.html)
- [Linux cgroup v2](https://www.kernel.org/doc/html/latest/admin-guide/cgroup-v2.html)
