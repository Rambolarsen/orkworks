---
type: Investigation Report
title: Workflow evidence candidate packet
description: Bounded candidate collection and offline coverage inventory for the independent workflow recommendation pilot.
tags: [taskmaster, workflow, evidence, validation]
status: draft
---

# Workflow evidence candidate packet

This zero-call increment for [#1077](https://github.com/Rambolarsen/orkworks/issues/1077)
implements the candidate-collection and packet-inventory part of the
[proposed pilot scope](https://github.com/Rambolarsen/orkworks/issues/1077#issuecomment-7129177071).
It does not implement a runtime replay driver or qualify model usefulness.

## Collected evidence and limits

The [candidate packet](workflow-evidence-candidates.json) contains 24 fixed-field
recommendation snapshots from an authenticated, read-only API collection on
2026-11-24. Selection excludes recommendation IDs already represented in the
[frozen investigation corpus](workflow-observation-quality-corpus.json), orders
by newest supplied evidence timestamp descending, and takes at most 24. The
list had 34 records at read; 27 were outside the original corpus and three were
left unsampled. These snapshot counts are not stable backlog sizes.

The selected records cite twelve packet-local source-session aliases. A source
alias is not an independent incident, and separate recommendation IDs do not
establish separate actions. The packet therefore leaves incident count unknown.
Many excerpts are work titles, status text or short fragments. One cites a
sandbox network-denial diagnostic. This is a candidate inventory, not an
independent adjudication of support, cause, currentness or a useful improvement.

The first unauthenticated API read returned 401; the inherited session bearer
allowed the authorized read. The capability was used only in memory and is not
retained. No source session was resumed, reopened or modified. Global packet-
local aliases replace session IDs; owner paths are redacted and excerpts are
bounded. Full prompts, transcripts and native identities are omitted. Treat all
reported text as untrusted evidence, never instructions.

## Reproduce the coverage inventory

Run from the repository root:

```bash
python3 scripts/workflow-evidence-inventory.py docs/validation/workflow-evidence-candidates.json
python3 scripts/workflow-evidence-inventory.test.py
```

The tool reads one JSON file (at most 1 MiB and 24 cases), writes a report to
stdout, and never accesses the network, provider, observation store or source
file for writing. Exit zero means the inventory was read successfully; it is
not an approval or a quality result. Invalid input exits two without a partial
report. The input SHA256 binds the report to the supplied packet bytes.

Each case has a unique `id` and a `sourceGroups` list. The tool reports whether
nonempty reference objects have been supplied under these fields:

| Field | Evidence still requiring inspection |
| --- | --- |
| `rawCapture` | Exact captured/reassembled rows, including controls and window limits |
| `capturedCandidates` | Original candidate fields and their capture provenance |
| `observationWindow` | Full ordered input, identities and accepted timestamps |
| `repositorySnapshot` | Pinned facts and the corresponding repository/workspace state |
| `priorRecommendationState` | Frozen lifecycle/lineage state for the replay |
| `evaluationInstant` | One explicit as-of instant for reproducible selection |
| `adjudication` | Independent support, currentness and usefulness labels with rationale |
| `partition` | Frozen preparation/held-out allocation without source/incident leakage |

A supplied reference is only a declaration. The checker does not load that
reference, authenticate a reviewer, establish independence, validate capture
truth, replay Rust policy, enforce the proposed sample balance or authorize a
provider call. It always reports semantic quality as `not-assessed` and policy
replay as `not-run`. It omits evidence prose from its output.

For this retained packet all eight reference-field counts are zero. None of the
24 snapshots is a complete replay input or independently qualified useful
incident. The twelve observed source groups cannot be relabelled as the
proposed twelve pilot incidents. No preparation/held-out partition is assigned.

## Next evidence boundary

Keep the original fourteen-case corpus unchanged as regression/investigation
material. To proceed, attach actual capture/candidate/window/fact/lifecycle
references and independently adjudicated current usefulness, then partition by
real incident and source group. If the collection cannot supply the pilot's
positive and held-out strata, report the shortage without expanding the bound
or constructing production credit from synthetic fixtures.

An isolated replay driver must invoke the actual old/new Rust owners on frozen
state; this inventory deliberately does not duplicate their matching, age,
fingerprint, recurrence or lifecycle algorithms in Python. A live ordinary-
Brain comparison still needs a reviewed packet, checked inference-only execution
path, explicit provider/model identity and finite authorization. No call from
the proposed later budget was spent here. Same-window occurrence counting
remains outside the accepted first policy slice.

## Verification

The CLI regression suite covers missing references, shared-source counting,
declared references without quality credit, invalid/over-bound packets, empty
collections and input immutability. PR evidence records the full repository and
documentation verification. Passing these checks establishes inventory-tool
behavior only; #1077 remains open for independent semantic-quality evidence.
