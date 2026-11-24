---
type: Validation Guide
title: Offline workflow evidence boundary replay
description: Explicit-input, zero-provider replay of current grounding, exact-family derivation and ordinary observation selection, with synthetic controls and qualification limits.
tags: [taskmaster, peon, workflow, validation, replay]
status: draft
---

# Offline workflow evidence boundary replay

This test-only driver advances the offline tooling for
[#1077](https://github.com/Rambolarsen/orkworks/issues/1077). It invokes the actual
current Rust owners on supplied fixtures; it does not recreate their policy in
Python or infer missing historical context. No production API or runtime setting
is added. The [candidate inventory](workflow-evidence-packet.md) remains a
collection of incomplete snapshots, not a replay-ready empirical corpus.

## Scope and owners

| Boundary | Actual owner called | What the result means |
| --- | --- | --- |
| Captured excerpt grounding | `peon::evidence_is_grounded` | The excerpt passes/fails the current raw-plus-visible check, or capture is unavailable |
| Exact-family derivation | `taskmaster::evaluate_workflow_improvements` | Proposed records/updates derived from supplied retained rows and existing recommendation state; nothing is persisted |
| Ordinary observation selection | `taskmaster::context::recent_workflow_observations` | Current age/text/latest-sixteen selection, not a model output |

These are separate boundaries. A new captured candidate's grounding verdict
never filters supplied retained observations retroactively. Existing-ID updates
and previously accepted reports retain the accepted policy's rules. The driver
does not reconstruct storage, normalize fingerprints, apply replay/rate limits,
collect repository facts, build Brain prompts or exercise in-flight application
fences. It is not capture-to-publication integration or a baseline comparison. Priority
and confidence are the existing derived fields, not measured semantic precision.

## Run the synthetic controls

From an owned repository checkout, record the code SHA and any local diff next
to the result. Use explicit input and a new private output path:

```bash
git rev-parse HEAD
workflow_replay_dir="$(mktemp -d)"
ORKWORKS_WORKFLOW_REPLAY_INPUT="$PWD/crates/orkworksd/tests/fixtures/workflow-evidence-replay/synthetic.json" \
ORKWORKS_WORKFLOW_REPLAY_OUTPUT="$workflow_replay_dir/report.json" \
cargo test --manifest-path crates/orkworksd/Cargo.toml --bin orkworksd \
  taskmaster::workflow_evidence_replay_tests::replay_workflow_evidence_packet_from_files \
  -- --exact --ignored
cat "$workflow_replay_dir/report.json"
```

The entrypoint is ignored by default and the exact filter above runs only that
offline test. It does not construct `AppState`, start an application, call a
provider or access session APIs. The only explicit I/O is the supplied fixture
and new report path. The whole packet is checked before publishing the report
from a temporary file with no-overwrite semantics. Existing outputs and input
bytes are preserved; errors do not trigger retries or replacement filenames.

The normal Rust suite also exercises the driver directly with the committed
synthetic controls. They cover hidden/title payloads, visible and printed quoted
errors, raw-missing colored spans, missing capture, nanosecond/offset age
boundaries, weak-confidence versus ordinary input, immutable retained history,
existing-ID grandfathering, invalid envelopes and report publication behavior.
All controls are constructed scenarios, not observed production successes.

## Fixture envelope

The version-one test envelope has `schemaVersion: 1` and a `cases` array. It is
bounded to 8 MiB and 24 cases; per-case stored observation and recommendation
arrays are each bounded to 10,000 records. These are driver limits, not changes
to runtime retention or admission. Unknown envelope/case/candidate fields and
invalid typed data are rejected.

Each case supplies a unique `id`, `declaredOrigin` (`synthetic` or `observed`),
`workspaceId`, and one RFC3339 `asOf`. Optional boundary inputs are:

- `capturedCandidates`: unique candidate IDs, `evidence`, and optional
  `reassembledRows`. These rows must be the exact prepared view for the intended
  grounding check, not a guessed current tail. Missing/null rows produce
  `unavailable`; an explicitly empty captured array is tested as empty text.
- `storedObservations`: full normalized `WorkflowObservation` records in
  strictly increasing, nonzero sequence order with unique IDs. The actual
  production record decoder is used. Fingerprints and timestamps are supplied,
  not synthesized or repaired. Production deserialization is reused, not full
  store validation; the caller must supply a normalized, internally consistent
  frozen snapshot.
- `existingRecommendations`: full recommendation records for the declared
  workspace, using the production decoder. An explicit empty array represents
  a supplied empty prior state; omission does not.

Omitted stored observations produce `null` ordinary selection and exact-family
results. Supplied observations allow ordinary selection, but exact-family
results stay `null` until prior recommendation state is also supplied. Explicit
empty snapshots can produce empty arrays. Missing evidence must not become a
false negative outcome.

The report contains IDs, decision summaries and an input-byte SHA256; description,
evidence and captured-row text are omitted. IDs must be sanitized by the packet
producer. The digest does not prove provenance or identify the compiled binary. Redaction
that changes replay-relevant bytes or identity joins creates a transformed
fixture; it cannot silently retain original-capture qualification.
`declaredOrigin` is copied as a declaration: even `observed` does not authenticate
capture, independent labels, current usefulness or incident identity.

## Qualification remains open

Every report says `inputProvenance: unverified`, `semanticQuality: not-assessed`
and `baselineCompared: false`. A grounded quoted-error fixture deliberately
passes: passing substring policy is not semantic support. No independent
usefulness labels or useful-output rate is generated.

The current session capability cannot read terminal history under the accepted
listener contract; this driver does not widen that authority or discover private
local source files. Historical replay still needs owner-supplied exact capture,
window/context and frozen state, plus independent adjudication. A later paired
baseline/candidate driver must use the actual pinned owners for each arm; no
old-policy imitation or synthetic control may be recredited as empirical quality.
Live provider comparison still requires separate finite authorization.
