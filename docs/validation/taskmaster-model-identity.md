---
type: Validation Evidence
title: Taskmaster model identity trial
description: Two bounded Codex inference calls distinguish a display-name model rejection from a canonical-ID success without proving a historical failure's cause.
tags: [orkworks, taskmaster, models, validation]
status: stable
---

# Taskmaster model identity trial

Tracks [#1076](https://github.com/Rambolarsen/orkworks/issues/1076), after the
settings fix in [#1457](https://github.com/Rambolarsen/orkworks/pull/1457).
This is source/profile evidence, not published-installer qualification.

## Result and limit

On 2026-11-06, the current Codex CLI/account rejected `GPT-6.1-Sol` and accepted
`gpt-6.1-sol` in otherwise equivalent synthetic inference requests. The rejected
call explicitly reported that the supplied model was not supported. This
establishes the model-identity effect for this trial only.

It does **not** prove the cause of the earlier Analyze now failure. The issue
reports `request validation error (CLI exit 1)` for that attempt; this trial produced a different
pre-fix summary, `provider failure (CLI exit 1)`. The historical full provider
error and exact request were not recovered or replayed. Do not describe the
historical mismatch as confirmed causality.

## Authorized method

The owner authorized two bounded inference calls after an explicit request to
except this live CLI diagnostic from the session's no-additional-harness rule.
The diagnostic used the existing Taskmaster native inference profile, not an
interactive coding session. It made exactly two `ProcessRunner::run_prepared`
inference invocations, with no diagnostic-level retries or fallback. Separate
`--version` probes did not request inference. Each inference had a 90-second
process-runner timeout and used low reasoning effort.

- Source commit: `3b66b37dc519322b8813dc794c1411064b3c5952`.
- Profile source: `crates/orkworksd/src/providers/inference.rs`, SHA-256 `292ec7d280e5aded2ca5fa68a1518ef1574626bf6a51a896cbdc1e53154f2dd9` at that commit.
- Installed CLI: `codex-cli 0.166.0`; the production compatibility gate passed.
- Both calls used production `prepare`, `check_version`, `ProcessRunner` and
  decoding, from a temporary ignored test in the inference module. That
  diagnostic was removed after execution; it is not part of normal tests.
- The supplied model was the only input varied; the production profile also
  derives its one-model catalog from that ID. Each call had a private temporary
  working directory, the existing read-only sandbox and disabled optional tools.
- Existing login storage and administrator-managed policy remained authoritative
  under [ADR 0054](../adr/0054-taskmaster-honors-managed-cli-policy.md). Credentials
  were not copied. This is not a zero-side-effect certification for managed policy.
- The diagnostic invoked no Taskmaster/Peon settings or live workspace-mutation
  API and did not edit managed policy. Managed-policy side effects were not
  separately audited. The prompt contained no repository or private workspace data.
- Raw output was captured in a fresh mode-0700 directory with create-new,
  mode-0600 files. Only the synthetic response, fixed error text and hashes are
  retained here; thread identifiers and other raw diagnostics are omitted.

Both prompts were exactly:

```text
This isolated inference diagnostic supplies no repository data. Return exactly the JSON object {"ok":true}. Do not invoke tools, execute commands, edit files, or start coding sessions.
```

## Observations

| Supplied model | CLI exit | Elapsed | Outcome |
| --- | --- | --- | --- |
| `GPT-6.1-Sol` | 1 | 2839 ms | Unsupported model; no completed answer |
| `gpt-6.1-sol` | 0 | 6661 ms | Decoded `{"ok":true}` |

The rejected call emitted both an `error` event and a terminal `turn.failed`
event with this same message:

```text
The 'GPT-6.1-Sol' model is not supported when using Codex with a ChatGPT account.
```

Both captured stderr streams were empty. The successful response passed the
production decoder, which rejects unexpected tool-action events.

- Trial 0: recorded `2026-11-06T13:03:10+00:00`; captured stdout UTF-8 SHA-256 `104610d329c59ca2b8051519d7573afc64753044dea12fa0f084c7d5b506336d`; private record SHA-256 `b1f806628004e54de2e8617db9124b2fca7904d6192775a21b804433d3686525`.
- Trial 1: recorded `2026-11-06T13:03:16+00:00`; captured stdout UTF-8 SHA-256 `60e10abbb9d1c9d80a6635ff9d168d1393a08edf5f5ff6c918eecdc3396dbf4f`; private record SHA-256 `aaef5f67229078ba6e69c4886be066a549416341b804b0ec909c043359318dc8`.

## Diagnostic follow-up

The trial exposed a narrow classification gap: the known-model matcher did not
recognize `model is not supported`. A regression using the observed message in
stderr and terminal JSONL failed with `ProviderFailure` before the fix. The
matcher now returns `ModelFailure` and the fixed summary
`model failure (CLI exit 1)`, without exposing the supplied model or login type.
No additional live call was used to verify this classifier change.
