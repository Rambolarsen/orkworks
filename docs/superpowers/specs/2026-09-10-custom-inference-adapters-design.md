# JSON-defined Taskmaster inference adapters

Status: accepted for implementation after owner-requested subagent review
Tracking: [issue #503](https://github.com/Rambolarsen/orkworks/issues/503)

## Scope

Extend the existing harness JSON registry with an optional `inference`
capability, independent of interactive launch, resume, and Peon. A custom
adapter does not require Peon support. Existing documents without this field
retain their behavior. Preserve existing built-in Codex, Claude, and Ollama
transports behind the same capability interface. In this version a custom
harness still requires its existing `launch` definition; “independent” means
inference does not consume that definition and does not require Peon. Making
interactive launch itself optional is outside this change.

The owner approved JSON-defined adapters and explicit trust for custom
background executables in conversation. This document specifies the contract
for implementation. The owner requested subagent review, patching, and then
implementation on 2026-09-10. The authoritative Taskmaster knowledge
specification and ADR must record the extension before implementation code.

## Provider configuration and authentication

A model provider configured inside a coding tool is not a new OrkWorks
adapter. Pass the user's selected model identifier unchanged, with no automatic
prefix, provider substitution, or fallback. Keep the CLI's existing home,
configuration, authentication stores, endpoint settings, and credential
environment available. Never copy credential files or require fresh API keys.

The adapter describes how to invoke a tool or user-owned wrapper, not an
allowlist of inference services. This does not automatically remove existing
built-in profile limitations: a profile that cannot preserve a particular
configuration reports incompatibility. The user may explicitly register and
trust a custom wrapper implementing the contract instead. OrkWorks does not
generate wrappers that bypass managed policy.

## JSON contract

Add an optional `inference` field to custom definitions and built-in overrides
in the existing global `harnesses.json`. Example capability fragment:

The field is an additive schema-3 extension. Version-2 documents containing
the field (including null) are rejected with a migration diagnostic; ordinary
version-2 documents still migrate to version 3. Unsupported document versions
remain rejected. Preserve the field across store mutations and renderer editor
round trips. A launch-kind replacement must not skip other fields in a patch.

```json
{
  "inference": {
    "kind": "command",
    "command": "/absolute/path/to/my-inference-wrapper",
    "args": ["--model", "{model}"],
    "input": "stdin",
    "output": "result-json-v1",
    "timeoutSecs": 60
  }
}
```

`kind: command` accepts a direct executable (absolute path or PATH-resolved
name), an argv array, input mode `stdin` or `file`, output mode
`result-json-v1`, and a timeout from 1 to 120 seconds. The default timeout is
60 seconds. An optional `reasoningEffortArgs` array is appended only when the
user explicitly selects an effort. No shell command-string evaluation occurs.

Only `{model}`, `{effort}`, and `{promptFile}` substitutions are supported,
each within a single argv element. Require `{model}` exactly once in `args`;
require `{effort}` exactly once when `reasoningEffortArgs` is present. File
input requires `{promptFile}` exactly once; stdin input forbids it. Reject
unknown fields, unknown placeholders, empty executable/model values, control
characters, more than 64 arguments, and templates over 4 KiB each. Model
identifiers are opaque nonempty strings bounded to 256 bytes; do not restrict
them to a hardcoded provider naming convention.

There is no prompt-in-argv mode, inline script, secret field, environment
override map, arbitrary response expression, regex engine, or plugin loader
in this first version. A CLI with another protocol can be supported by a
user-owned wrapper. Built-in profiles use a closed `kind: builtin` variant
with a known profile identifier, installed by code-owned registry metadata;
user JSON cannot select that variant. User overrides may replace inference
with a complete command definition or clear it with null; omission preserves
the current capability. Partial merges within inference are not supported.
Carry capability kind and origin through provider projection: neither an ID
nor an executable name is sufficient to inherit built-in trust. Overrides of
execution-affecting built-in fields must invalidate the built-in profile unless
its own compatibility rules explicitly support them.

## Execution and response protocol

Use the existing bounded process runner and a private temporary working
directory. Send at most 256 KiB of prompt text through stdin or an owned
temporary file. Keep file and directory alive until child processes and pipe
readers have finished; clean them up on success and failure. Do not inherit
interactive launch/resume arguments or session identifiers. Strip all
`ORKWORKS_*` variables and shell startup variables `BASH_ENV` and `ENV` from
the child. Preserve provider authentication/configuration variables.

Require successful process exit and a single stdout JSON object:

```json
{
  "version": 1,
  "status": "success",
  "result": "{\"enrichments\":[],\"proposals\":[]}"
}
```

`result` is a nonempty string passed to the existing Taskmaster schema and
evidence validator, not accepted directly as a recommendation. Reject extra
top-level fields, duplicate keys in both the envelope and the nested result
JSON, unsupported versions/statuses, additional
stdout text, malformed JSON, empty output, and output over 64 KiB. Stderr is
also bounded to 64 KiB and never returned verbatim in Settings errors.
Timeout/nonzero exit/protocol errors do not trigger a provider fallback or
modify Peon. Existing Taskmaster reservations count failed model invocations.

Built-in decoders continue to validate their native terminal/error/tool events.
The custom response envelope cannot prove that an arbitrary executable did not
perform actions; that distinction must remain visible.

## Trust and policy

Custom adapters are disabled for background inference until the user approves
them in Settings. Importing/editing JSON, model output, a session report, or a
knowledge update cannot grant trust. Store grants in sidecar-owned application
settings separately from harness definitions, using the existing privileged
Electron-main to sidecar boundary. Do not add a session-token trust route.

Approval displays executable, resolved path, argument templates, input mode,
timeout, and the following distinction: this executable receives permitted
workspace context and existing credential access, may run configured hooks or
plugins, and is not sandboxed or certified safe by OrkWorks. Administrator
policy remains authoritative. This is explicit executable trust, not a claim
that optional automation has been disabled by a declarative field.

Bind approval to a canonical digest of the inference definition and resolved
executable path. Changing the definition or resolution invalidates approval;
revocation immediately makes the adapter unavailable. Updates to the executable
at that same path remain within the user's trust in the installed tool, as do
its dependencies and configuration; disclose that scope. Do not claim a digest
of JSON detects changes to executable contents or arbitrary dependencies.

Canonical identity uses SHA-256 over a versioned serialized record containing
harness ID, origin, the fully defaulted validated capability, and canonical
absolute executable path. Resolve bare names using the sidecar's current PATH
without running them; reject relative paths containing directory components,
empty/relative PATH entries, and non-executable targets. Resolve symlinks and
spawn the approved absolute target, never repeat a PATH lookup at spawn.
Equivalent key ordering or omitted default values produces the same digest.

Persist grants in `~/.orkworks/taskmaster/inference-trust.json` as a version-1
document with a monotonic generation and per-harness digest/path grants.
Missing means no grants; malformed/unsupported/unreadable means fail closed,
not reset-and-approve. Use atomic replacement and the Taskmaster persistence
lock. A dedicated privileged trust endpoint accepts approve/revoke operations
and an expected identity/revision; stale requests conflict. General settings
replacement cannot insert grants. Electron exposes narrowly typed inspect,
approve, and revoke IPC, with matching independently declared renderer types.

The trust document is bounded to 1 MiB and 1,024 grants. Reject duplicate keys,
non-regular or symlinked trust files, malformed digests, and stored paths with
control characters or non-normalized components. Stored path validation must
not require the tool to remain installed: revocation still works after removal.
On Windows, direct custom execution resolves native `.exe` targets only; a
script requires an explicit native interpreter/wrapper, not implicit shell
startup. Native Windows verification remains a release/CI requirement.

Revalidate approval before spawning and atomically with accepting results.
Capture the adapter digest, resolved path, and durable trust generation in an
evaluation identity; include it in cache keys as well as the evaluation
snapshot. Definition/trust mutations and final recommendation commit must
share a serialization boundary, extending the existing generation-guarded
acceptance path rather than introducing a check-then-write race. A
revocation/reapproval cycle increments generation even when the digest is
unchanged. Provider/model, workspace, and context invalidation remain in force.

The current runner has deadline-based cleanup but no external cancellation
token. Initial revocation prevents new calls and rejects pending output, but
an already-running executable can continue until exit or timeout and its side
effects cannot be undone. Disclose this limitation; do not claim immediate
process cancellation. Trust is global, while permitted analysis context
remains workspace-scoped.

## Settings and compatibility

Expose separate states for unavailable, invalid definition, approval required,
approved custom adapter, and built-in profile. Show inference-only custom tools
without requiring a Peon capability. Do not remove a stored selection merely
because its adapter is temporarily unavailable. Deterministic recommendations
continue when background inference is unavailable.

Declared capability and runnable capability are distinct. An approval-required
adapter must not report ready, collect context, reserve usage, invoke version
probes, or run model-discovery commands on the Taskmaster path. Gate runnable
status before collection and reservation, then revalidate before spawn.

Custom inference uses static model lists or free-text model entry in v1.
Taskmaster must not call the existing automatic dynamic discovery effect for
custom adapters, even after approval: discovery would be a separate executable
outside the approved inference definition. Existing Peon discovery is unchanged.
Validate model strings consistently at settings and invocation boundaries:
nonempty, at most 256 UTF-8 bytes, no control characters, no trimming or prefix
rewriting. Reject explicit reasoning effort when the adapter declares no
reasoning arguments rather than silently dropping it.

The later shared capability mapping retains three code-owned builtin variants:
Codex and Claude use their existing versioned CLI profiles; Ollama retains its
existing HTTP transport and explicit endpoint selection. JSON does not grant
access to those bindings. Custom command definitions are projected for
Taskmaster independently of Peon and do not become eligible Peon providers.

Protocol validation runs through local fixtures during development. Do not run
an imported executable automatically to discover its capabilities. A future
interactive connection test must disclose that it runs trusted code and can
consume provider usage; it is not required for this first implementation.

## Implementation boundaries and verification

- Extend `harness/definition.rs`, registry projection, and store validation;
  cover custom definitions, overrides, round trips, unknown fields, and legacy
  documents. Preserve existing schema-version handling rather than silently
  interpreting an unsupported version.
- Add a dedicated inference-definition/validation module and custom transport
  module under `providers/`; keep native built-in preparation separate from
  generic argv expansion and response parsing.
- Extend Taskmaster settings persistence and privileged handlers with
  fingerprint-bound grants and revocation. Renderer actions never choose an
  arbitrary sidecar URL or write grant files directly.
- Update the desktop Recommendations settings, API types, documentation, and
  example custom harness JSON. Preserve Electron/renderer boundaries.
- Test no-Peon custom providers, custom model identifiers, stdin/file delivery,
  literal argv metacharacters, inherited fake credential/config sentinels,
  stripped session tokens, temp cleanup, timeout/output bounds, malformed and
  duplicate-key responses, no automatic execution before trust, and no fallback.
- Test definition/path changes and revocation across restarts, pending calls,
  and cache hits; confirm Peon settings and active sessions are unchanged.
- Use local fake executables for process tests. No real credentials, model
  calls, installations, or user CLI configuration changes are required.

## Deliberate limits

JSON configuration supplies extensibility, not a sandbox. Direct HTTP custom
providers and storing API credentials inside OrkWorks are outside this change.
Arbitrary native output parsers and automated wrapper generation are deferred.
Copilot, OpenCode, and Aider do not gain a built-in supported profile merely
because custom trusted adapters become available.

## Review and implementation sequence

Two read-only subagents reviewed correctness and integration coverage. The
patched design makes scheduling eligibility distinct from declaration, defines
trust/cache identity and acceptance serialization, corrects the nested response
example, preserves origin, and states revocation's process-lifetime limitation.
It also specifies schema-2 rejection, schema-3 round trips, static/free-text
custom model entry, and privileged trust persistence. Root verified the cited
schema, projection, cache, and runner paths before adopting these findings.

Start with an inert schema slice: command-kind validation and round trips only,
with no runnable provider projection. Builtin capability migration, trust/API,
custom transport, cache integration, and Settings activation follow before this
feature is considered usable. User JSON rejects builtin kind from the start.
