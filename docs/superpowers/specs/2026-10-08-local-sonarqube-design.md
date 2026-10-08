# Local SonarQube and repository simplification

Approved by the owner in the 2026-10-08 session; implementation tracked in
[issue #777](https://github.com/Rambolarsen/orkworks/issues/777).

## Purpose and boundary

Provide free, local developer tooling for verifying the repository and finding
behavior-preserving reductions in maintained code and complexity. This is an
optional extension of the containerized development workflow, not a dependency
of the desktop app, sidecar, Taskmaster, release pipeline or hosted CI scanner.

## Runtime and commands

Use SonarQube Community Build 26.9.0.129388 and PostgreSQL 17.11 through a
separate Podman Compose project. Publish only the Sonar UI on loopback port
9000; retain database and Sonar state in named volumes. Extend the existing
toolchain Containerfile with a scanner stage using @sonar/scan 5.0.1, without
changing the default development stage. The scanner installs desktop
dependencies with pnpm and imports or runs Clippy for the non-root sidecar
manifest. Named scanner caches never share host node_modules or target files.

`python3 scripts/sonar.py up|scan|report|compare|down` is the public interface
on macOS/Linux; native Windows PowerShell uses `py -3` or `python`. Support
Python 3.9+, native Windows file locks and user-only credential ACLs. Transfer
snapshots through relative `podman cp` paths rather than host bind mounts.
`up` creates private credentials outside Git, starts the services and provisions
a local token. `down` stops containers and preserves named volumes. Startup
documents Podman VM memory, Compose-provider and Elasticsearch kernel limits;
the helper does not disable bootstrap checks or silently resize a running VM.

## Snapshot and report contract

Community Build has no native multi-branch analysis. Derive a separate project
key from each canonical checkout path. Lock scans in a checkout and stage a
copy of eligible Git-listed source, including unstaged and nonignored new
source files. Exclude credentials, generated output, platform-specific caches
and symlinks. Hash the exact staged bytes and their relative paths; record Git
HEAD, actual branch and dirty status independently. Do not send Git metadata.

Use explicit TypeScript/Electron/Rust source paths, desktop test paths and
generated exclusions. Rust inline tests remain in Rust source analysis; the
report must not label its line total as pure production lines. No coverage
report is currently available, so coverage is unverified, not zero.

Wait for the submitted Compute Engine task, require SUCCESS for this project,
and tie the report to its analysis ID. Refuse to read a different latest
analysis as this snapshot. Record server, analyzer/profile and scanner/scope
identity (including effective server settings and gate conditions), project/file
metrics, open issues, analysis warnings, quality gate and provenance in
ignored `.sonar/reports/` JSON. Pagination must be complete or fail explicitly.
Compare reports only when their analysis configuration matches. Missing
metrics remain null, and unavailable deltas remain null.

## Skill contract

`skills/simplifying-repo/SKILL.md` makes this workflow discoverable. It selects
one bounded area from measured findings, verifies the reported issue against
current code, and follows existing issue, spec, worktree, TDD and review rules.
Intentional IPC type duplication and ownership/lifecycle checks are preserved.
Metrics guide inspection; a passing gate does not prove behavior preservation.
Moving branches into helpers, shortening formatting, dropping tests or hiding
warnings does not by itself demonstrate a simplification. Report honest
before/after measurements together with behavioral verification and limits.

## Validation

Exercise snapshot inclusion/exclusion, fingerprint changes, project isolation,
stale/failed analysis rejection, unknown metrics and incompatible comparison
with the real helper. Run skill consumers before and after the instructions.
Bring up the actual ARM64 Podman stack and complete a real Rust/TypeScript scan.
Run native Windows/Linux helper CI tests; document WSL2/Hyper-V prerequisites
and distinguish helper CI from native Windows Podman verification.
Run the repository verification helper, docs checks and scoped independent
review before PR handoff. Do not refactor application code during this setup.
