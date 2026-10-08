# Local SonarQube Implementation Plan

> **For agentic workers:** Use executing-plans to implement this plan inline.

**Goal:** Deliver an optional free Podman SonarQube workflow and simplification skill.

**Architecture:** Separate persistent analysis services from snapshot-based,
throwaway scans. Bind reports to completed analysis and exact source identity.

**Tech Stack:** Podman Compose, PostgreSQL, SonarQube Community Build, pnpm
SonarScanner, Python standard library.

**Spec:** `docs/superpowers/specs/2026-10-08-local-sonarqube-design.md`.

## Global constraints

- SonarQube Community Build 26.9.0.129388; PostgreSQL 17.11; @sonar/scan 5.0.1.
- Local developer tooling only; no product/runtime or release dependency.
- Native Windows PowerShell/Python plus macOS/Linux; Windows helper CI and VM runbook.
- Named container dependency volumes, loopback UI, private credentials outside Git.
- Report Rust inline test contribution and unavailable coverage honestly.
- Preserve Electron/renderer boundaries and intentional IPC type duplication.

## Uncertainty and blind-spot checkpoint

Least confidence: current Podman/ARM64 scanner behavior and the server's live
API payloads. Mitigation: validate the actual stack, compiler invocation and
report identity; do not rely on a passing static config check.

Project blind spot: fewer lines and a passing quality gate can hide moved
complexity, omitted languages, stale analyses or weakened tests. Keep immutable
snapshots, unchanged analyzer settings and behavior verification in the skill.
Existing architecture-audit issues remain separate; this setup picks none.

## Task 1: Working local analysis workflow

Files: `compose.sonar.yaml`, `Containerfile`, `compose.yaml`,
`sonar-project.properties`, `scripts/sonar.py`, `scripts/sonar_test.py`,
`.gitignore`, `docs/adr/0079-local-sonarqube-analysis.md`, ADR index.

Interfaces: `snapshot(root, destination)` returns exact-source identity;
`metric_values(measures)` preserves missing values;
`completed_task(payload, project)` rejects failed/misbound analyses;
`verify_analysis(expected_id, payload)` rejects stale reports;
`compare(before, after)` requires identical configuration.

- [x] Write behavioral tests against real temporary Git checkouts and literal API fixtures.
- [x] Run `python3 scripts/sonar_test.py` before the helper exists; observe missing implementation.
- [x] Implement these interfaces and the CLI commands from the design.
- [x] Run the same tests until all pass.
- [x] Run `up`, complete `scan --label baseline`, read `report`, and exercise `compare`.
Commit the verified analysis workflow and decision record with the skill.

## Task 2: Usable skill and runbook

Files: `skills/simplifying-repo/SKILL.md`, `docs/agents/local-sonarqube.md`,
`docs/agents/index.md`, `docs/agents/apm.md`, `docs/agents/project-context.md`,
root `AGENTS.md`, `README.md`, PR CI helper-test routing.

- [x] Run a fresh-context consumer without the skill. It rejected stale evidence
  but could not provide runnable scanner commands: the missing technique is
  an executable snapshot/report workflow, not another generic audit checklist.
- [x] Write concise skill instructions linked to the commands and runbook.
- [x] Repeat the consumer scenario with the skill; verify observable decisions.
- [x] Validate skill frontmatter and reference links.
- [x] Run `bash scripts/verify-repo.sh`, docs currency and worktree currency.
- [x] Perform independent correctness and completeness review; fix supported findings.
Track commit, push, PR and bounded babysitting disposition in issue #777.

## Agent routing

One implementation writer (the session owner). No parallel implementation.
Up to four fresh read-only evaluations across the task: baseline consumer,
skill consumer, correctness reviewer and completeness reviewer. Use available
Luna at high effort; no reviewer receives intended answers. Baseline feeds the
skill; reviews inspect the final artifacts and cannot run until those exist.

## Verification evidence (2026-10-08)

- 19 helper tests pass on macOS/Python 3.9; native Windows/Linux CI is included.
- Actual ARM64 Podman services started; Rust Clippy, Rust and TypeScript sensors
  completed, followed by a successful server Compute Engine task.
- Project analysis: 195 source files (88 Rust), 362 open issues, ncloc 130969,
  cyclomatic complexity 17138, cognitive complexity 10908, duplicate lines 1149.
  These source totals include Rust inline tests; coverage is unverified.
- The server reports an encoding warning on an existing Rust test's literal
  replacement character. The helper preserves analysis warnings for inspection.
- Live refresh and offline compare succeeded; identical reports produce zero
  metric deltas. A changed Clippy setting caused live refresh to refuse evidence.
- `RUST_TEST_THREADS=4 bash scripts/verify-repo.sh` completed every required step.
  Initial unconstrained Rust fixture run failed three probe barriers; the bounded
  rerun passed all 1809 Rust unit tests and four integration tests.
- Skill frontmatter validation and fresh read-only consumer replay passed. The
  consumer produced executable native Windows commands and rejected stale
  shared-main evidence; it selected no unsupported refactor.
- Independent correctness/completeness review prompted captured effective server
  settings, gate conditions, Windows Compose-provider installation instructions
  and scoped recovery for a force-killed scanner. Follow-up tests cover these.
- Windows Podman VM execution remains a documented platform verification limit;
  helper CI must not be described as a completed native Windows scan.
- No active Taskmaster recommendation matched this tooling change (209 records
  inspected); there is no matching completion to post.
