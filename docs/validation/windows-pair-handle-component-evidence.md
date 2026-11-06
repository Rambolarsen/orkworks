---
type: Component Qualification Evidence
title: Windows pair-handle component evidence
description: Exact-head outcome of the single approved Windows SQLite native-handle component attempt, with source ownership and production activation excluded.
tags: [orkworks, hierarchy, windows, native, qualification]
status: draft
workflow_status: component-cases-passed
---

# Windows pair-handle component evidence

Tracking: [#1459](https://github.com/Rambolarsen/orkworks/issues/1459),
[#1176](https://github.com/Rambolarsen/orkworks/issues/1176). Implementation:
`026592ad3b2d352210682e33441fd261d11f7d31` ([#1456](https://github.com/Rambolarsen/orkworks/pull/1456)).

## Outcome

The single approved [workflow run](https://github.com/Rambolarsen/orkworks/actions/runs/42211800477)
passed all six component cases and all 19 expected/observed boolean checks on
local NTFS with bundled SQLite 3.46.0 and the explicit win32 VFS. The documented
per-schema native handles matched the retained original full FILE_ID_INFO tuples,
including after the toy transaction. Swapped/foreign comparisons denied; held
file/leaf-directory sharing guards denied the selected operations and released
rename controls succeeded.

This is positive evidence for this handle-binding/sharing component on the
recorded runtime, not qualification of a complete pair/source owner or a
production profile. No native retry or second dispatch ran.

## Scope and authorization

This executes phase2 of the [approved bounded plan](../superpowers/plans/2026-11-06-windows-pair-handle-qualification.md).
The [accepted owner design](../superpowers/specs/2026-11-06-original-pair-owner-design.md#scoped-owner-protocol-acceptance--2026-11-06)
needs no repeat approval. The prior [native-owner audit](fresh-pair-native-owner-audit.md)
remains an audit of actual production seams.

Only the ignored `qualify_windows_pair_handles` integration test is selected.
Its private synthetic files, toy tables and comparisons are not schema2/format6,
an admitted source, PairSlot/PairCore, a coupled initializer or production wiring.
No Copilot/hosted model probe, application or live sidecar is launched.

The fixed budget is one manual dispatch, at most one qualifying native process,
15-minute job, 120-second native runtime and 256 KiB native output/report.
Disposable synthetic-fixture cleanup is authorized only after native/SQLite
handles close. It does not change application StoppedOccupied/Unknown policy.
No retry, rerun, alternate VFS/platform or replacement native attempt is permitted.

## Run identity

- Workflow: `pair-handle-probe.yml`, event `workflow_dispatch`.
- Owned dispatch ref: `windows-pair-evidence`; run head and expected SHA:
  `026592ad3b2d352210682e33441fd261d11f7d31`.
- Plan commit: `14c9de4e844589923218f5cb1ab2177863580f9f`.
- Run ID: `42211800477`; created `2026-11-06T11:38:55Z`.
- Prior manual run inventory was empty; the final inventory contains exactly this
  one completed successful run. Dispatch 1/1 and qualifying native process 1/1 are
  consumed; the existing authorization has no remaining attempt.
- Exact-head check, locked compile/artifact selection, single native step and
  sanitized artifact upload all completed successfully.
- Job started `2026-11-06T11:39:00Z`, completed `2026-11-06T11:43:24Z` (264 seconds).
- Native report interval: `2026-11-06T11:43:18.5552432Z` to
  `2026-11-06T11:43:18.7108357Z` (0.1555925 seconds). Exit 0, termination confirmed,
  no timeout, output overflow or invalid record.

## Runtime and retained evidence

| Context | Observed value |
| --- | --- |
| Hosted runner | windows-latest, image 20261030.294.1 |
| Rust compiler | rustc 1.99.1 (58cf8f9c3, 2026-10-13) |
| Cargo | cargo 1.99.1 (5f94df478, 2026-08-27) |
| SQLite | Runtime 3.46.0; locked libsqlite3-sys 0.30.1, rusqlite 0.32.1 |
| Filesystem / VFS | Local fixed-drive NTFS / win32; required by the harness before toy SQL |
| Executable SHA-256 | 5f5a963849ba760c0adc85b8232e53ff94bf4b4094e3c6b1d8c82ce1d15eeac0 |
| Sanitized JSON | 4,442 bytes, below 256 KiB; original downloaded bytes SHA-256 d8d018617f4d92c5d5f7addd762941f123fc73f23325c02ef6277c01d0d09b88 |
| GitHub artifact | windows-pair-handle-component-result, ID 13499256571, compressed 955 bytes; seven-day workflow retention |

The complete sanitized JSON is retained below as parsed/reformatted content;
the byte hash above identifies the original CRLF download. No raw handles,
file identities, fixture paths, database contents or environment dump are retained.
The artifact reports one terminated process and all six ordered completion tags.
The owning session additionally checked every expected observation tag, boolean,
source pin, runtime classification and reported elapsed time against the plan.

## Case results

All listed predicates expected true and observed true. Code 0 accompanies successful
comparisons/operations; denial codes are recorded observations, not passing by
absence of a later case.

| Case | Expected and observed predicates | Numeric codes |
| --- | --- | --- |
| Original roles / toy commit | main and hierarchy match their originals, are distinct, contain the two toy rows, and preserve role identity after commit (six checks) | 0 |
| Swapped / foreign | Both swapped-role comparisons and both comparisons with the owned foreign file deny (four checks); foreign file never opened through SQLite | 0 |
| File sharing | Held main-file rename/delete deny; identity remains exact; released rename succeeds (four checks) | Held rename 32, held delete 32; identity/control 0 |
| Directory sharing | Held leaf-directory rename denies; released rename succeeds (two checks) | Held rename 32; control 0 |
| Query failure | Unattached-schema file_control returns non-OK, checked before consuming any output handle (one check) | SQLite 1 |
| Partial creation | Second CREATE_NEW denies against an owned collision; first identity remains retained, without SQLite initialization or recreation (two checks) | Collision 80; retained identity 0 |

Normal exit was confirmed and the test returned success only after its owned
synthetic fixture cleanup completed. No forced-timeout cleanup/recovery path was
exercised. This disposal exception supplies no application recovery policy.

## Remaining gates

Even a passing result is only component compatibility on this exact runtime.
It does not establish source authority, the complete coupled owner/initializer,
full ancestor/reparse namespace guards, complete paired old/new/unknown disposition,
restart safety, hard journal/scratch bounds, crash/power-loss durability,
macOS/Linux support or any production profile/feature activation. The directory
case tests the held leaf only. Page/max-page settings reduce the toy workload;
they do not prove hard auxiliary resource limits.

## Sanitized result JSON

```json
{
  "implementation_sha": "026592ad3b2d352210682e33441fd261d11f7d31",
  "plan_commit": "14c9de4e844589923218f5cb1ab2177863580f9f",
  "executable_sha256": "5f5a963849ba760c0adc85b8232e53ff94bf4b4094e3c6b1d8c82ce1d15eeac0",
  "rustc": "rustc 1.99.1 (58cf8f9c3 2026-10-13)",
  "cargo": "cargo 1.99.1 (5f94df478 2026-08-27)",
  "runner_image_version": "20261030.294.1",
  "runner_class": "windows-latest",
  "sqlite_dependency": "libsqlite3-sys-0.30.1",
  "rusqlite_dependency": "0.32.1",
  "started_utc": "2026-11-06T11:43:18.5552432Z",
  "ended_utc": "2026-11-06T11:43:18.7108357Z",
  "native_process_starts": 1,
  "timeout_seconds": 120,
  "terminated": true,
  "timed_out": false,
  "output_overflow": false,
  "exit_code": 0,
  "complete": true,
  "success": true,
  "invalid_record": false,
  "records": [
    {"runtime": "sqlite", "version": "3.46.0"},
    {"filesystem": "NTFS", "vfs": "win32"},
    {"case": "original_roles", "code": 0, "expected": true, "observation": "main_original", "observed": true},
    {"case": "original_roles", "code": 0, "expected": true, "observation": "hierarchy_original", "observed": true},
    {"case": "original_roles", "code": 0, "expected": true, "observation": "distinct", "observed": true},
    {"case": "original_roles", "code": 0, "expected": true, "observation": "toy_rows", "observed": true},
    {"case": "original_roles", "code": 0, "expected": true, "observation": "main_after_commit", "observed": true},
    {"case": "original_roles", "code": 0, "expected": true, "observation": "hierarchy_after_commit", "observed": true},
    {"case": "original_roles", "complete": true},
    {"case": "swapped_foreign", "code": 0, "expected": true, "observation": "swapped_main_denied", "observed": true},
    {"case": "swapped_foreign", "code": 0, "expected": true, "observation": "swapped_hierarchy_denied", "observed": true},
    {"case": "swapped_foreign", "code": 0, "expected": true, "observation": "foreign_main_denied", "observed": true},
    {"case": "swapped_foreign", "code": 0, "expected": true, "observation": "foreign_hierarchy_denied", "observed": true},
    {"case": "swapped_foreign", "complete": true},
    {"case": "file_sharing", "code": 32, "expected": true, "observation": "held_rename_denied", "observed": true},
    {"case": "file_sharing", "code": 32, "expected": true, "observation": "held_delete_denied", "observed": true},
    {"case": "file_sharing", "code": 0, "expected": true, "observation": "identity_retained", "observed": true},
    {"case": "file_sharing", "code": 0, "expected": true, "observation": "released_rename_succeeded", "observed": true},
    {"case": "file_sharing", "complete": true},
    {"case": "directory_sharing", "code": 32, "expected": true, "observation": "held_rename_denied", "observed": true},
    {"case": "directory_sharing", "code": 0, "expected": true, "observation": "released_rename_succeeded", "observed": true},
    {"case": "directory_sharing", "complete": true},
    {"case": "query_failure", "code": 1, "expected": true, "observation": "unknown_schema_denied", "observed": true},
    {"case": "query_failure", "complete": true},
    {"case": "partial_creation", "code": 80, "expected": true, "observation": "second_create_denied", "observed": true},
    {"case": "partial_creation", "code": 0, "expected": true, "observation": "first_retained", "observed": true},
    {"case": "partial_creation", "complete": true}
  ]
}
```
