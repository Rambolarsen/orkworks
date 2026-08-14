# Task 2 report — Reset and re-arm the terminal label lifecycle

Commit: `8f087f9 feat: reset session labels for declared commands` (branch `label-reset-commands`, not pushed).

## What I implemented

**`crates/orkworksd/src/main.rs` (Step 3)**

- Added `pub(crate) struct LabelHint { text: String, epoch: u64 }` (`Clone, Debug, PartialEq, Eq`) beside `PeonState`, documented as a queued `InputLabel` refinement tagged with the label epoch it was queued under.
- Changed `PeonState::label_hint` to `StdRwLock<HashMap<String, LabelHint>>` and added `label_epochs: StdRwLock<HashMap<String, u64>>`.
- Initialized `label_epochs` in application startup and in all 35 other explicit `PeonState` literals across the crate (production + tests).

**`crates/orkworksd/src/runtime/terminal_runtime.rs` (Step 4)**

- `reset_command_for_persisted_harness(state, id, line) -> bool` — reads the session's durable `SessionMetadata.harness`, returns false for an absent metadata record, an empty harness ID, or an ID the resolved catalog doesn't know, and otherwise returns true only when `line.trim()` equals one entry of that definition's `label_reset_commands` (byte-exact; no prefix match, no case folding, no fallback to `SessionInfo.harness_id`).
- `reset_label_for_declared_command(state, id, line) -> bool` — returns false immediately when the command isn't declared. For a declared command it takes the `label_epochs` **write** guard and holds it across the whole reset: `saturating_add(1)` on the epoch, remove from `label_hint` and `label_pending`, write `placeholder_label(id)` to durable metadata (when present), then to `SessionInfo`. It does not record the command as `last_user_input`.
- `pub(crate) fn queue_label_hint(state, id, line: String)` — holds the `label_epochs` **read** guard across both the epoch capture and the `LabelHint`/`label_pending` insert, so a reset cannot interleave between the two.
- In `record_terminal_input_impl`, right after `is_sensitive` is known and before `label_worthy` is computed:
  ```rust
  if !is_sensitive && reset_label_for_declared_command(state, id, &line) {
      return Some(());
  }
  ```
  The ordinary descriptive-seeding path is unchanged except that its hint/pending insert now goes through `queue_label_hint`.

**Lock order** (Task 3 must take the same order): `label_epochs` → `workspace` (metadata) → `sessions`. Nothing in this change holds the workspace or sessions lock while acquiring `label_epochs`: the harness lookup takes and releases the workspace lock before the epoch guard, and `queue_label_hint` is called with no other lock held.

## Tests

New tests in `runtime::terminal_runtime::tests`, plus fixtures `set_harness`, `set_label`, `seed_label_hint` (the brief's four-arg test fixture, renamed per the controller's ruling so it doesn't collide with the production `queue_label_hint`), `live_label`, `stored_label`, `label_epoch`, and a shared `assert_label_lifecycle_untouched` negative assertion (label live + durable, hint value *and* epoch, pending flag, session epoch):

- `declared_reset_replaces_live_and_persisted_label_and_rearms_seeding` — the brief's test verbatim (with `seed_label_hint`), including the follow-up descriptive line re-seeding both label copies.
- `near_miss_reset_commands_leave_the_label_lifecycle_untouched` — table-driven over `"/new extra\r"`, `"new\r"`, `"/NEW\r"`.
- `declared_reset_is_scoped_to_the_harness_that_declares_it` — `/new` inert for `codex`.
- `empty_persisted_harness_id_never_resets_even_with_a_live_harness_id` — empty `SessionMetadata.harness` while `SessionInfo.harness_id = Some("claude-code")`, pinning the no-fallback rule.
- `unknown_persisted_harness_id_never_resets` — persisted `"mystery-tool"`.
- `sensitive_input_never_resets_the_label` — pre-dispatch `is_sensitive = true` via `record_peon_input_side_effects`.
- `declared_reset_command_is_not_recorded_as_last_user_input`.

Results: `cargo test --manifest-path crates/orkworksd/Cargo.toml runtime::terminal_runtime::tests` → **56 passed, 0 failed**. Full suite `cargo test --manifest-path crates/orkworksd/Cargo.toml` → **595 passed, 0 failed**. `cargo build` warning count is unchanged from `HEAD~` (one pre-existing `never used` warning); `cargo clippy --all-targets` produces no diagnostic naming any new symbol or line.

## TDD Evidence

**RED (1) — tests written before any state or behavior existed:**

```
$ cargo test --manifest-path crates/orkworksd/Cargo.toml runtime::terminal_runtime::tests
error[E0422]: cannot find struct, variant or union type `LabelHint` in the crate root
    --> src/runtime/terminal_runtime.rs:1850:20
error[E0609]: no field `label_epochs` on type `PeonState`
    --> src/runtime/terminal_runtime.rs:1863:14
     = note: available fields are: `last_output`, `last_inference`, `in_flight`, `label_hint`, `label_pending` ... and 3 others
error: could not compile `orkworksd` (bin "orkworksd" test) due to 4 previous errors
```

Expected: the epoch-bearing hint type and the epoch map are Step 3 work that did not exist yet.

**RED (2) — behavioral, with the state type in place but the reset branch removed** (I re-ran this after implementing, by temporarily replacing the `record_terminal_input_impl` branch with a no-op, to prove the new tests fail for the intended reason and are not vacuous):

```
test runtime::terminal_runtime::tests::declared_reset_replaces_live_and_persisted_label_and_rearms_seeding ... FAILED
test runtime::terminal_runtime::tests::declared_reset_command_is_not_recorded_as_last_user_input ... FAILED

---- ...declared_reset_replaces_live_and_persisted_label_and_rearms_seeding stdout ----
assertion `left == right` failed
  left: "Old conversation title"
 right: "Session label-re"

test result: FAILED. 54 passed; 2 failed
```

Expected: without the reset branch, `/new` is ordinary non-descriptive input — the old title survives and the command is stored as `last_user_input`. (The negative tests pass in this state by construction; they assert nothing changes, and the positive test is what guards them against vacuity.)

**GREEN:**

```
$ cargo test --manifest-path crates/orkworksd/Cargo.toml runtime::terminal_runtime::tests
test runtime::terminal_runtime::tests::declared_reset_replaces_live_and_persisted_label_and_rearms_seeding ... ok
test runtime::terminal_runtime::tests::near_miss_reset_commands_leave_the_label_lifecycle_untouched ... ok
test runtime::terminal_runtime::tests::declared_reset_is_scoped_to_the_harness_that_declares_it ... ok
test runtime::terminal_runtime::tests::empty_persisted_harness_id_never_resets_even_with_a_live_harness_id ... ok
test runtime::terminal_runtime::tests::unknown_persisted_harness_id_never_resets ... ok
test runtime::terminal_runtime::tests::sensitive_input_never_resets_the_label ... ok
test runtime::terminal_runtime::tests::declared_reset_command_is_not_recorded_as_last_user_input ... ok
test result: ok. 56 passed; 0 failed; 0 ignored; 0 measured; 539 filtered out

$ cargo test --manifest-path crates/orkworksd/Cargo.toml
test result: ok. 595 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 21.36s
```

## Files changed

In the brief's list:
- `crates/orkworksd/src/main.rs` — `LabelHint`, `label_epochs`, all `PeonState` literals.
- `crates/orkworksd/src/runtime/terminal_runtime.rs` — reset detection/application, `queue_label_hint`, the `record_terminal_input_impl` branch, new tests, and two existing hint assertions updated to `LabelHint { text, epoch: 0 }`.

Beyond the brief's list, forced by the `label_hint` value-type change (minimum change only; Task 3 owns the epoch-correctness work in these files):
- `crates/orkworksd/src/runtime/peon_runtime.rs` — production: `h.text` in the `[User input]` prompt line and in the `is_usable_input_label` echo gate (`hint.as_deref()` → `hint.as_ref()` + `&hint.text`). Tests: `PeonState` literals and five `LabelHint { text: …, epoch: 0 }` insertions. **No epoch check added** — that is Task 3's.
- `crates/orkworksd/src/http/session_handlers.rs` — `create_session`'s initial-prompt seeding now calls `queue_label_hint` instead of hand-inserting hint + pending (identical behavior for a fresh session, epoch 0, and avoids a second construction site); `PeonState` literals and two hint assertions updated.
- `crates/orkworksd/src/runtime/session_runtime.rs` — one `PeonState` test literal gains `label_epochs`.

`.claude/hooks/doc-check.sh` reports nothing for this diff.

## Self-review findings

- Fixed during review: the brief's four-arg test fixture would have collided with the production `queue_label_hint`; renamed to `seed_label_hint` per the controller's ruling, and it now seeds the epoch map alongside the hint and pending flag so "epoch untouched" in the negative tests means an explicitly present `0`, not merely an absent entry.
- The negative assertions were factored into one helper so every negative case checks all four pieces of state (live label, durable label, hint text+epoch, pending flag, epoch) rather than drifting apart.
- Verified no new compiler or clippy warnings by diffing warning output against the pre-change tree.
- YAGNI check: no epoch consumer was added anywhere (`peon_runtime.rs` still ignores `epoch`), matching the Task 2/3 split.

## Concerns

- `terminal_runtime.rs` is now ~3,200 lines (about +360 from this task, two thirds of it tests). It remains one file per the instruction not to restructure, but it is well past a comfortable size and is worth a split (transport vs. accepted-input bookkeeping vs. tests) as separate work.
- The lock order this task establishes — `label_epochs` → `workspace` → `sessions` — is a convention enforced only by review. Task 3's stale check in `peon_runtime.rs` must take the epoch read guard *before* the workspace/sessions locks it already uses inside `spawn_blocking`, or the two paths can deadlock.
- `LabelHint::epoch` currently has no production reader; that is intended (Task 3 adds it) but means the epoch's correctness is only pinned by terminal-runtime tests until Task 3 lands.

---

# Fix report — review round 1 ("Needs fixes")

Test-only changes; no production line was touched (`git diff` on `terminal_runtime.rs` shows additions only, zero deletions). Committed separately as a `test:` commit rather than an amend, so the review finding and its fix stay legible in history.

## What I changed

All in the `runtime::terminal_runtime::tests` module of `crates/orkworksd/src/runtime/terminal_runtime.rs`.

**IMPORTANT — epoch-carrying hint was untested.** Extended `declared_reset_replaces_live_and_persisted_label_and_rearms_seeding`: after the second `record_terminal_input`, it now asserts the queued hint is exactly `LabelHint { text: "fix the next login bug".into(), epoch: 1 }`. This is the first assertion in the crate pinning a non-zero epoch, so `queue_label_hint`'s epoch capture is now covered.

**MINOR 1 — only `claude-code` + `/new` exercised.** Added `each_declared_command_resets_its_own_harness`, a table over `("claude-code", "/clear")`, `("claude-code", "/reset")`, `("opencode", "/clear")`, `("opencode", "/new")`. Each case asserts the full reset: both label copies become the placeholder, the hint and pending flag are cleared, and the epoch is 1.

**MINOR 2 — `saturating_add(1)` accumulation unpinned.** Added `a_second_declared_reset_advances_the_epoch_again`: `/clear` (epoch 1) → descriptive line (hint at epoch 1) → `/reset` (epoch 2, label back to placeholder, work cleared) → descriptive line (hint at epoch 2). This pins both the accumulation and that each new conversation's hint carries the current epoch.

## Non-vacuity verification

Each new assertion was checked against a deliberately broken production variant, then the file was restored from a backup.

```
$ # A: queue_label_hint's epoch capture replaced with `let epoch = 0;`
    runtime::terminal_runtime::tests::a_second_declared_reset_advances_the_epoch_again
    runtime::terminal_runtime::tests::declared_reset_replaces_live_and_persisted_label_and_rearms_seeding
test result: FAILED. 56 passed; 2 failed; 0 ignored; 0 measured; 539 filtered out

$ # B: `*epoch = epoch.saturating_add(1)` replaced with `*epoch = 1;`
    runtime::terminal_runtime::tests::a_second_declared_reset_advances_the_epoch_again
test result: FAILED. 57 passed; 1 failed; 0 ignored; 0 measured; 539 filtered out

$ # C: command matching restricted to the first declared entry (`.take(1)`)
    runtime::terminal_runtime::tests::a_second_declared_reset_advances_the_epoch_again
    runtime::terminal_runtime::tests::declared_reset_command_is_not_recorded_as_last_user_input
    runtime::terminal_runtime::tests::declared_reset_replaces_live_and_persisted_label_and_rearms_seeding
    runtime::terminal_runtime::tests::each_declared_command_resets_its_own_harness
test result: FAILED. 54 passed; 4 failed; 0 ignored; 0 measured; 539 filtered out
```

Mutant A is the exact regression the reviewer described (`let epoch = 0;` used to leave all 595 tests green — it now fails 2). Mutant C confirms the `/reset` and OpenCode coverage is what catches a `commands[0]`-only implementation; note the pre-existing `/new` case also fails under C because `/new` is the *last* entry of Claude Code's list, but the new table is what makes that intent explicit and covers OpenCode at all.

## Covering test file

- `crates/orkworksd/src/runtime/terminal_runtime.rs` — `runtime::terminal_runtime::tests::{declared_reset_replaces_live_and_persisted_label_and_rearms_seeding, each_declared_command_resets_its_own_harness, a_second_declared_reset_advances_the_epoch_again}`

## Commands run and output

```
$ cargo test --manifest-path crates/orkworksd/Cargo.toml runtime::terminal_runtime::tests
test runtime::terminal_runtime::tests::each_declared_command_resets_its_own_harness ... ok
test runtime::terminal_runtime::tests::a_second_declared_reset_advances_the_epoch_again ... ok
test runtime::terminal_runtime::tests::declared_reset_replaces_live_and_persisted_label_and_rearms_seeding ... ok
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 539 filtered out; finished in 0.06s

$ cargo test --manifest-path crates/orkworksd/Cargo.toml
test result: ok. 597 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 20.43s
```

(58 focused, up from 56; 597 total, up from 595 — the two new tests.)
