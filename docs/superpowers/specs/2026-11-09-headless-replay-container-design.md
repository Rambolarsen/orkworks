---
type: Design Proposal
title: Headless replay fixture container
description: Proposed isolated image and bounded host runner for the existing rendered replay scenario.
tags: [orkworks, testing, containers, electron]
status: draft
workflow_status: proposed
---

# Headless replay fixture container

## Outcome and approval boundary

An agent can run one existing real Electron replay fixture without a host
display, recover screenshots and diagnostics as its own host user, and leave no
running test-owned container. Success means the recorded-grid, resize and
hidden-recovery assertions pass in the selected qualified runtime, with useful
failure/timeout evidence and unchanged host source ownership.

This is a proposal for [#95](https://github.com/Rambolarsen/orkworks/issues/95),
not implemented or approved runtime behavior. The user authorized this design
phase. The image/security/execution decisions below are the author's proposal;
owner approval is required before dependent implementation. Merging this
proposal alone does not grant that approval. One approval of this concrete
contract is sufficient; do not request a second plan approval for unchanged
scope.

## Existing contracts and evidence

- [The desktop fixture guide](https://github.com/Rambolarsen/orkworks/blob/main/apps/desktop/AGENTS.md#rendered-historical-replay-fixture)
  documents `app-review-replay`: real App, TerminalPanel, HistoricalTerminal,
  xterm DOM rendering and observers; synthetic backend, bridge and WebSocket.
- [PR #1634](https://github.com/Rambolarsen/orkworks/pull/1634) added the
  fixture, temporary Electron profiles, PNG/JSON snapshots and subprocess log.
  It retains the existing Linux-only `--no-sandbox` fixture argument.
- [PR #1639](https://github.com/Rambolarsen/orkworks/pull/1639) aligned the
  toolchain prewarm with the desktop's pnpm 11.24.0 pin. Its actual Linux arm64
  frozen-install/loader audit found absent Xvfb and 25 unresolved Electron
  45.2.1 libraries. That audit did not qualify a headless fixture run.
- [The current container guide](../../agents/project-context.md#containerized-development-environment-optional)
  describes the toolchain-only `dev` service, host source bind and named caches.
  Preserve that service and its caches; this proposal does not qualify or fix
  its existing bind-mount/UID behavior.
- [ADR 0009](../../adr/0009-electron-security-posture.md) continues to govern
  production Electron isolation. No production sandbox, preload, main/renderer
  boundary, [runtime contract](../../agents/runtime-contracts.md), or
  [release pipeline](../../../specs/release-pipeline.md) changes are proposed.

## Separate image, existing fixture

Add a `headless-replay` stage derived from `dev` in the root Containerfile.
Keep `dev`, `sonar`, the default Compose target and native development unchanged.
The added stage is an explicit image snapshot of this fixture, rather than a
new general-purpose development container or live bind-mounted app.

Prepare dependencies during the image build:

- Start with the measured Bookworm runtime closure: `xvfb`, `xauth`,
  `libgtk-3-0`, `libnss3`, `libasound2`, `libgbm1`, and `fonts-dejavu-core`.
  This is a candidate apt set, not an already verified minimal package list.
  Qualify the transitive closure against `ldd` and the actual locked Electron;
  add a package only for a demonstrated missing dependency.
- Copy only desktop package/lock/workspace manifests, patches and required
  install scripts, then perform the frozen pnpm install and the locked
  package's explicit Electron binary installation. Do not assume a successful
  pnpm install downloaded Electron. Preserve the xterm patch and overrides.
- Copy renderer `src/` and only the existing application-shell test plus any
  assets it actually requires, using explicit source paths. Do not copy `.git`,
  `.env` files, local state, host dependencies, credentials or the repository
  root wholesale. Build context exclusions remain in force.
- Store source/dependencies under `/opt/replay`, owned by root and readable
  but not writable by the runtime user. Cache image layers by manifest/lock
  inputs; do not replace, chown, prune or share the existing dev named caches.

The image's fixed command invokes Node 22 directly on
`tests/applicationShell.test.mjs` with
`--experimental-strip-types --test --test-name-pattern='fixed shell: app-review-replay$'`.
No pnpm install, download, Vite server, sidecar, PTY or coding harness runs during
the fixture. The fixture bundles with esbuild in its temporary directory. This
path does not require or qualify a packaged desktop build or Electron fs-ext
ABI repair; it does not load the Electron main-process native modules.

## Test-only execution and security boundary

Run the image as the existing non-root `node` user (UID/GID 1000), using the
engine's default seccomp policy, `--cap-drop=ALL`, `no-new-privileges`,
`--network=none` and `--init`. Initial limits are 2 GiB memory, 256 PIDs and
256 MiB shared memory; qualify them with the actual fixture. Do not silently
remove a limit or a security option to get a pass.

The supported host command has no arbitrary command/volume/privilege pass-through.
The input image is built only from trusted repository source and its frozen
dependencies; this is not a service for executing arbitrary repositories.
It forwards no host home, environment credentials, DISPLAY/socket, device,
source bind, Docker/Podman socket, SSH agent or additional capability. Only
fixture-specific image environment is set; use container-local HOME/TMP and
`ORKWORKS_REPLAY_ARTIFACT_DIR=/artifacts/replay`.

Retain the existing Linux fixture-only Chromium `--no-sandbox` argument in this
isolated path. It is not evidence of production renderer security or permission
to run the production app, arbitrary websites, coding tools or untrusted
repositories with that argument. Record this narrow developer-test exception in
an ADR before code implementation, once approved; do not supersede ADR 0009's
production decision. If the fixture needs privileged mode, host IPC/display,
seccomp disabling or additional capabilities, stop and bring back the concrete
incompatibility instead of weakening this contract.

Use an ordinary disposable writable container layer, not `--read-only`, so
`/artifacts/replay` (created at build time for UID/GID 1000) survives process
exit for copying. No persistent data volume or root initializer is needed.
Source/dependencies remain protected by ownership. The container layer and its
local profiles disappear when that exact container is removed. This deliberate
choice avoids a tmpfs artifact disappearing before export and avoids introducing
volume chown/UID mapping to the first headless path.

## Host ownership, artifacts and cleanup

Add one fixture-specific Bash host command, `scripts/headless-replay.sh`, with
an explicit `--engine docker|podman`. It builds the target, runs the fixed
fixture, exports its evidence, reports exit/timeout/cleanup status and prints
the artifact directory. No engine auto-switching, daemon startup, sudo, global
configuration change, generic container executor or plugin is needed.

Create a new private host temporary directory for each invocation. Never use
an existing directory as a cleanup target. Build/create/run commands have a
15-minute build budget, 30-second create budget and 90-second fixture budget;
export and cleanup each get 30 seconds. The fixture retains its internal
5-second readiness and 30-second Electron subprocess bounds.

The wrapper owns a private CID file populated by its own create command and
operates only on that validated container ID, never process names, unrelated
containers, broad labels or global prune. Use engine stop with a 5-second grace,
then exact-container kill if necessary. On success, assertion failure, timeout
or interruption, attempt bounded evidence export and removal of that container.
If the engine becomes unreachable or creation/cleanup cannot be settled, exit
nonzero and report the exact CID/private CID-file path and last confirmed state;
do not claim cleanup or background monitoring. Never delete a guessed resource.
Image cache layers may remain for reuse; no global image/cache cleanup occurs.

Wait until the container is stopped before export. Copy only the known replay
PNG/JSON names and subprocess log, without archive ownership preservation or
symlink following, into the newly created host directory. Reject links and
non-regular files; verify caller UID/GID before reporting success. Never use
sudo or recursively chown the source tree or a user-supplied path. Docker's
[copy documentation](https://docs.docker.com/reference/cli/docker/container/cp/)
specifies caller ownership for normal host copy; Podman's
[copy semantics](https://docs.podman.io/en/latest/markdown/podman-cp.1.html)
must be qualified independently. Do not infer host ownership from container UID.

Bound export to the fixture's five normal PNG/JSON pairs, optional failure pair
and subprocess log. Enforce limits before/during transfer rather than copying
unbounded data and checking afterward: at most 13 files, 8 MiB per PNG, 1 MiB per JSON, 2 MiB for
the subprocess log, and 64 MiB total. Host engine/test output has a separate
2 MiB retained-log cap with an explicit truncation marker. Treat report content
as data, never a command. Startup failure or forced timeout can lack screenshots;
retain the host log and explicit reason rather than inventing a passing report.

## Qualification matrix and limits

| Profile | Required evidence before claiming support |
| --- | --- |
| Linux amd64, rootful Docker daemon, non-root host CLI user | Real offline replay success, PNG inspection, failure/timeout/interrupt cleanup, source hash/ownership unchanged, exported artifacts owned by caller. Primary CI candidate. |
| Linux amd64, native rootless Podman | The same checks plus actual rootless-mode evidence and artifact ownership; default namespace/security options must work without host reconfiguration. |
| macOS arm64, Podman Linux VM | Useful local qualification of this distinct VM profile; record actual engine/rootless mode. It does not establish native Linux rootless behavior or native macOS app behavior. |

These are candidate profiles, not current support claims. The first implementation
may publish only profiles actually qualified. An unavailable/refused profile is
reported as unqualified with its reason; it does not justify a weaker security
profile or a claim that #95 is complete. Docker Desktop, Windows/WSL, rootless
Docker and host display forwarding are outside this first slice.

This fixture does not test a real sidecar/PTY, native permissions, signing,
installers, GPU rendering or production history storage. It does not remove
native/release verification. The general dev Compose bind-mount/ownership
questions in #95 remain separate even if this no-mount fixture succeeds.

## Concise implementation plan and acceptance

1. After owner approval, record the test-only isolation decision in a new ADR
   and its index. Add the explicit `headless-replay` build stage and snapshot
   inputs. Check the manifest prewarm, frozen lockfile/patches, Electron binary,
   fonts/Xvfb and no unresolved `ldd` dependencies. Run the real fixed fixture
   offline as non-root with the full security profile; inspect the five stage
   screenshots and the saved-grid assertions. Existing image targets must keep
   building and retain their commands.
2. Implement the one host command with behavioral tests of an injected fake
   engine executable: unavailable engine, failed/timed-out build/create,
   assertion failure, run timeout, interruption, bounded/truncated output,
   failed export/ownership checks and unreachable cleanup. Assert actual
   attempted commands, result/evidence and exact-CID cleanup; never source-text
   checks or deletion of a pre-existing canary container/directory. Then verify
   the same lifecycle against real candidate engines. Do not enable runtime
   support based only on the fake-engine tests.
3. Add targeted CI for a real available qualification profile and document the
   exact supported setup/command, dependency/binary preparation, evidence paths,
   cache behavior, startup failures and native limitations in the container
   guide and scoped desktop guide. No unsupported matrix claims. Run checks
   for the changed surfaces, `scripts/verify-repo.sh` after code implementation,
   docs check/build, diff/doc/worktree checks and current-code review before PR
   handoff. Record each qualified or unavailable matrix row on #95.

These steps are one coupled deliverable: a usable isolated replay command with
proven ownership and cleanup. Root remains the implementation owner; use no
parallel writers. The builder may choose helper structure and the exact apt
closure based on measured failures within the fixed boundaries. Changing the
security profile, mounting host paths or expanding fixture scope requires
review of the materially changed design before implementation.

## Risks, rollback and review

| Dimension | Rating and evidence |
| --- | --- |
| Dependencies | 3: known image/engine/Node/Electron boundaries; engine profiles require independent qualification. |
| Blast radius | 1: explicit test-only target/command; existing development and product entry points stay unchanged. |
| State changes | 2: owned ephemeral image/container/artifact state; no host source or user-session writes. |
| Reversibility | 1: stop/remove the exact owned container and revert additive stage/runner/docs; no data migration or global cleanup. |
| Uncertainty | 3: missing libraries measured, but candidate apt closure and full runtime/ownership matrix have not run. |

Total: 10/25, a judgment aid rather than approval. Unavailable runtime or cleanup
proof blocks the corresponding support claim; it is not a reason to weaken the
contract. Rollback must retain exported failure evidence and never remove
pre-existing containers, volumes or host directories.

Plan review: **Ready for owner decision; implementation remains gated.** Scope,
interfaces, acceptance and recovery were checked against #95 and the current
fixture. Fresh builder-check routing: one read-only reviewer, one bounded round,
questions about missing consequential decisions only; root is the sole writer.
Builder check completed: **no consequential unanswered questions**. The reviewer
checked exact-CID lifecycle, stopped-container artifact persistence, no-mount UID
behavior, support gates and bounded export. No owner decision was silently
inferred; this remains a proposal awaiting approval.

Blind-spot planning checkpoint:

- Least-confident assumption: host artifact ownership across engines. Reviewed
  current engine help and both primary copy references above. Docker documents
  caller ownership; Podman behavior remains an explicit qualification gate,
  not an inferred match. Enforce bounded transfer before claiming success.
- Project-level risk: treating a macOS Podman VM pass as native Linux rootless
  support, or a synthetic replay as native/app-sidecar acceptance. Compared the
  existing #95 scope, fixture and ADR 0009; kept distinct matrix rows and evidence
  limits rather than expanding claims. No new issue is needed: #95 already owns
  these outstanding requirements.
- Alternative rejected: read-only live source bind with runtime installation.
  The actual #1639 audit showed pnpm creates a project temporary file even for a
  frozen install. Snapshot/dependency build layers remove this writable-bind
  and UID/chown dependency. A read-only runtime root plus artifact tmpfs was also
  rejected because artifacts must survive exit; the owned disposable writable
  layer avoids a persistent-volume initializer.
