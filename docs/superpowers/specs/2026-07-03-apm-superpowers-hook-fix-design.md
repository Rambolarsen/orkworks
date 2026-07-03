# APM And Superpowers Hook Fix Design

## Goal

Update the repo's APM toolchain and `obra/superpowers` package so Codex no longer fails on `SessionStart` with hook exit code `127`.

## Problem

The current repo state uses:

- `apm` CLI `0.20.0`
- `obra/superpowers` commit `8cf39006140a743dce31ba4046fceab90cc214e6` (`v5.1.0`)

In this state, the generated Codex hook bundle still invokes Superpowers through:

- `.codex/hooks/superpowers/hooks/run-hook.cmd session-start`

That generated bundle is incomplete for this repo layout and reproduces the `127` failure path.

## Chosen Approach

Update both layers:

1. Update the local `apm` CLI to its latest available release.
2. Update repo APM dependencies so `obra/superpowers` resolves to current upstream.
3. Regenerate installed artifacts with `apm install`.
4. Verify whether Codex still receives a Superpowers `SessionStart` hook.
5. If it does, apply a narrow repo-local fix by removing only the stale Codex Superpowers `SessionStart` wiring.

## Why This Approach

- Upstream Superpowers release notes for `v6.1.0` state that Codex no longer ships a `SessionStart` hook.
- The failure is in generated runtime artifacts, so metadata-only updates are insufficient.
- Verification must inspect the generated `.codex/hooks.json` and execute the installed hook path directly.

## Success Criteria

- `apm` CLI updates successfully or a concrete blocker is documented.
- `apm.lock.yaml` resolves `obra/superpowers` to a newer upstream revision.
- `apm install` completes far enough to regenerate Codex hook artifacts.
- `.codex/hooks.json` no longer contains a Superpowers `SessionStart` hook for Codex, or a narrow local fix removes it.
- Executing the generated Codex SessionStart hook path no longer exits with code `127`.
