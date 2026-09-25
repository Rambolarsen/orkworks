# Master-session runner native confinement gate

**Status: closed to launch. No OS/harness combination is currently proven
eligible.** This is a no-go due to missing production-backed enforcement and
native evidence; it is not a report that the operating systems' candidate
primitives failed native tests.

This record applies to the hard launch gate in
[issue #610](https://github.com/Rambolarsen/orkworks/issues/610) and the
[master-session runner design](../superpowers/specs/2026-09-25-master-session-parallel-runner-design.md).
The needed production launch boundary and native proof are tracked in
[follow-up issue #617](https://github.com/Rambolarsen/orkworks/issues/617).
Until that work and its native fixtures pass, runner launches must remain
unavailable. Do not start runner implementation past this gate based on a
provider-only process owner, worktree `cwd`, harness prompt, hook, process
group, or asserted PID.

## Eligibility matrix

The current non-retired built-in interactive definitions are Claude Code
(`claude-code`), OpenCode (`opencode`), Codex (`codex`), Antigravity CLI
(`antigravity`), Aider (`aider`), GitHub Copilot CLI (`copilot`), and
`generic-shell`. Custom command-template harnesses use the same generic PTY
path and are also unqualified. Gemini (`gemini`) is retired and is not a
candidate for new launches.

| OS target | Harnesses considered | Current eligibility | Evidence status |
| --- | --- | --- | --- |
| macOS (native host audited: arm64) | Every non-retired built-in listed above; custom command-template harnesses | None | Source/entitlement audit only; no runner fixture or harness test run |
| Windows | Every non-retired built-in listed above; custom command-template harnesses | None | Native mechanisms researched; no production harness integration or runner fixture run |
| Linux | Every non-retired built-in listed above; custom command-template harnesses | None | Native mechanisms researched; no production harness integration or runner fixture run |

The table is deliberately conservative: it describes current OrkWorks
qualification, not the theoretical capabilities of an OS. A future pass must
name the exact OS version, architecture, harness ID and definition generation,
launcher/supervisor generation, fixture revision, command, and observed
results. Evidence for one row does not qualify another OS, harness, or
definition generation.

## Repository evidence

The generic interactive path in
`crates/orkworksd/src/runtime/session_runtime.rs` spawns through
`portable_pty`, sets the process working directory, and forwards the host
environment according to `should_forward_terminal_env` in
`crates/orkworksd/src/runtime/terminal_runtime.rs`. That filter allows `HOME`
and `ANTHROPIC_API_KEY`. A working directory is not filesystem confinement,
and the inherited home/environment does not establish credential isolation.

The current macOS release entitlements in
`apps/desktop/build/entitlements.mac.plist` and
`apps/desktop/build/entitlements.mac.inherit.plist` do not enable App Sandbox
or sandbox inheritance. The generic PTY child is not launched with a narrower
worktree-only sandbox.

The generic PTY cancellation path signals/kills the root child; it does not
provide a generation-bound census or proof that every descendant has exited.
The Windows Job Object in
`crates/orkworksd/src/providers/windows_process.rs` is scoped to provider
inference. It is not attached to interactive harness launches and currently
sets kill-on-close, not the runner's complete confinement/resource policy.
Existing Windows provider fixture results in
[ADR 0056](../adr/0056-one-sidecar-per-open-workspace.md) remain fixture
evidence for that provider path only.

The existing PTY runtime also does not provide the runner's required
server-observed successful completion contract. A normal harness completion
must be distinguished from timeout, cancellation, forced termination, and an
exit with surviving descendants; each eligible harness generation needs a
native clean-exit fixture.

## Platform research, not qualification

- **macOS:** App Sandbox is an OS-enforced control, and Apple documents child
  sandbox inheritance, but OrkWorks' current release entitlements do not enable
  it. No per-worktree sandbox, aggregate resource ceiling, credential boundary,
  or crash-surviving generation owner has been integrated and tested for the
  interactive PTY path. See [Apple App Sandbox](https://developer.apple.com/documentation/security/protecting-user-data-with-app-sandbox)
  and [sandbox inheritance](https://developer.apple.com/library/archive/documentation/Miscellaneous/Reference/EntitlementKeyReference/Chapters/EnablingAppSandbox.html).
- **Windows:** AppContainer and Job Objects are candidate primitives for
  isolation, process ownership, and resource controls. The provider-only Job
  Object does not cover a harness PTY; the sandbox launch, PTY compatibility,
  credential boundary, and successful completion path remain unintegrated and
  untested. See Microsoft's [AppContainer overview](https://learn.microsoft.com/en-us/windows/win32/secauthz/appcontainer-isolation)
  and [Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects).
- **Linux:** Landlock and cgroup v2/systemd are candidate primitives, subject
  to kernel ABI, delegated-controller, service-manager, descriptor, and
  credential-socket constraints. They are not used by the PTY harness path.
  A cgroup CPU rate quota is not itself a cumulative CPU-time limit. See the
  [Landlock documentation](https://docs.kernel.org/userspace-api/landlock.html)
  and [cgroup v2 documentation](https://docs.kernel.org/admin-guide/cgroup-v2.html).

These sources establish candidate mechanisms only. They do not substitute for
native tests through the production child-launch boundary.

## Required proof before reopening the gate

Follow-up [#617](https://github.com/Rambolarsen/orkworks/issues/617) must
produce a production-backed native launch boundary and bounded helper fixtures
that exercise it. The fixtures must verify canonical worktree allow/deny
behavior (including symlinks/reparse points and Git worktree metadata),
descendant ownership, aggregate ceilings, cancellation and owner-crash
termination, fake home/keychain credential isolation, successful completion
without `Kill`, and foreign sentinel survival. Run them natively for each
candidate OS/harness pair. If any prerequisite cannot be enforced or any
result is ambiguous, that exact pair remains unavailable; no evidence may be
generalized across pairs.

No adversarial fixture, resource-exhaustion test, or real coding harness was
run for this checkpoint. That remains outstanding work, not a passing test.
