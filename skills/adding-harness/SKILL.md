---
name: adding-harness
description: Use before adding or changing an OrkWorks harness adapter so launch, resume, native session ID capture, hooks, status probes, voice, capacity, tests, and docs are reviewed consistently.
---

# Adding Harnesses

Use this skill before adding a new harness or changing an existing harness adapter.

## Where harness code lives

- `crates/orkworksd/src/harness/definition.rs` — `HarnessDefinition` (launch/resume command templates, capability flags, usage-limit patterns, `labelResetCommands`) and the `harnesses-v2.json` builtin document it parses from `crates/orkworksd/resources/harnesses-v2.json`
- `crates/orkworksd/src/harness/registry.rs` — builtin resolution and persistence of user-defined configs to `~/.orkworks/harnesses.json`
- `crates/orkworksd/src/providers.rs` — Peon inference provider definitions (`builtin_provider_registry()`). This is currently a separate registry: a harness that should also serve as a Peon inference tool needs a matching `ProviderDefinition` here. (A unification that derives providers from `HarnessConfig` is designed but not yet implemented — `docs/superpowers/specs/2026-07-03-harness-registry-unification-design.md`.)
- `crates/orkworksd/src/http/harness_handlers.rs` — harness CRUD endpoints
- `apps/desktop/src/harnessTypes.ts` — renderer-side harness types

## Required Checks

1. Confirm the harness is covered by an authoritative OrkWorks spec or create/update the spec first.
2. Record the launch command, required working directory behavior, model argument syntax, and whether OrkWorks must preserve the selected model string exactly.
3. Prefer launching the harness binary directly in `command-template` definitions. Do not wrap it in a login shell (`bash -lc`, `sh -l`): login-shell profile output (nvm/rbenv init, MOTDs) arrives before the harness's first render and, for hookless harnesses, confounds first-turn work-signal inference. OrkWorks tolerates roughly the 2-second startup attention grace of banner output (issue #390), but a direct spawn avoids the class entirely.
4. Verify exact resume support from primary documentation or a local CLI help command. Record the command shape.
5. Verify latest-session fallback semantics. If undocumented, do not invent fallback behavior.
6. Identify native session ID capture sources in reliability order:
   - environment variable
   - hook JSON payload
   - structured JSONL event
   - documented status command
   - deterministic output parser
   - manual entry
   - Peon inference
7. Mark any capture path that types into the harness session or writes harness config as user-approved only.
8. Record provider/model detection behavior and whether Peon is allowed to infer missing fields.
9. Record native voice support. Voice must remain pass-through unless a spec explicitly says otherwise.
10. Record capacity/context/status signals the harness exposes and whether they are documented enough to parse.
11. Record in-session label-reset commands per ADR 0040 (`HarnessDefinition.labelResetCommands`): verify the harness's start-fresh commands against primary documentation or the installed CLI, declare them with cited evidence, or explicitly declare none — an empty declaration must be verified-absent, never unexamined.
12. Add or update tests for launch command rendering, resume strategy selection, session ID capture, label-reset command declarations, and remembered-session UI state.
13. Update `docs/agents/architecture.md`, relevant specs, and ADRs if the adapter adds routes, metadata fields, protocol changes, or new boundaries.

## Output

Before implementation, write a short harness adapter note in the relevant spec or plan with:

- harness ID
- adapter ID
- launch command
- exact resume command
- latest fallback behavior
- native session ID capture source
- confidence/source string for capture
- user-approval requirements
- label-reset command declaration (with cited evidence, or verified-none)
- test files to update
