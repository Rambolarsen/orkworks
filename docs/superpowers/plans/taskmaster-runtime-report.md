# Taskmaster runtime implementation report

This worker report describes the pre-review implementation. The root's later
integration corrections and current blockers are tracked in
[the implementation plan](2026-09-09-taskmaster-knowledge.md). In particular,
the inference-duration lease, atomic generation-guarded commit, current-fact
revalidation, strict configuration-file allowlist, relevant knowledge retrieval,
and both handoff prompts were tightened after this report.

## Implemented surface

- `GET`/`POST /settings/taskmaster` and `POST /settings/taskmaster/knowledge`
  are registered in the sidecar. Every route requires the existing
  `x-orkworks-open-plan-token` / `ORKWORKS_OPEN_PLAN_TOKEN` authority check.
- Settings use the fixed contract names and defaults, persist under
  `~/.orkworks/taskmaster/settings.json`, apply canonical-path workspace
  overrides, and preserve an explicit override `selection: null` as clearing
  the inherited selection.
- Knowledge bundles validate format version, ISO timestamp, relative Markdown
  page IDs, SHA-256 page content hashes, related-page closure, monotonic
  sequence, and bounds (256 pages, 64 KiB/page, 2 MiB aggregate). This aligns
  with the publisher limits.
- Evaluation accounting persists before inference, uses a process mutex plus
  an OS advisory lock at `~/.orkworks/taskmaster/.evaluation.lock`, limits by
  UTC day and workspace interval, and fails closed when its ledger is corrupt.
- Provider reuse is factored into an explicit-prompt seam. Only the Ollama HTTP
  transport is allowed today; arbitrary CLI/provider profiles return the
  structured `unsupported_capability` error. It does not build a Peon prompt
  or change Peon state.
- Recommendation JSON now supports defaulted `repositoryEvidence` and
  `knowledgeEvidence`, retaining legacy observation evidence unchanged.
- The existing debounce now starts a single-flight, reservation-gated model
  enrichment call. Its response is strict JSON and can attach only supplied
  knowledge IDs to still-`proposed` recommendations; accepted/completed/dismissed
  records are not rewritten. Settings/knowledge generation is checked before
  applying output.
- Repository context is bounded to 32 regular UTF-8 files, 8 KiB/file and
  64 KiB aggregate at depth five. It canonicalizes every target, rejects
  symlinks and configured/built-in sensitive paths, and consults git's ignore
  rules. Source-code context expands the allowed extension set; session-only
  context reads no repository files.
- Model output can create at most three proactive recommendations only when it
  cites supplied repository fact hashes. Their identity is server-derived from
  target surface plus sorted fact hashes, with zero recurrence and no source
  sessions. Unsupplied repository or knowledge citations are rejected.
- The persisted input cache key now gates unchanged evaluation inputs, and a
  five-minute periodic poll runs alongside the existing debounced path.

## Verification

- Red: the initial runtime tests failed to compile before the runtime types and
  validator existed.
- Green: focused `taskmaster::runtime::tests` (4 tests) passed, including bad
  digest, corrupt-ledger fail-closed, and `selection: null` override behavior.
- Full `cargo test --manifest-path crates/orkworksd/Cargo.toml` passed: 1,062
  unit tests and 3 script tests.
- `cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check` and
  `cargo build --manifest-path crates/orkworksd/Cargo.toml` passed.

## Remaining contract gaps

- Runtime views are re-opened per handler rather than held by an `AppState`
  manager. Durable state is reloaded under the global advisory lock for every
  reservation/mutation and stale generation rejects result writes, but a
  provider call can still consume a reservation after a just-raced config
  change; this is safe but not ideal.
- Only Ollama is enabled for tool-free Taskmaster calls. The verified compiled
  Claude profile supplied by the root task needs a dedicated implementation;
  no arbitrary Peon/custom CLI template is used.
