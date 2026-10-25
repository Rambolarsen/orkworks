---
type: Process Guide
title: Taskmaster knowledge publication and recovery
description: Operate signed reference knowledge, paired starter trust, promotion and offline recovery without exposing private publisher material.
tags: [orkworks, taskmaster, knowledge, privacy, operations]
status: stable
---

# Taskmaster knowledge publication and recovery

Taskmaster reference data is independent of application updates. The owner’s
publisher exports only exact reviewed, allowlisted distilled pages, preserves
maturity and stable IDs, signs immutable bundles and manifests, and joins its
existing Pages deployment. Public clients never clone the private source or
receive private receipts, source paths, review hashes or credentials. The
[knowledge specification](../../specs/taskmaster-knowledge.md) defines eligibility;
[ADR 0081](../adr/0081-signed-taskmaster-knowledge-activation-proof.md) defines proof.

## Public artifact identity

The initial production publication was deployed on 2026-10-16. The feed is
[the signed manifest](https://rambolarsen.github.io/brain/orkworks-knowledge/manifest.json).
A generated packaged starter retains its original signed bundle and signed
manifest attestation, verified independently in Electron and Rust.

| Identity | Initial publication |
| --- | --- |
| Version / sequence | `brain-1` / `1` |
| Signed privacy policy | `1` |
| Public SPKI SHA-256 | `a35da5934c629903f8f31db5067a9c5086f8372c34e3d917c9efb7decfd465dd` |
| Bundle-envelope SHA-256 | `9d7812815ea4bdae1da3388f2d8ceb22784c9559998ffb76d4a3ee9f20336a3e` |
| Manifest-envelope SHA-256 | `13dc8acab5408965e2f2cfa6f2fc246c53fb4d23b42f9373c54b43f1c3842d19` |
| Reviewed public-core SHA-256 | `6755ec27c1d62d3a900dfec1d7d5828c84e1b4449d4f5af261c67cbacf26abda` |
| Guidance | Eight distilled pages and a generated index; assessment capability present |

The [immutable bundle](https://rambolarsen.github.io/brain/orkworks-knowledge/bundles/brain-1.json)
and [historical signed manifest](https://rambolarsen.github.io/brain/orkworks-knowledge/manifests/brain-1.json)
remain available after newer publications. A private publisher receipt connects
these public identities to the reviewed source revision; do not copy that receipt
into app resources, public evidence or Pages.

## Complete readiness rubric publication

On 2026-10-25 the owner approved the exact public candidate and signed
publication in [Brain PR #127](https://github.com/Rambolarsen/brain/pull/127).
The [publication workflow](https://github.com/Rambolarsen/brain/actions/runs/40211250600)
completed after that merge.
Current source packages the generated activation proof for this publication.
The pinned public key is unchanged. Older format-1 publications remain eligible
for older clients; readiness requires the complete format-2 descriptor.

| Identity | Readiness publication |
| --- | --- |
| Version / sequence / bundle format | `brain-3` / `3` / `2` |
| Bundle-envelope SHA-256 | `ac2d1750c76163cad6226eb94ee316cae46788623380589025753adca69fa511` |
| Manifest-envelope SHA-256 | `4e028762fae8a8dca9ebd346210dc965295e43ea43ed6c760ca545de9ea48eb3` |
| Reviewed public-core SHA-256 | `1d6cb0f03c3f66b43257e94fef8160f011758ce699bcfc2e856ff8b9a93768b7` |
| Guidance | Ten pages, including the complete fourteen-concept readiness rubric |

The [immutable readiness bundle](https://rambolarsen.github.io/brain/orkworks-knowledge/bundles/brain-3.json)
was verified against the signed manifest with the existing pin. Removing the
archive-owned version, sequence and publication time yields the exact reviewed
public candidate in every field. Offline Electron and Rust tests exercise the
actual generated starter. This publication does not establish model score quality
or availability in a published OrkWorks installer.

## Promotion and publishing ownership

A changed source byte needs renewed review of the complete public artifact and
an updated exact allowlist hash. New pages are excluded by default. Review links,
labels, metadata, provenance and relationships as well as prose; excluded pages,
private assessments, experiments and raw research have no per-page exception.
Do not promote a practice’s maturity merely to include it.

The owner reviews content and key trust separately from source implementation.
The publisher’s private guide governs source promotion, protected archive setup,
key inventory and historical regeneration. Production runs only from current
trusted main through the reviewed serialized Pages workflow. Archive protection
forbids force pushes and deletion with no bypass actors; normal publication
appends one verified transaction. Preserve prior immutable URLs and receipts.
Do not rerun historical workflows predating the current-main freshness gate.

## Signing custody and rotation

The owner-provisioned repository secret is `TASKMASTER_KNOWLEDGE_SIGNING_KEY`.
The replacement private key is retained in encrypted macOS Keychain under service
`OrkWorks Taskmaster knowledge signing`, account
`Rambolarsen/brain:rotation-2026-10-16`. Private bytes are never printed, committed,
attached to issues, or included in application resources. Keychain custody is an
encrypted local copy; the owner maintains a separate encrypted offline backup.

Rotation requires explicit approval of the new public fingerprint and paired
application trust. Retain old public verification keys for historical publisher
archives. Ship the new app public pin and a generated starter signed by that key
with matching manifest attestation in one release. Rust embeds the same resource
key used by Electron. A downloaded key never expands client trust. Older clients
remain pinned to their packaged key; preserve older compatible publications.

## Offline recovery and diagnostics

A fresh source build can use the signed starter without the feed. The updater
checks at startup when due and every six hours while running. A failed, offline,
tampered, incompatible or interrupted update preserves independently verified
working knowledge; there is no separate age expiry. Legacy unsigned data is
ineligible and cannot establish a sequence floor.

Electron stores active/previous proof and advisory update status under its userData
`knowledge/keys/<fingerprint>/` namespace. The sidecar persists proof at
`~/.orkworks/taskmaster/knowledge/<fingerprint>/activation.json`. Neither version
text nor advisory status authorizes inference. Missing, unreadable or corrupt
proof makes Brain analysis unavailable; observation recommendations continue.
Never hand-edit recommendation or proof files to bypass the gate.

If the privacy-unavailable message persists, confirm that the app and sidecar were
rebuilt/restarted with the signed starter and matching key. Older published
installers may still contain the deliberate closed gate. Inspect knowledge-update
errors separately from the latest analysis outcome. A healthy current proof does
not erase prior provider failures, and executable approval does not confer
knowledge eligibility.

Run focused source checks after installing the frozen desktop dependencies with
Node from `.nvmrc` and rebuilding `fs-ext` for host Node:

```bash
rtk proxy node --experimental-strip-types --test apps/desktop/tests/knowledgeStarter.test.ts
rtk proxy cargo test --manifest-path crates/orkworksd/Cargo.toml production_starter
```

Resource tests exercise the actual packaged pair, not a handwritten public copy.
Production route/runtime tests verify offline activation and restart, corruption
revocation, authenticated ready status and native/custom manual/background
invocation. Packaged and live-feed acceptance is recorded separately; synthetic
fixture success alone does not complete issue #529.
