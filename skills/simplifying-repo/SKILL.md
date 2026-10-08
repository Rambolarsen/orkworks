---
name: simplifying-repo
description: Use when asked to validate repository health, reduce code complexity or maintained code lines, remove redundant logic, or investigate SonarQube maintainability findings.
---

# Simplifying the Repository

Reduce the work a maintainer must understand while preserving implemented
behavior. Use measured evidence to choose one bounded improvement.

## Establish evidence

Read the root and applicable scoped instructions, authoritative specs and
existing issues. Follow the repository's branch, approval and review workflow.
Keep other sessions' work intact. An audit request authorizes an audit; make
changes only within the user's implementation scope.

Read [the local SonarQube runbook](../../docs/agents/local-sonarqube.md) for
first-time setup, prerequisites, credentials and metric limits.

On native Windows, replace `python3` with `py -3` (or `python`); follow the
runbook for WSL2/Hyper-V and private credentials.

| Action | Command from the owning checkout |
| --- | --- |
| Start local services | `rtk proxy python3 scripts/sonar.py up` |
| Capture baseline | `rtk proxy python3 scripts/sonar.py scan --label baseline` |
| Read current evidence | `rtk proxy python3 scripts/sonar.py report --label current` |
| Capture changed state | `rtk proxy python3 scripts/sonar.py scan --label after` |
| Compare | `rtk proxy python3 scripts/sonar.py compare .sonar/reports/baseline.json .sonar/reports/after.json` |

Run the relevant existing verification before editing. Record pre-existing
failures separately. `scan` includes source edits and new nonignored files;
`report` rejects changed source and stale server analyses. Never substitute a
green result from another revision or a shared main project. If analysis cannot
run, report the missing evidence and continue only the authorized inspection.

## Select and simplify

Inspect analysis warnings, file metrics and issue locations, then trace callers, data ownership,
tests and contracts before accepting a finding. Rank by removable complexity,
likely code reduction, confidence and regression risk. Check existing issues
before creating or adopting work. Apply the uncertainty/blind-spot checkpoint
and issue/spec guardrails from `surfacing-blind-spots` when planning a change.

Favor verified dead code, repeated decisions, redundant state and abstractions
whose deletion removes responsibilities. Start with one coherent area. Preserve
the independent Electron/renderer IPC type copies and other deliberate boundary
duplication. Sonar findings identify inspection candidates, not permission to
cross boundaries or remove functionality.

Use the applicable debugging, planning, TDD and review skills for the chosen
change. Preserve meaningful tests and error/lifecycle safeguards. Keep rule
profiles, exclusions and source/test scope unchanged during comparison.

## Verify the reduction

Rescan after behavioral checks. Compare the whole affected call path, including
new helpers: lower complexity in one function may merely move the branches.
Code relocation, compressed formatting, deleted tests, broad suppressions and
weaker quality gates do not demonstrate a reduction in maintained complexity.

Report source identities, files changed, measured deltas, checks run, remaining
failures and tradeoffs. A passing quality gate is not proof of preserved
behavior. Missing values remain unknown. Sonar's Rust source line counts
include inline tests; distinguish actual production changes from test changes
in the diff and do not claim pure production totals. Coverage remains
unverified until a real coverage report is imported.

Keep immutable baseline evidence until the PR is reviewed. Tie off matching
Taskmaster recommendations through the sidecar API when required by the root
guide. Follow the existing PR babysitting and worktree cleanup workflow.
