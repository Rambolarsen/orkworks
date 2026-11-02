---
type: Validation Report
title: Selective Rust CodeQL qualification
description: Retained source, coverage and scan-routing evidence for the approved CodeQL publisher migration.
tags: [ci, codeql, security, validation]
status: draft
workflow_status: qualifying
---

# Selective Rust CodeQL qualification

Delivery of [issue #1312](https://github.com/Rambolarsen/orkworks/issues/1312)
follows the owner-approved
[security design](../superpowers/specs/2026-11-02-codeql-scan-routing-design.md)
and [implementation plan](../superpowers/plans/2026-11-02-codeql-scan-routing.md).
This report distinguishes successful staging from live activation.

## Non-publishing stage

[PR #1313](https://github.com/Rambolarsen/orkworks/pull/1313) merged the staged
workflow at `a5f4cef3ac894ce60bd31d1831be857fb23207e1`. Managed default setup
remained active and `CODEQL_ADVANCED_UPLOAD` was absent. Both the workflow's
SARIF upload and database publication were disabled.

The staged PR run
[41475811518](https://github.com/Rambolarsen/orkworks/actions/runs/41475811518)
and managed run
[41475803379](https://github.com/Rambolarsen/orkworks/actions/runs/41475803379)
passed all five languages. The PR head was
`c30dd0f0324f367edafb9f649331a200d0a5f3db`; the staged merge source was
`33afc54216afdea73b188264edef52b13fe2ad96`. Git tree identities matched.

For every language, CodeQL 2.27.3, query-pack versions and individual SARIF
findings matched. Both runs used PR diff-informed analysis and had zero PR
findings. This does not establish a zero-alert repository. Full main comparison
is required before activation.

Both Rust scans reported 196 files successfully extracted and one file extracted
with errors. Staged SARIF and managed logs identify the existing macro-expansion
warning at `crates/orkworksd/src/taskmaster/runtime.rs:2150:9` for
`$crate::panic::panic_2021`. The staged scan reports 179638 Rust source lines.
These counters do not establish complete extraction or pure production line counts.

Forty-one documentation tooling tests passed, including actual embedded inputs
across Rust crates, real Git cumulative diffs/renames, publication boundaries,
and execution of metadata collection against canonical language alias fixtures.
Workflow linting and the documentation build passed. The full local verifier
stopped at the existing Node 26/required Node 22 prerequisite; hosted checks used
their configured versions. The final Codex review of the staged code was clean.

## Main and repeated qualification

Full-source comparison is in progress at the immutable merged staging commit:

- Owned main run: [41477528118](https://github.com/Rambolarsen/orkworks/actions/runs/41477528118).
- Managed main run: [41477525339](https://github.com/Rambolarsen/orkworks/actions/runs/41477525339).
- Forced-full manual repeat: [41477677099](https://github.com/Rambolarsen/orkworks/actions/runs/41477677099), on the frozen qualification branch at the same commit.

## Hosted unrelated-change routing

The prose-only [PR #1316](https://github.com/Rambolarsen/orkworks/pull/1316)
triggered owned run
[41478156533, attempt 1](https://github.com/Rambolarsen/orkworks/actions/runs/41478156533/attempts/1).
Routing and all four non-Rust jobs passed. No Rust analysis job was created.
The diff consists only of this report and its Markdown directory index. Managed
scanning still runs separately during qualification; it has not been disabled.

## Activation state

Not activated. Default setup remains configured. Main/repeated coverage,
query/finding comparison, a real prose-only routing check, cancellation evidence
and the full publisher cutover/restore checks remain required. No billing saving
or lower per-scan runtime is claimed from PR timings.
