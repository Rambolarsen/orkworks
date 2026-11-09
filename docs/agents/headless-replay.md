---
type: Development Guide
title: Isolated headless replay
description: Run and diagnose the isolated offline replay fixture with explicit profile and cleanup evidence.
tags: [orkworks, testing, containers, electron]
status: stable
---

# Isolated headless replay

Run the existing recorded-terminal replay fixture in a snapshot image, without
mounting a host workspace, home, display or engine socket into the container.
It exercises real App/TerminalPanel/HistoricalTerminal/xterm DOM rendering with
synthetic backend/bridge/WebSocket data. It does not start a sidecar, PTY or
coding harness and does not qualify a packaged app or native platform behavior.
The boundary follows [ADR 0113](../adr/0113-headless-replay-fixture-isolation.md)
and the [approved design](../superpowers/specs/2026-11-09-headless-replay-container-design.md).

## Requirements and command

The host needs Bash, Python 3.9+ (standard library only), Git and an already
reachable selected container engine. No host Node, Rust, Electron or display
server is required. The command never starts a daemon or VM, uses sudo, changes
host security settings or selects a different engine after failure. Use a
trusted checkout; the image build installs the frozen repository dependencies.

```bash
bash scripts/headless-replay.sh --engine podman
# Or, for the separately qualified Docker profile:
bash scripts/headless-replay.sh --engine docker
```

The image build needs registry/package-download networking. The fixture itself
runs with networking disabled. The build context includes only tracked fixture
inputs from the current working files: manifests, overrides, patches, installer,
renderer source, JSX tsconfig and the one shell test. Add new required inputs to
the explicit allowlist; untracked files are excluded. Symlink inputs are refused,
including links to private files. Repository-root/private files are not sent to
the daemon. Credentials and ambient host environment are not injected into the
runtime container. Known proxy defaults are explicitly cleared (Podman's proxy
inheritance is disabled); before start, container environment must match the
trusted image plus empty proxies and known engine-generated identity fields.
Unexpected names, changed values or engine-reported mounts refuse execution;
values are never logged. Use only the qualified engine/profile configuration;
custom engine auto-mount/privilege settings are not qualified by these runs.
See the [Docker proxy defaults](https://docs.docker.com/engine/cli/proxy/) and
[Podman create defaults](https://docs.podman.io/en/latest/markdown/podman-create.1.html).
A changed renderer rebuilds its source layer; frozen dependency
layers are reused when the selected engine has them cached.

The explicit `headless-replay` image target prepares Xvfb/xauth, fonts, the
Bookworm GTK/NSS/ALSA/GBM dependency closure, patched dependencies and the locked
Electron binary. Its build rejects unresolved `ldd` dependencies. It invokes
Node directly at runtime: no package installation, download, Vite server or
Electron fs-ext rebuild occurs then. This renderer fixture never loads the
Electron main-process native modules; it is not an app ABI/build qualification.
The default dev Compose service, existing named caches and release flow are
unchanged. Those live-bind ownership questions remain separate #95 work.

## Runtime and evidence

The runner uses UID/GID 1000, no-new-privileges, all capabilities dropped, default
seccomp, init, no network, 2 GiB memory, 256 PIDs and 256 MiB shared memory. It
retains the existing Linux-only fixture Chromium `--no-sandbox` argument inside
this developer-test boundary. It provides no arbitrary command, volume or
privilege pass-through. Do not disable limits, seccomp or isolation to turn a
failed qualification green.

The approved run budgets are build 15 minutes, create 30 seconds, fixture 90
seconds, export 30 seconds and cleanup 30 seconds. Interrupting the host command
attempts bounded cleanup of only its private CID. The stopped container's
writable layer retains artifacts until export/removal; source/dependencies are
root-owned and runtime read-only by ownership. No persistent volume or recursive
host chown is used.

The command prints a fresh private artifact directory, created under the host's
normal temporary directory (or TMPDIR):

- `result.json`: image/CID, actual engine profile, outcome, export and cleanup
  status, confirmed lifecycle milestones and the last successfully inspected state
  (retained even if later engine access fails). `run.log`: bounded build/engine/test output, capped at 2 MiB with an
  explicit truncation marker.
- `replay/`: five normal PNG/JSON pairs (visible and initially-hidden/revealed
  grids at wide/narrow sizes), optional failure PNG/JSON, and subprocess log.
  Only these 13 names are eligible for export: PNGs at most 8 MiB, JSON at most
  1 MiB, subprocess log at most 2 MiB, total at most 64 MiB.

Engine tar bytes are capped before parsing. Only one regular expected file is
accepted per archive; links, unexpected paths/types and oversized data are
refused. Files are written by the caller without tar ownership/mode restoration,
then UID/GID verified. No archive is extracted into a user-supplied directory.

A passing result requires the normal artifacts and confirmed container removal.
Failure/timeout can lack screenshots; retain logs and the explicit reason.
`cleanup: unsettled` or `identity-unsettled` is a failure, not completion. The
printed CID/private CID-file path and `result.json` identify what remains
unconfirmed. Inspect that exact resource after restoring engine access; never
prune or stop unrelated sessions, containers or volumes. Image build/cache layers
may remain intentionally; the command does not globally prune images.

## Qualification and support limits

| Profile | Evidence boundary |
| --- | --- |
| macOS arm64, Podman Linux VM with rootless server | Locally qualified: real replay, assertion failure, renderer timeout and interruption; source hashes/ownership unchanged, caller-owned export and exact-CID removal. This does not establish native Linux rootless behavior. |
| Linux amd64, rootful Docker, non-root host CLI user | The `Headless replay (Linux Docker)` CI job runs the real four-case qualification. Require its passing current-head result and receipts before claiming a particular commit is qualified. |
| Native Linux amd64, rootless Podman | Candidate, not qualified by the VM run. Run the same real qualification on that host before adding a support claim. |

Docker Desktop, rootless Docker and Windows/WSL are not qualified by these rows.
Host display forwarding, GPU/native chrome, permissions, signing/installers and
actual sidecar/PTY/replay-store integration remain outside this fixture. The
fixed UI flow uses the actual recorded 120 × 40 grid, column-121 wrapping,
container resizing and recovery after a hidden initial mount, not empty panels
or source-string assertions. PNGs are evidence to inspect, not cross-platform
pixel goldens.

## Checks and startup diagnosis

```bash
python3 scripts/headless_replay_test.py
ORKWORKS_REAL_REPLAY_ENGINE=podman python3 scripts/headless_replay_qualification_test.py
```

The first command uses a fake engine and controlled subprocesses; it never needs
a daemon. The second opts into real image builds and containers, preserves its
qualification evidence, and verifies success, assertion failure, timeout and
interruption against a separately owned stopped canary. Fault injection changes
only the temporary snapshot, never the host fixture. `host.json` records host
architecture, caller ownership and source-hash/ownership preservation.

If startup fails, read `run.log` and `result.json` first. If Python is missing,
the Bash entry cannot create those files; its stderr states that prerequisite. Unreachable engine,
missing Python/Git, absent tracked input, refused symlink, package/network failure,
missing Electron binary/library, renderer console error, resource exhaustion,
failed export and unsettled cleanup are different failures. The wrapper does not
auto-repair the host, install tools or relax security. The frozen install plus
explicit Electron installer, `ldd` check, inherited JSX configuration and renderer
console logging make these boundaries observable. CI retains evidence for seven
days even on a failed qualification.
