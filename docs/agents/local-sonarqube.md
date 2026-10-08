---
type: Process Guide
title: Local SonarQube analysis
description: Free local Podman analysis and verified repository simplification evidence.
tags: [development, podman, sonarqube, complexity]
status: stable
---

# Local SonarQube analysis

SonarQube Community Build and PostgreSQL run locally through Podman. This is
optional developer tooling: the app and sidecar do not connect to it. The UI is
published only at `http://127.0.0.1:9000`. No paid license or cloud account is
required. Source is analyzed by the local server; initial builds and analyzer
provisioning download public dependencies.

## First-time setup

Install Python 3.9+, Git, Podman and a Compose provider (`podman-compose` or `docker-compose`).
On macOS and Windows, Podman uses a Linux VM. The official Sonar image supports ARM64.
The server's documented starting requirement is 4 GB RAM; allocate about 8 GiB
to the VM when also running PostgreSQL and scanning this repo, then adjust from
observed use. Do not resize or stop a VM owned by another active task.

### macOS and native Linux

For an existing stopped macOS default VM:

```bash
podman machine set --memory 8192 podman-machine-default
podman machine start podman-machine-default
podman machine ssh podman-machine-default sudo sysctl -w vm.max_map_count=524288 fs.file-max=131072
```

The sysctl command applies to the current VM boot. Persist these values in the
VM's `/etc/sysctl.d/99-sonarqube.conf` if desired, or reapply after restarting
it. On native Linux, configure these values on the container host. The Compose
service sets the required file-descriptor/thread limits and preserves
Elasticsearch bootstrap checks.

### Windows (native PowerShell)

Install Python 3.9+ and Git for Windows, plus Podman with its WSL2 or Hyper-V
provider. A native Python Compose provider can be installed in a private virtual environment:

```powershell
py -3 -m venv "$env:LOCALAPPDATA\OrkWorks\compose"
& "$env:LOCALAPPDATA\OrkWorks\compose\Scripts\python.exe" -m pip install podman-compose==1.6.0
$env:PODMAN_COMPOSE_PROVIDER = "$env:LOCALAPPDATA\OrkWorks\compose\Scripts\podman-compose.exe"
podman compose version
```

Set `PODMAN_COMPOSE_PROVIDER` in each new shell, or save it as a user environment
variable through Windows settings. This avoids a dependency on Docker Desktop.
Podman Desktop can manage the machine; the Python helper runs on Windows.
No Bash, WSL checkout or host Rust/Node install is needed for the scanner.

```powershell
# Initialize only if you do not already have a Podman machine.
podman machine init
podman machine start
podman machine ssh sudo sysctl -w vm.max_map_count=524288 fs.file-max=131072
py -3 scripts/sonar.py up
```

Apply the same Linux kernel limits after a VM restart. WSL2 shares its kernel
and resource allocation with other WSL distributions. Set sufficient memory
through `%USERPROFILE%\.wslconfig` (for example `[wsl2]` with `memory=8GB`),
then restart WSL only when other tasks permit it. `podman machine set --memory`
does not independently allocate memory for the WSL provider. Hyper-V uses its
VM memory settings instead. See the official Windows provider guide below.

Use `py -3` in place of `python3` in every command in this guide. If the Python
launcher is unavailable, use your installed `python` executable. Source is
copied through `podman cp` using relative paths; no Windows drive bind mount
or macOS temporary-directory share is required.

### Credentials and startup

From macOS/Linux, run `python3 scripts/sonar.py up` in the checkout. `up` starts
the pinned services, waits for readiness, replaces the initial admin password
and creates a local API token. Credentials are stored outside Git:

| Host | Private credential file | Protection |
| --- | --- | --- |
| macOS/Linux | `~/.config/orkworks/sonar.env` | Mode 600 |
| Windows | `%LOCALAPPDATA%\OrkWorks\sonar.env` | Inheritance removed; current user granted access via `icacls` |

`SONAR_ENV_FILE` can select a different private file. Login is `admin`; open
that file privately to obtain the generated password. Preserve this file
together with persistent service data. Startup does not erase an existing
database to recover lost credentials.

Service versions: SonarQube `26.9.0.129388-community`, PostgreSQL
`17.11-bookworm`. The scanner stage extends the existing toolchain and pins
`@sonar/scan` to `5.0.1`. Normal `compose.yaml` builds the `dev` stage.

## Scan and compare

```bash
python3 scripts/sonar.py scan --label baseline
# Implement one scoped, behavior-preserving change and run its required checks.
python3 scripts/sonar.py scan --label after
python3 scripts/sonar.py compare .sonar/reports/baseline.json .sonar/reports/after.json
```

`scan` copies Git-listed working source into a temporary snapshot, including
unstaged edits and new nonignored source. It excludes known credential files,
generated output, host dependency caches and symlinks. It installs desktop
dependencies with pnpm in the scanner, and runs Clippy for
`crates/orkworksd/Cargo.toml`. Scanner dependencies, build output and server
data use named volumes. The source snapshot lives in the disposable scanner
container, separate from the host's Node/Electron dependencies.

Community Build supports only one main analysis per project. Each canonical
checkout gets a separate local project key, so a feature worktree does not
overwrite the primary checkout's baseline. Same-checkout scans are locked.
The Git branch and HEAD remain explicit report metadata; these local projects
are not Sonar-native branch or pull-request analysis.

Reports in ignored `.sonar/reports/` contain the staged source fingerprint,
commit/dirty state, completed analysis ID, configuration identity, project/file
metrics, open issue inventory, analysis warnings and quality gate. They become available only
after the submitted server task succeeds. Retrieval checks the latest analysis
before and after collection. `compare` refuses different scope, profiles, effective server settings, quality
gate conditions or analyzer versions and preserves missing values as unknown.

```bash
python3 scripts/sonar.py report --label current
```

`report` refreshes evidence for the last scanned snapshot and refuses changed
source or a replacement analysis. Saved baseline JSON remains useful offline
for comparisons; use distinct labels and preserve it through review.

### Interpreting metrics

- Inspect `analysisWarnings` before interpreting metrics or calling an analysis clean.
- Complexity and duplication identify candidates for inspection; assess the
  whole affected call path to detect complexity merely moved into helpers.
- Rust inline test modules are analyzed with source. The `ncloc` total is not
  a pure production-code total. Inspect production and test changes separately.
- No coverage report is generated/imported by this setup. Coverage is
  unverified; a passing gate does not prove sufficient tests or correctness.
- Independent IPC type definitions in `electron/` and `src/` are intentional.
  Keep them even if a duplication finding suggests merging them.

Use [simplifying-repo](https://github.com/Rambolarsen/orkworks/blob/main/skills/simplifying-repo/SKILL.md) for the complete
agent workflow. It preserves existing scope, issue, review and verification
requirements and does not authorize broad refactoring on its own.

## Stop and troubleshoot

```bash
python3 scripts/sonar.py down
```

This removes the services, retaining named volumes. Worktree scanner dependency
volumes also persist for reuse. Removing them is an explicit housekeeping action.
Do not use `down -v` to troubleshoot: it deletes the stored analysis database.

To inspect startup failures, pass the same private environment file to Compose:

```bash
podman compose -p orkworks-sonar -f compose.sonar.yaml --env-file "$HOME/.config/orkworks/sonar.env" logs sonarqube
```

On Windows use `--env-file "$env:LOCALAPPDATA\OrkWorks\sonar.env"` instead.
If scan locking fails, another process already owns this checkout; wait for it
or use your own worktree. Windows/Linux helper tests run in PR CI, including
real lock contention and credential round trips. They do not exercise a
Windows Podman VM; verify `up` and a completed scan on your Windows machine.

If the helper is force-killed, its detached scanner may remain running. Identify
only containers for the affected project's key with `podman ps -a`, then remove
the matching `orkworks-<key>-scan-<id>` container explicitly with `podman rm -f`.
Do not remove another worktree's active scanner.

For refused connections, verify that the Podman VM and Compose provider work.
For Elasticsearch bootstrap failures, verify VM kernel limits and service
ulimits. For port conflicts, identify the existing owner before starting.
Authentication failure with retained volumes requires the matching saved
credentials; the helper never erases the database to recover access.
Failed Clippy or scanner analysis must be investigated before claiming complete
language coverage. Missing source or an incomplete issue inventory is a failed
analysis, not a clean repository.

## References

- [Podman on Windows](https://github.com/podman-container-tools/podman/blob/main/docs/tutorials/podman-for-windows.md)
- [Podman copy semantics](https://docs.podman.io/en/latest/markdown/podman-cp.1.html)

- [Community Build container installation](https://docs.sonarsource.com/sonarqube-community-build/server-installation/from-docker-image/installation-overview)
- [Host requirements](https://docs.sonarsource.com/sonarqube-community-build/server-installation/server-host-requirements)
- [Linux/Elasticsearch prerequisites](https://docs.sonarsource.com/sonarqube-community-build/server-installation/pre-installation/linux)
- [Rust analyzer](https://docs.sonarsource.com/sonarqube-community-build/analyzing-source-code/languages/rust)
- [Analysis and edition boundary](https://docs.sonarsource.com/sonarqube-community-build/analyzing-source-code/analysis-overview)
