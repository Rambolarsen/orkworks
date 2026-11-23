---
type: Validation
title: Offline ordinary-review configuration leaves
description: Complete logical leaf byte commitments and conditional resource/owner/prepared joins with explicit actual-source and native limits.
tags: [orkworks, review, configuration, qualification, validation]
status: stable
---

# Offline ordinary-review configuration leaves

Owner acceptance on 2026-11-20 with “keep going” covers the
[leaf contract](../superpowers/specs/2026-11-20-review-configuration-leaves-design.md)
and [offline plan](../superpowers/plans/2026-11-20-review-configuration-leaf-components.md).
This report is finite byte/conditional-join evidence for
[#68](https://github.com/Rambolarsen/orkworks/issues/68),
[#66](https://github.com/Rambolarsen/orkworks/issues/66) and
[#1899](https://github.com/Rambolarsen/orkworks/issues/1899), not actual capture,
resolver correctness, native eligibility or Start.

## Reproduction and independent bytes

Under the pinned Node22:

```sh
node --test docs/validation/ordinary-review-configuration-leaf-components.test.mjs
```

The [module](ordinary-review-configuration-leaf-components.mjs),
[tests](ordinary-review-configuration-leaf-components.test.mjs) and
[vectors](ordinary-review-configuration-leaf-vectors.json) are isolated in
docs/validation. Complete literal JSON bytes, lengths and domain-separated SHA-256
were computed independently with Python JSON/hashlib before the encoder existed.
The vector embeds the public 10,537-byte builtin document from audit base
b4dfd63b, not user configuration. Resource bytes, inventory and qualification
facts are synthetic. Actual Rust parsing/resolution of the copied catalog is
not established by encoding or hashing it here.

The current focused suite passes **180 tests**. All **36/36** deliberate
temporary-copy mutations were detected by existing assertions; the copies were
removed and repository code was never patched by that audit. Mutations cover
canonical/base64 equality, complete source/resource payload hashing, independent
inventory and required/absence coverage, owner/epoch/current snapshots, command/
workspace/digest links, source predicates, raw/encoded bounds and the parent join.
Fresh whole-task correctness review found no required findings. Its optional
buffer-reuse improvement was applied and all focused cases passed again. Initial execution failed with
no implementation; an added valid empty-inventory/none-adapter case failed until
the owner-map validator stopped requiring a phantom empty-string key. Expected
inventory lives separately from the leaf's own supplied inventory. Earlier
#1919/#1922 codec/test/vector bytes remain unchanged; the join delegates to the
existing #1922 prepared validator rather than replacing its checks.

## Logical cases and bounds

Closed shapes/domains, canonical UTF-8 and base64, duplicate/escaped/reordered
keys, sparse arrays, presence/state combinations, required absence, unknown or
symlink/unreadable-like values, literal runtime policy, stock absence and exhausted
owner epochs are checked. Active-selection revision zero is valid, distinct from
the positive owner epoch. Absent and regular empty bytes have different commitments.
Raw bytes include whitespace and binary values without normalization.

Raw 64 KiB builtin and 256 KiB resource boundary containers pass; one more decoded
byte denies. Standalone closed wrappers at exactly **128 KiB catalog** and
**512 KiB integration** pass and one more encoded byte denies. Their exact sum
is **640 KiB**; no independent combined-overflow witness with both leaves valid
exists. Boundary constructors count complete standard JSON bytes independently.

These wrapper maxima are not qualified stock resolutions or maximum usable
#1922 preparations: their stricter 12 KiB configuration and 48 KiB prepared limits
can deny a wrapper combination, and real builtin argv/templates are not inferred
from padded test values. No complete preview/cache/native/physical-memory or I/O
budget follows. Accepted byte limits are not measured peak validation-allocation
or CPU envelopes. Integration encoding requires an explicit validated catalog
platform parameter; path checks establish lexical absolute form only.

The conditional join checks independent expected inventory, current/captured
leaf bytes, exact synthetic configuration/resource owner references, one epoch,
selected tool/model, current workspace/active selection, both digest links and
final command to the copied prepared value. It also retains the parent original/
edited draft, source, expiry and owner checks. Replaced labels/owners, changed
assets/fragments, mixed epochs, unknown qualification, late argv or prompt authority,
wrong digest/program/argv/cwd and altered instruction deny the old join.

## Test-only facts and unqualified gates

Test facts contain separate expected inventory, captured leaves/prepared value,
configuration/resource owner tokens and parent prepared entry; current facts
contain current leaves/tokens, parent current facts and explicit simulation
predicates. stockToolIds is an independent literal test membership set; resolver,
inventory, fragment and literal-command qualification flags are assumptions.
Missing/truthy values never become positive proof. None of these records is a
public API or an authenticated production capability.

Actual stock-catalog/build/absence custody, mutation/failed-publication epoch
coverage, resolver output, adapter inventory/projector completeness, code-owned
fragment/template matching, asset/executable ownership and native/model/profile
eligibility remain unqualified. No actual user config, filesystem lease, hook
reconcile/install/revoke, provider or native coding-tool session is used. The
supported production-role set remains empty.

Complete preview entry/view/response codecs and separate 64 KiB witnesses,
protected admission initialization/recovery/retention/export/downgrade, source
fences and private prepared Start/lifecycle qualification remain open. No app
imports this module and no production authority or settings behavior changes.

## Repository regression checks

`bash scripts/verify-repo.sh` passed under Node22 after implementation: 3,615
Rust tests plus integrations (24 ignored), reporter32, native ABI, desktop
1,791 tests (5 skipped) and type-check/build, docs 2,490/site build and close-out
checks. Existing compiler/chunk warnings are outside this diff; README drift
needs no shipped-behavior edit for isolated logical fixtures. Generated package-
manager metadata was restored and foreign worktrees preserved. These checks do
not widen the source/native/physical qualification boundary above.
