# Generation-aware harness version probe cache

- Status: accepted
- Deciders: Copilot CLI
- Date: 2026-07-27

## Context

`harness/detect.rs::probe_tool_version` spawns `<executable> --version` with a 3 second timeout. `workspace/integration/status` requests can call it repeatedly for the same harness, and tight polling makes repeated spawns wasteful even when the binary has not changed.

The cache must not weaken the existing TOCTOU protections in `run_integration_action`. A request still has to capture workspace and harness identity, probe outside the locks, then revalidate the identity before it acts on the result.

## Decision

Add an AppState-owned in-memory cache for `probe_tool_version` output.

- The cache key is based on a single process-wide generation counter, plus harness id, launch command, and resolved executable path. Successful workspace switches and successful harness edits/deletes bump that counter. This is intentionally coarse: unrelated mutations can force an extra re-probe, but they can never serve a stale one.
- The cached value is the raw `--version` output (including `None` for spawn failures and timeouts) plus an expiry timestamp.
- `resolve_tool_gate` consults the cache after `probe_installed_tool` resolves a path and before spawning the version probe.
- Successful workspace switches and successful harness edits/deletes bump the generation so stale entries stop matching immediately. That bump is only an invalidation signal; the post-probe workspace/definition revalidation in `run_integration_action` is still what prevents an in-flight request from acting on stale identity.
- The cache remains process-local and ephemeral; no disk persistence is added.

The cache stores probe output, not the final compatibility verdict, so the current `min_version` check still runs on every request against the cached text.

Cache policy:

- Positive probe results use a 30 second TTL.
- Failed and timed-out probes use a 5 second TTL.
- The cache is pruned opportunistically and capped at 64 entries so it stays bounded even if many harness/path combinations are touched.
- Concurrent cache misses are not single-flight deduplicated; the goal is to make repeated polling cheap, not to collapse every race into one probe.

## Consequences

- Tight polling on the same harness becomes cheap when the binary has not changed.
- Cache invalidation stays aligned with the existing identity revalidation rules.
- The implementation adds a small amount of shared mutable state to `AppState`, and `resolve_tool_gate` gains a cache handle plus the generation token it needs to consult it.
- Same-path upgrades are only reflected after TTL expiry, because the executable path stays the same and the cache key does not include filesystem metadata.
- Old cached entries may remain in memory until pruned, but they are never returned once the generation changes or the entry expires.
