---
type: Validation Evidence
title: Root preparation source-codec component evidence
description: Independent synthetic canonical bytes and structural tests for accepted root source records without native, custody or transition qualification.
tags: [orkworks, hierarchy, preparation, codecs, validation]
status: draft
---

# Root preparation source-codec component evidence

Tracking: [#741](https://github.com/Rambolarsen/orkworks/issues/741),
[#742](https://github.com/Rambolarsen/orkworks/issues/742); PR #1093.
Source: [accepted component slice](../superpowers/specs/2026-10-26-root-preparation-source-codecs-design.md#owner-acceptance-and-component-handoff--2026-10-27).

## What these artifacts establish

[root-preparation-source-vectors.json](root-preparation-source-vectors.json)
retains eleven synthetic component vectors: inline Unicode and unbound reference
root assignments, local/nonlocal and Windows-native provenance, direct/wrapped
executable identities, initial input/state and reserved/attached parent shapes.
Each has exact canonical JSON, UTF-8 byte length and a raw SHA-256 fingerprint.
The initial state also binds the existing domain-separated preparation-input
digest; raw fixture fingerprints introduce no new source digest domain.

`generate-root-preparation-source-vectors.py`
uses Python standard-library JSON/SHA-256 independently of the JavaScript checker.
Run from the repository root, regenerate to a temporary file and compare exact
catalog bytes. No model, native tool, credential or live session is involved:

```bash
python3 docs/validation/generate-root-preparation-source-vectors.py --output /tmp/root-preparation-source-vectors.json
cmp docs/validation/root-preparation-source-vectors.json /tmp/root-preparation-source-vectors.json
node --test docs/validation/root-preparation-source-components.test.mjs
pnpm --dir docs docs:check
```

[root-preparation-source-components.mjs](root-preparation-source-components.mjs)
checks closed structural variants, finite bounds, exact canonical stored bytes
and repeated identities against one complete validated run component. It rejects
future child fields/dependencies in roots, content/hash drift, duplicate IDs and
paths, invalid native encoding, wrong variant fields, initial runtime/plan data,
and mismatched initial-input/state/parent bindings. A focused cross-owner check
compares the complete canonical executable identities retained by adapter and
capability evidence, rejecting content, path and invocation-order drift. Counts/path/descriptor limits
have boundary tests. The checker supports arrays, booleans and null for these
new records; the older run/approval checker and its catalog remain unchanged.

The first test run against a minimal no-validation stub failed for missing
fingerprints and missing rejection behavior; after implementation the new suite
passes. The existing 43 run/approval tests also remain in docs:check.

## Qualification limits

Semantic conflicts between explanatory expected-output text and the structured
output contract remain an untested approval obligation; this checker receives no
complete bootstrap/output contract. Rule-relative versus native-provenance source
resolution is also untested. Neither may be inferred from component validity.

A successful check is **component structural validity only**, never an eligible
bootstrap or permission to execute. The catalog labels its bindings synthetic
and lists unverified obligations. Repeated digests do not establish referenced
repository/worktree/source objects, native path canonicality, package inventory
completeness, effective settings/enforcement or retained reference bytes/custody.
A wrapped inventory's tags and file digests can be structurally valid while
omitting a required interpreter/package file; only qualified adapter evidence
can establish completeness. The checker does not fetch or inspect those paths.

The attached vector is a record shape with matching synthetic identities, not
session-producer proof. No source-state transition, actual attachment, uniqueness
across stored runs, startup ordering, durable admission/recovery, authority cutoff,
shared accounting or complete command effect is exercised. The initial-state
checker deliberately rejects a fabricated post-attachment state; later lifecycle
values require their own closed codecs and validated effects.

Next, close the selected nested permission/evidence and source-custody records,
then join complete draft/request/receipt/retained rows and full atomic effects.
#740 remains deferred/no-go until November; this packet cannot promote any
profile or old synthetic bootstrap binding. Keep #741/#742 and broader component
issues open. Native qualification and separately approved executable plans remain
independent prerequisites.
