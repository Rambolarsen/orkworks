---
name: walking-failure-paths
description: Use when asked to audit robustness or generate resilience-improvement tasks, when a component's behavior under external failure is unknown, or after an incident where a dependency (process, file, port, provider) failed and the app handled it badly.
---

# Walking Failure Paths

## Overview

**Core question: pick a component; now kill its neighbor mid-operation — what does the user see?**

OrkWorks lives on messy external state: hand-editable JSON metadata under `~/.orkworks/`, a sidecar that can die mid-write, harness binaries that may be missing or hang, local inference endpoints that may be down. This audit enumerates one component's external dependencies, breaks each one *on paper against the actual code*, and files issues where the traced outcome is silent corruption, a stuck UI, or a lie to the user.

**A failure you did not trace through the code is not a finding.** "What if the file is corrupt?" is a prompt; the finding is the specific code path that swallows the parse error and the specific wrong state the user then sees.

## Process

1. **Bound the area** — one component (e.g. the metadata store, the PTY runtime, the provider manager, the Electron↔sidecar bridge). Start bounded, follow the data flow (see `skills/surfacing-blind-spots/`).
2. **Enumerate its neighbors** across these categories:
   - **Files**: missing, truncated (killed mid-write), corrupt JSON, hand-edited to invalid values, permissions denied, disk full
   - **Processes**: sidecar killed, harness binary absent, child exits nonzero instantly, child hangs forever, zombie child
   - **Network/ports**: port already taken, provider endpoint down, provider responds slowly or with garbage
   - **Time/ordering**: two writers racing, events arriving after the session ended, clock skew in timestamps
3. **Trace each failure through the actual code** — read the error-handling path, not the happy path. Classify the outcome:
   - ✅ handled: error surfaced truthfully, state stays consistent
   - ⚠️ degraded silently: `let _ =`, `unwrap_or_default()`, empty `catch` — the failure vanishes and state drifts
   - ❌ stuck or lying: spinner forever, stale status shown as live, crash
4. **Verify the worst ones for real** where cheap — corrupt a temp-dir metadata file and run the relevant test, or add a quick throwaway test. Only ⚠️/❌ outcomes you verified (or traced with line-level specificity) become findings.
5. **Filter and file** — apply the guardrail filter and issue format from `skills/surfacing-blind-spots/` (Phases 3–4). Each issue names the dependency, the failure injected, the code path (`file:line`), and the observed/traced user-visible outcome.

## Red flags — stop and restart

- A finding phrased as "we should handle errors better" — no injected failure, no traced path, no user-visible outcome
- Filing a ✅ outcome because the handling *looks* inelegant — graceful is graceful
- Inventing failure modes the platform can't produce (verify the OS/API can actually fail that way)
- Proposing retry/timeout frameworks — the issue states the untreated failure; architecture is the implementer's call

## Common mistakes

| Mistake | Fix |
| ------- | --- |
| Only breaking inputs, not timing | Mid-write kills and racing writers find the nastier bugs |
| Tracing to the backend boundary and stopping | Follow to what the *user sees* — the frontend often converts a clean backend error into a silent nothing |
| Treating `tracing::warn!` as "handled" | A log line the user never reads is not surfacing; check what the UI shows |
