---
name: adding-harness
description: Use before adding or changing an OrkWorks harness adapter so launch, resume, native session ID capture, hooks, status probes, voice, capacity, tests, and docs are reviewed consistently.
---

# Adding Harnesses

Use this skill before adding a new harness or changing an existing harness adapter.

## Required Checks

1. Confirm the harness is covered by an authoritative OrkWorks spec or create/update the spec first.
2. Record the launch command, required working directory behavior, model argument syntax, and whether OrkWorks must preserve the selected model string exactly.
3. Verify exact resume support from primary documentation or a local CLI help command. Record the command shape.
4. Verify latest-session fallback semantics. If undocumented, do not invent fallback behavior.
5. Identify native session ID capture sources in reliability order:
   - environment variable
   - hook JSON payload
   - structured JSONL event
   - documented status command
   - deterministic output parser
   - manual entry
   - Peon inference
6. Mark any capture path that types into the harness session or writes harness config as user-approved only.
7. Record provider/model detection behavior and whether Peon is allowed to infer missing fields.
8. Record native voice support. Voice must remain pass-through unless a spec explicitly says otherwise.
9. Record capacity/context/status signals the harness exposes and whether they are documented enough to parse.
10. Add or update tests for launch command rendering, resume strategy selection, session ID capture, and remembered-session UI state.
11. Update `docs/agents/architecture.md`, relevant specs, and ADRs if the adapter adds routes, metadata fields, protocol changes, or new boundaries.

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
- test files to update
