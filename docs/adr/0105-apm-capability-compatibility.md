---
type: Architecture Decision Record
title: APM compatibility follows capabilities
description: Admit reviewed APM operations by CLI capabilities with automated behavioral checks instead of a version allowlist.
tags: [apm, skills, desktop, compatibility]
status: stable
---

# APM compatibility follows capabilities

- Status: accepted
- Deciders: repository owner, approved 2026-11-02
- Date: 2026-11-02

## Context

The exact APM 0.33.0 gate blocks upgrades even when the reviewed operation still
works. Owner approved capability discovery plus automated real installation
fixtures in [#1300](https://github.com/Rambolarsen/orkworks/issues/1300).
This supplements [ADR 0097](0097-desktop-skill-management.md); Electron remains
the operation owner.

## Decision

Share a bounded capability probe between status and preview. Version is display
information, not admission. Require install flags --target, --only, --dry-run,
--no-trust-bin and --global; report missing options or probe errors explicitly.
Retain Codex-only local standalone sources, existing manifest/policy validation,
private previews, input drift checks, native confirmation and Unix-only installs.

Run disposable real-CLI workspace and personal fixtures against latest stable
APM in scheduled/manual CI and relevant pull requests. Runtime probing executes
help/version only. It never installs test skills in user directories.

## Consequences

APM upgrades do not require allowlist edits. Capability declarations do not prove
semantics or authenticity; CI supplies behavioral evidence for the tested release,
not every future release. CI failures require investigation and do not remotely
change installed app policy. Existing partial-install and filesystem-race limits
remain. Reverting needs no migration. See the [plan](../superpowers/plans/2026-11-02-apm-capabilities.md).
