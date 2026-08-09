# Antigravity CLI harness migration

**Status:** proposed

## Context

Google retired Gemini CLI access for Gemini Code Assist for individuals on
2026-06-18 and directs those users to Antigravity CLI. The existing built-in
`gemini` harness can no longer launch an authenticated session for that tier.

## Decision

Add Antigravity CLI as a distinct built-in harness:

- ID: `antigravity`
- Display name: `Antigravity CLI`
- Launch command: `agy`
- Exact resume command: `agy --conversation={harnessSessionId}`
- Latest-session resume: `agy --continue`, scoped to the current working
  directory

The selected model is not passed at launch until official Antigravity CLI
documentation establishes a stable model-selection argument. Antigravity's
own routing and model configuration therefore remain intact.

The legacy `gemini` built-in remains in the registry solely to resolve
existing persisted settings and historical session metadata. It is marked
retired and excluded from new-session choices. It is not renamed and its ID,
configuration, integrations, historical sessions, and old conversation IDs
are not migrated to Antigravity.

## Adapter evidence and boundaries

Antigravity documents `agy` installation/authentication, project launch, and
conversation resumption. It does not yet establish a version-pinned,
reproducible hook payload or an eligible local configuration target for the
existing OrkWorks reporter installer.

Accordingly, the initial Antigravity adapter supports launch and documented
resume only. It has no compiled session-signal handler, integration installer,
capacity parser, provider/Peon command, native voice binding, or static model
catalog. Session identity may be captured only from documented terminal output
once an exact, version-pinned fixture is added; otherwise OrkWorks falls back
to its normal Peon/process metadata behavior. Voice remains pass-through.

## User experience

The New Session dialog lists Antigravity CLI and never lists Gemini CLI.
Existing Gemini sessions continue to render with their original name and icon;
they can be inspected but are not offered as a new launch target. Existing
user overrides for `gemini` continue to load, but do not make the retired
harness selectable for a new session.

## Error handling

If `agy` is unavailable, launch reports the standard executable-not-found
error. OrkWorks does not install Antigravity automatically and does not
attempt to translate Gemini authentication or conversation state.

## Tests and documentation

The implementation will cover registry resolution, launch rendering, exact and
latest resume selection, legacy Gemini visibility filtering, remembered
new-session selection fallback, renderer icon resolution, and the migration's
documentation/evidence register updates. An ADR is unnecessary because this
adapts an existing registry boundary without adding a route, metadata field, or
new architectural decision.
