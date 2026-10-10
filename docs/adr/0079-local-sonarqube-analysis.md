---
type: "Architecture Decision"
title: "Local SonarQube analysis uses isolated checkout snapshots"
description: "Architecture decision record: Local SonarQube analysis uses isolated checkout snapshots."
tags: ["orkworks", "architecture"]
---

# Local SonarQube analysis uses isolated checkout snapshots

- Status: accepted
- Deciders: repository owner
- Date: 2026-10-08

## Context

The owner wants free, local complexity and code-reduction tooling through
Podman. OrkWorks supports concurrent agent worktrees, and SonarQube Community
Build has one main analysis per project. Scanning every worktree into the same
project would mix evidence from unrelated work. App verification remains the
existing compiler, tests and review workflow.

## Decision

Use a separate optional SonarQube Community Build/PostgreSQL Compose stack and
an additional stage of the existing toolchain for scanning. Services retain
named volumes and publish only a loopback UI. Native Windows PowerShell and
macOS/Linux use a Python helper with platform locks and private credentials.
Relative `podman cp` transfers avoid platform-specific host bind mounts.

Each canonical checkout has a separate project. Scan a copy of eligible
Git-listed working files, record its content fingerprint and actual Git state,
and bind reports to the completed server analysis. Reports are comparable only
under the same scanner, analyzer/profile and source-scope configuration.

The simplification skill uses these measurements to investigate bounded
behavior-preserving changes under the existing development workflow. Product
code does not depend on or connect to SonarQube.

## Consequences

Local scans remain free and worktree results do not overwrite the shared
baseline. They require Podman resources and initial public dependency downloads.
Native Sonar branch/PR decoration is unavailable in Community Build. Rust inline
tests contribute to analyzed source lines, and absent coverage remains unknown.
Named cache volumes need explicit housekeeping when no longer useful.

See the [runbook](../agents/local-sonarqube.md) and
[approved design](../superpowers/specs/2026-10-08-local-sonarqube-design.md).
