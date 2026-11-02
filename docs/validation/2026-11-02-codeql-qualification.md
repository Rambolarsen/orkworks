---
type: Validation Report
title: Selective Rust CodeQL qualification
description: Retained source, coverage and scan-routing evidence for the approved CodeQL publisher migration.
tags: [ci, codeql, security, validation]
status: stable
workflow_status: activated
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

Full-source comparison passed at the immutable merged staging commit:

- Owned main run: [41477528118](https://github.com/Rambolarsen/orkworks/actions/runs/41477528118).
- Managed main run: [41477525339](https://github.com/Rambolarsen/orkworks/actions/runs/41477525339).
- Forced-full manual repeat: [41477677099](https://github.com/Rambolarsen/orkworks/actions/runs/41477677099), on the frozen qualification branch at the same commit.

All three full runs used the same source commit and CodeQL 2.27.3/query-pack
versions. Individual findings matched after resolving standard SARIF defaults:
rule default severity when `result.level` is omitted, and `startLine` when
`endLine` is omitted. Messages, primary file/region and duplicate findings were
retained in the comparison; no findings were filtered. These are serialization
defaults in [SARIF 2.1.0](https://docs.oasis-open.org/sarif/sarif/v2.1.0/sarif-v2.1.0.html),
not severity or location changes.

| Language | Managed findings | Owned main | Manual repeat | Extracted-file locations in both owned full runs |
| --- | ---: | ---: | ---: | ---: |
| Actions | 0 | 0 | 0 | 10 |
| C/C++ | 0 | 0 | 0 | 2 |
| JavaScript/TypeScript | 26 | 26 | 26 | 404 |
| Python | 0 | 0 | 0 | 57 |
| Rust | 13 | 13 | 13 | 197 |

The successful-file location sets and extraction-warning/error diagnostics match
between the owned full scans. Managed full Rust logs match the same 196 successful
files, one error file and existing `runtime.rs:2150:9` macro warning.

Rust job runtimes were 476 seconds managed, 318 seconds owned main and 303 seconds
on the manual repeat. These observations do not establish a per-scan speedup:
runner/cache/load conditions differ. Routing avoids the Rust job entirely for
qualifying unrelated changes; actual aggregate savings depend on change mix.

## Hosted unrelated-change routing

The prose-only [PR #1316](https://github.com/Rambolarsen/orkworks/pull/1316)
triggered owned run
[41478156533, attempt 1](https://github.com/Rambolarsen/orkworks/actions/runs/41478156533/attempts/1).
Routing and all four non-Rust jobs passed. No Rust analysis job was created.
The diff consists only of this report and its Markdown directory index.
A bounded rerun of that PR scan (attempt 2) was cancelled by the newer commit
`9e572042cd5143033a05022237dff600344de0a9`. The replacement owned run
[41478724182](https://github.com/Rambolarsen/orkworks/actions/runs/41478724182)
passed its routing job and four non-Rust jobs. The main and manual full scans
were not cancelled by these PR updates. Managed
scanning still runs separately during qualification; it has not been disabled.

## Activation state

Activated after successful qualification. Managed default setup is
`not-configured`; `CODEQL_ADVANCED_UPLOAD=true` enables the owned publisher.
The first full publishing run
[41479654105](https://github.com/Rambolarsen/orkworks/actions/runs/41479654105)
passed all five jobs on main at the exact qualified commit. Its published
SARIF matches the immutable managed baseline's source tree, CodeQL/query-pack
versions and individual findings across all five languages. Successful-file
locations and extraction diagnostics remain unchanged. The new publisher was
never used as its own comparison baseline.

All existing `/language:actions`, `/language:c-cpp`,
`/language:javascript-typescript`, `/language:python`, and `/language:rust`
categories received successful owned uploads. All 27 previously open alert
identities were retained, with no additional or missing open identities at the
cutover check. No historical analyses or alerts were deleted.

A concurrent Rust source change at
`81322cc57c9f63de4778b42c0cd5bcf6d15efe29` triggered all five owned jobs in
[41479894475](https://github.com/Rambolarsen/orkworks/actions/runs/41479894475),
where all five jobs passed, confirming positive routing for a real Rust change.
It is a separate source
revision and was not substituted for the frozen activation comparison.

## Recovery and remaining limits

The pre-cutover configuration and SARIF were captured before mutation. Disabling
managed setup retained its languages, default suite, remote threat model and
standard runner settings, as confirmed by the subsequent GET. Recovery is to
set `CODEQL_ADVANCED_UPLOAD=false`, restore managed setup with `state=configured`
and the recorded settings, then verify a new full managed scan. Do not leave both
publishers enabled or delete historical results during recovery. The authenticated
path successfully changed both relevant settings during cutover; restoration was
not needed and a production rollback drill was not performed.

Artifacts retain SARIF, CLI/query-pack versions, source SHA and database metadata
for seven days. The baseline macro-expansion limitation remains unchanged. The
independent AI security-findings quota failure is outside this migration. Neither
future billing nor an aggregate percentage saving is inferred from these runs.
