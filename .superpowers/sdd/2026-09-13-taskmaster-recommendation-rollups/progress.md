# SDD ledger — plan: docs/superpowers/plans/2026-09-13-taskmaster-recommendation-rollups.md

## Pre-flight plan scan

| Tasks | Shared file/interface | Finding | Ruling |
|---|---|---|---|
| 1 and 2 | `specs/taskmaster.md`, observation identity contract | Task 1 documents the v2/legacy contract consumed by Task 2. No contradiction. | Task 1 must land before Task 2 changes behavior. |
| 2 and 3 | `WorkflowObservation`, `WorkflowObservationEvidence`, `problemArea` | Task 2 adds optional observation identity; Task 3 carries it into recommendation evidence. No contradiction. | Preserve `None` for legacy records and default it during deserialization. |
| 3 and 4 | `Recommendation`, `RecommendationStatus`, parent/member fields | Task 3 defines schema; Task 4 persists and validates it. No contradiction. | Store validation is authoritative at read/transaction boundaries. |
| 3 and 5 | `RollupFamilySnapshot`, `stable_rollup_id`, cluster validation | Task 3 produces pure helpers; Task 5 supplies model output and applies validated results. No contradiction. | Keep model I/O out of pure validation. |
| 4 and 5 | `RecommendationStore::apply_rollup_transaction` | Task 5 depends on Task 4’s atomic graph write. No contradiction. | Do not add evaluator writes that bypass the transaction. |
| 4 and 7 | retention/lifecycle graph invariants | Task 7 verifies Task 4’s recovery and cleanup behavior. No contradiction. | Failure tests are required before declaring the store complete. |
| 5 and 6 | list filtering, rollup status, generated fix prompt | Task 5 persists parent/member state; Task 6 projects it through HTTP/UI. No contradiction. | API filtering must use persisted graph invariants, not renderer-only suppression. |

| Task | Self-consistency scan | Ruling |
|---|---|---|
| 1 | Documentation-only files and doc check agree. | Proceed. |
| 2 | Candidate/evidence field changes and focused Rust tests agree. | Proceed. |
| 3 | Schema, pure validation functions, and test surface agree. | Proceed. |
| 4 | Transaction API, recovery behavior, and store tests agree. | Proceed. |
| 5 | Token, model validation, scheduler integration, and evaluator tests agree. | Proceed. |
| 6 | Rust DTO, TypeScript types, renderer behavior, and API tests agree. | Proceed. |
| 7 | End-to-end/failure tests and required verification commands agree. | Proceed. |

No plan requirement is left without a task. The spec is authoritative if an
implementation detail needs refinement; any ruling that changes the spec must
be recorded here before code is changed.

## Task status

- Task 1: complete
- Task 2: complete
- Task 3: complete
- Task 4: complete
- Task 5: complete
- Task 6: in progress
- Task 7: pending

Task 1: fix round 1/5 (4 addressed, 0 open; commits b63d197..1c71e017)
Task 1: fix round 2/5 (1 addressed, 0 open; commits 1c71e017..848fa586)
Task 1: complete (commits bb500945..848fa586, review clean)
Task 2: minor (deferred): update workflow-observation handler module docs to mention optional `problemArea`.
Task 2: complete (commits 848fa586..7683790b, review clean)
Task 3: fix round 1/5 (2 addressed, 0 open; commits e344a9c..11b0ab5)
Task 3: note: focused Task 3 tests and formatting passed; the worker's full sidecar run reported two unrelated existing provider-test failures.
Task 3: complete (commits 7683790b..11b0ab5, review clean)
Task 4: Ruling: `apply_rollup_transaction` expected state is a sorted `BTreeMap<String, Option<String>>`, where `Some(hash)` requires matching current JSON bytes and `None` requires absence; parent/member inputs are immutable — chosen to make optimistic concurrency explicit before staging, at the cost of adding a concrete public store contract the implementation must preserve.
Task 4: fix round 1/5 (6 addressed, 0 open; commits 11b0ab5..55739338)
Task 4: fix round 2/5 (1 addressed, 0 open; commits 55739338..57987ab0)
Task 4: fix round 3/5 (2 addressed, 0 open; commits 57987ab0..cef243a5)
Task 4: verification: taskmaster evaluator tests 12 passed; store tests 23 passed; formatter and diff checks passed.
Task 4: complete (commits 11b0ab5..cef243a5, review clean)
Task 5: Ruling: a rollup result may update an existing active parent for the same deterministic member set; a fresh snapshot may also supersede a proposed parent and reassign its members atomically, while any result captured before the active parent graph changed is stale and rejected because the supplied snapshot has no authority over a newer graph.
Task 5: fix round 1/5 (4 addressed, 0 open; commits 012cf875..ffc8359)
Task 5: fix round 2/5 (4 addressed, 0 open; commit 50a6fca)
Task 5: fix round 3/5 (2 addressed, 1 intentionally rejected as conflicting with the no-changed-active-parent contract; current fixes uncommitted)
Task 6: fix round 1/5 (2 addressed, 0 open; commit 8df3aff..5833cca)
Task 6: fix round 2/5 (4 addressed, 0 open; commits 2e8f686..837bc07; active-parent snapshot/revalidation refinement included)
Task 6: verification: focused Taskmaster tests 135 passed before final lifecycle refinement; final rollup evaluator tests 14 passed; desktop full suite 741 passed/1 unrelated Windows dialog-drag failure; Rust full suite 1224 passed/10 environment-sensitive failures.
