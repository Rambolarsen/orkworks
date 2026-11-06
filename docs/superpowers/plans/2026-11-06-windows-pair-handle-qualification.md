---
type: Qualification Plan
title: Bounded Windows pair-handle component qualification
description: Reviewed-plan target for an isolated ignored Rust harness and one manual Windows native attempt, with no admitted pair/source or production activation.
tags: [orkworks, hierarchy, windows, native, qualification]
status: draft
workflow_status: execution-owner-approval-pending
---

# Bounded Windows pair-handle component qualification

Tracking: [#1453](https://github.com/Rambolarsen/orkworks/issues/1453),
[#1176](https://github.com/Rambolarsen/orkworks/issues/1176). Planning base:
`1fd362775d27aad9532248630c009a893a47eb93`.

## Outcome and pass condition

Gather the first actual native evidence for the accepted
[pair-owner handle-binding candidate](../specs/2026-11-06-original-pair-owner-design.md#scoped-owner-protocol-acceptance--2026-11-06):
can the pinned bundled SQLite win32 VFS expose both actual native main-file
handles, match retained original identities and coexist with selected sharing
guards on this Windows/NTFS runner? Success is six explicit component results,
including negative controls, from one exact-head native process. Unsupported or
incomplete evidence is unavailable/failure, never a skipped passing profile.

This is **not** a coupled initializer/publisher execution plan. Toy SQL has no
schema2/format6 headers, source records, permits, assignment/lineage, PairSlot,
PairCore, production constructor or capability. It proves neither source authority,
pair durability after crash, full namespace walking, hard auxiliary/scratch caps,
restart safety nor macOS/Linux support. Existing native/source/resource gates stand.

Only this plan and the unchanged design acceptance are being written now. Harness/
workflow implementation and native execution require the owner's explicit plan
approval. No database creation or candidate native probe runs while preparing/reviewing
this plan.

## Delivery files and fixed interface

One writer implements the coupled harness/workflow task inline after approval.
Use existing dependencies, scoped Rust instructions/rust-skills and appropriate
review/verification; no production module or Cargo dependency/feature change.

| File | Responsibility |
| --- | --- |
| Create crates/orkworksd/tests/windows_pair_handle_probe.rs | cfg(windows), one ignored integration test named qualify_windows_pair_handles. All helpers private to that test target; explicit existing-file-only win32 opens, borrowed per-schema handle query and native identity comparison |
| Create .github/workflows/pair-handle-probe.yml | workflow_dispatch only, hosted windows-latest only, contents:read permission, no secrets/self-hosted/push/PR/scheduled trigger. Build exact-head probe and run it once under the limits below |
| Create docs/validation/windows-pair-handle-component-evidence.md after the attempt | Sanitized result, exact source head/build/runtime context and per-case disposition, with explicit non-claims. Index it in docs/validation/index.md |

The probe entry accepts no filenames, database paths, VFS choice or source/owner
input. It internally creates owned temporary roots and fixed synthetic leaves.
Use documented SQLite/Windows calls from the accepted design. No private-layout
cast, replacement native handle, custom VFS/shim, /dev/fd URI or path-only fallback.
All actual handles are queried while the same connection/schema is exclusively
owned; never close a borrowed SQLite handle. An unavailable API/identity or wrong
VFS/version/filesystem fails before toy SQL and produces no qualified result.

Keep #[ignore] on the sole native test and cfg(windows) on the target. Ordinary
cargo test/PR/main verification must not run this probe. CI can compile/list the
target, but native execution is only the manual workflow's explicit ignored test.
Existing required baseline tests may run their usual isolated SQL/native fixtures
when validating harness code; those are not this probe or candidate evidence.

## One attempt and its hard stopping rules

Approval covers implementing/reviewing the two test/workflow files and **one manual
workflow dispatch with at most one native probe executable process**. No automatic
rerun, account/model/provider/VFS/platform fallback or retry after failure.
Compilation/dependency provisioning is separate from that native-process count.
No model/coding-harness invocation, app launch, live sidecar or production database.

- Capture the implementation head and pass it as required expected_sha. The job
  compares its actual checked-out SHA before building/executing. A mismatch stops
  the attempt without a native run; no silent new-head dispatch.
- Maximum job wall time15 minutes, including build; native executable wall time
  at most120 seconds. Use the owned process handle to terminate only that probe
  (and its unexpected child tree) on timeout. Confirm termination before fixture
  disposal. No replacement process while disposition/termination is unknown.
- Build with --locked and --no-run. Parse Cargo's JSON compiler-artifact for this
  integration target to select exactly its executable; no glob/old build reuse.
  Run it directly with qualify_windows_pair_handles --exact --ignored
  --test-threads=1. A second executable start is outside this approval.
- Require actual local NTFS and the planned bundled SQLite3.46.0/win32 interface;
  record actual runner/build versions. Unexpected dependency/image/VFS/FS behavior
  is unavailable and stops, not a skipped pass or unreviewed compatibility change.
- At most six internally owned temporary roots, two main SQLite files per case
  and one additional collision/foreign file where required. Native schema/data
  operations are the fixed cases below: two toy tables and one tiny synthetic
  row each, with no bulk/fill/looping workload. Use memory temp storage; page/max-
  page settings may reduce workload but are not claimed hard auxiliary limits.
- Native stdout/stderr and any retained result artifact are at most256 KiB.
  Emit only fixed phase/case tags, booleans, numeric OS/SQLite error codes and
  source/toolchain/runtime version context. No raw handles/file IDs, paths,
  usernames, environment dump, tokens, hidden reasoning or application data.
  Bound the report before writes; unexpected unbounded output fails the attempt.

These are experiment execution/workload limits, not new production quotas or
native profile guarantees. No stress, fault-injected crash, power-loss, exhaustion,
full initializer, source admission or multi-platform qualification is included.

## Cases and explicit oracles

Execute in order, one process, stopping on the first unexpected result. Missing
later cases make the report incomplete, not successful.

| Case | Fixed native action | Required observation |
| --- | --- | --- |
| Original roles / toy commit | CREATE_NEW two synthetic files with retained guards; open main and attach hierarchy via win32 existing-file-only access. Query actual native handles and full FILE_ID_INFO. Commit one toy row per schema and query handles/rows again | Both handles match their retained originals, differ from each other and keep exact roles; toy rows agree. This is no accepted pair initialization or durability test |
| Swapped/foreign identities | Compare those actual schema identities against swapped expected roles and a separately created owned foreign file, without writing the foreign file through SQLite | Comparisons deny both variants; no path/name equality fallback |
| File sharing guard | While original creation/SQLite handles are held, attempt rename and delete of the owned main file. After connection/guards close, perform the same owned rename as a control | Guarded operations deny and identity stays exact; released control succeeds. Failure of the control is unavailable evidence, not proof of a guard |
| Directory sharing guard | Hold the proposed no-delete-sharing handle for an owned leaf directory, attempt its rename, then release it and repeat within the owned test root | Held rename denies; released control succeeds. This tests this leaf only, not complete ancestor/reparse traversal |
| Query failure | Ask sqlite3_file_control for a schema name not attached to the same connection; check error before consuming any output handle | Fail closed, no handle use or fallback. If the real intended opcode is unsupported, case1 already stops; do not install a custom VFS to force support |
| Partial creation | Retain a newly created first file and preoccupy the second fixed leaf with an owned collision file; attempt CREATE_NEW second file | Second creation denies; first guard/identity remains retained, no SQLite initialization or automatic retry/recreation. This observes native facts, not a production PairCore state machine |

Every helper must propagate unexpected failures with redacted numeric/context
codes rather than formatting native identities or paths. The integration test
may not import production admission factories or represent any result as
FreshPairOwner, PreparedChild, source validity, schema2/format6 acceptance or a
supported profile. Keep code changes confined to this component experiment.

## Disposable-fixture cleanup exception and evidence

Explicitly authorize disposal of these **synthetic owned test fixtures only**
after observations and after all probe/SQLite/native handles are closed, including
a failed case. This test-fixture exception does not relax StoppedOccupied or
Unknown for any application pair, source, namespace, epoch or user file. No
persistent failed-attempt marker or runtime cleanup helper is added.

If the process times out, terminate it and confirm exit before attempting cleanup
inside its recorded owned test roots. If ownership/termination cannot be proved,
leave those test roots and report failure; do not delete a replacement path or
start another process. Runner teardown is not counted as a native recovery proof.
No test data is imported, registered or copied into actual OrkWorks storage.

Retain a bounded sanitized JSON summary and concise Markdown evidence: plan/
implementation SHA, executable digest, dependency/SQLite/toolchain/runner versions,
actual FS/VFS classification, start/end/exit/timeout, six case tags and expected/
observed booleans/error codes. Do not retain DB contents, raw native IDs/paths or
broad CI environment. A failed or unavailable result is retained as such.

The evidence conclusion is only documented-handle/component compatibility on
that exact runtime. It cannot elect a source owner, establish the native creation
protocol's full namespace guards, prove full paired old/new/disposition, clear
strict commit/revocation gates, restore a permit or activate any feature.

## Implementation sequence and verification

Use two bounded phases of this same approved plan. Phase1 delivers the ignored
harness/manual workflow through its own reviewed PR, then stops at the repository
terminal-state handoff. [GitHub requires the manual workflow to exist on the default branch](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/manually-run-a-workflow)
before dispatch; do not try to execute an unregistered new
workflow from an arbitrary PR. Phase2 resumes with the exact merged implementation
head and unused one-dispatch/one-process authorization, runs the component attempt
and delivers evidence through its owned evidence PR. Track dispatch/attempt state
in the implementation issue/handoff; a session change never resets the budget or
requires approval again within unchanged scope. No background run is implied.

- [ ] Implement the test-only ignored harness and manual-only workflow against the
  fixed cases/limits. First check the test entry/ignore marker, no production
  imports/constructor or trigger and read-only workflow permissions. Keep the
  source guard and report/cleanup rules visible at the private helper seam.
- [ ] Run the applicable Rust build/test/fmt and bash scripts/verify-repo.sh with
  the existing required Node/desktop setup, without running the ignored probe.
  Verify it is absent on non-Windows and ignored by default on Windows. Review
  workflow/code through the repository's current /code-review low gate (escalate
  only for documented risk/size), plus required CI and complete PR feedback.
- [ ] At the explicit phase2 handoff, capture exact reviewed merged implementation
  head and confirm the one attempt is unused, then make the single manual
  dispatch with expected_sha. Verify workflow/run identity and head before
  attributing any result. Inspect the one native process outcome, all six case
  records and limits; no result from another head/platform counts.
- [ ] Write the sanitized evidence/disposition and link it to #1176 and this plan;
  run appropriate docs/code checks and complete the owning PR lifecycle. A red or
  blocked probe is a valid recorded investigation outcome, not permission to
  widen/retry the plan. No production eligibility or activation claim.

## Inline planning review and investigated risks

Least certain: actual Windows/SQLite handle/guard compatibility. Static checks
confirm the documented opcode in bundled libsqlite3-sys0.30.1/SQLite3.46.0 and
existing Windows dependencies; native behavior is the experiment, not assumed.
The rust integration target and manual ignored entry avoid actual source-owner
factories. Existing Windows CI uses filtered cargo tests; ignoring this entry
keeps ordinary verification from becoming an unapproved native attempt.

Main project blind spot: a successful toy transaction could be mislabeled pair
qualification, or test disposal could become a runtime cleanup policy. The fixed
schema names/absence of source types, explicit six-case evidence conclusion and
synthetic-only cleanup exception address that distinction. Namespace/restart/
auxiliary/durability/source-cutoff behavior stays outside the result.

| Dimension | Rating / evidence |
| --- | --- |
| Dependencies |3: known bundled SQLite/Windows APIs and existing hosted CI; actual native compatibility remains to test |
| Blast radius |2: ignored test target and manual workflow, no production wiring |
| State changes |2: disposable synthetic native files/toy SQL only after explicit approval |
| Reversibility |2: branch revert plus owned-fixture disposal/confirmed termination; no user store changes |
| Uncertainty |4: supported-interface source evidence, untested native candidate |

Sum13 is a judgment aid, not an approval threshold. Inline scope/clarity/coverage
review: each intended component observation maps to one of the six cases, error/negative
controls and head/process/evidence limits; no full initializer or native guarantee
is inferred. Disposition: **Ready for scoped owner execution approval**, not
approved to implement/dispatch. No extra technical review of the unchanged owner
design is claimed. If the build/runtime cannot meet this exact plan, stop and
report the concrete conflict before dependent changes or another native attempt.
