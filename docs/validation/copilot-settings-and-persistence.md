---
type: Validation Evidence
title: Copilot settings migration and persistence boundaries
description: Version-bound model-free settings migration evidence and the separate telemetry and session-storage gates.
tags: [copilot, validation, role-profiles, settings]
---

# Copilot settings migration and persistence boundaries

Date: 2026-11-01. Tracks [#740](https://github.com/Rambolarsen/orkworks/issues/740);
consumers #741–#743. Continues the [November hosted turn](copilot-november-transport-probe.md).

## Decision

**The pinned 1.0.90 CLI migrates the tested legacy setting into settings.json,
and the legacy value wins in the tested initial conflicts. After migration,
a settings.json edit takes effect.** This qualifies a narrow discovery-setting
route, not telemetry precedence, skill delivery or a permission profile.
No new model call, CLI server, native-addon import or production change ran.
All six role profiles remain unverified/no-go.

The [retained packet](fixtures/copilot-1.0.90-settings/manifest.json) includes a
reproducible model-free inspector, exact synthetic settings before and after
each command, normalized discovery output and pinned source selections. The
native, app and native-addon hashes matched; the existing SEA inspector also
reconfirmed that all six selected cache files match the embedded package.
The three-file inspector pin is not a complete startup dependency attestation.

## Version-specific discovery experiment

Each case uses a fresh disposable home and one synthetic Git repository with
one inert skill. It runs the pinned native executable with
`--no-auto-update skill list --json`, using a 30-second timeout and no retry.
Only PATH, HOME and TMPDIR may be inherited; all other variables start absent,
then the packet's explicit discovery overrides are applied. Hooks, memory,
IDE auto-connect, remote export and remote sessions are requested off in the
synthetic settings. These controls are setup inputs, not independently tested
isolation or network-enforcement claims. No real user settings are read by the
observer or changed.

The controlled setting is `disabledSkills`, containing the exact sentinel name
or an empty array. Here “enabled” means the CLI discovery result's `enabled`
field. It does not mean the skill body reached a model or was invoked.

| Case | Initial legacy config value | Initial settings value | Discovery enabled | Settings after command |
| --- | --- | --- | --- | --- |
| Baseline | No config file | Key absent | true | Key absent |
| Legacy only | Sentinel disabled | Key absent | false | Sentinel disabled |
| Settings only | No config file | Sentinel disabled | false | Sentinel disabled |
| Conflict A | Sentinel disabled | Empty disabled list | false | Sentinel disabled |
| Conflict B | Empty disabled list | Sentinel disabled | true | Empty disabled list |
| Later settings edit | Already migrated managed state from Conflict A | Empty disabled list | true | Empty disabled list |

All six commands exited 0 with empty stderr and exactly the sentinel plus two
expected built-in skill names. Initial legacy cases rewrote `config.json` to
managed state and moved the tested key to `settings.json`. The final command
edits only the post-migration settings file. Exact file bytes and outcomes are
retained per case; this is direct file/discovery evidence, not an inference
from a model echo. Additional uncontrolled discovery sources stop the inspector.

The current [vendor configuration reference](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-config-dir-reference)
identifies `settings.json` as the editable file and says user settings left in
`config.json` are ignored. That is current, unpinned guidance. It does not
describe this 1.0.90 first-load migration fixture; do not apply it retroactively.
This result covers the tested key and layout only, not every legacy key,
managed policy, repository setting, invalid JSON or migration-recovery case.

The earlier hosted probe seeded the same values in both files. Its rewrite
therefore cannot distinguish which file supplied telemetry controls. Future
settings-only capture tests should start without legacy user settings in
`config.json`, retain before/after bytes and bind actual model-facing receipts.
The existing observation remains valid within its recorded limits.

## Telemetry controls and session storage are separate

Version-pinned `app.js` selections show the capture-class overlay supplied to
an OTel host, a separate `enableSessionStore` tracking branch, and a session
filesystem selection that uses `eventsLogDirectory` or a default filesystem.
These are selected call sites, not complete native implementations. In
particular, `enableSessionStore: false` in an internal branch does not establish
a supported CLI switch or disable event-file persistence. No internal API or
custom session filesystem was instantiated.

Current [managed telemetry documentation](https://docs.github.com/en/copilot/reference/enterprise-administrators/enterprise-managed-settings#telemetry)
describes independent capture classes and managed overrides. Explicit class
values override the legacy broad capture shorthand; managed telemetry can
override local and environment controls. Its identity class leaves the
pseudonymous identifier intact. These are current vendor contracts, not tested
1.0.90 effective-policy or per-request receipts.

The current [session-data reference](https://docs.github.com/en/copilot/concepts/security-governance-and-network-settings/session-data)
separately describes session files, a SQLite history index and optional account
sync. This explains why disabling telemetry response capture cannot by itself
prove that responses are absent from native history. In the prior hosted turn,
assistant content and nonempty reasoning-related fields were observed in its
session event file; this follow-up neither reopens those deleted raw files nor
upgrades the field-presence observation to plaintext reasoning capture.

Three gates must remain distinct: OTel content selection, local native session
persistence, and remote session export. Deleting a disposable directory after a
probe proves none of their pre-storage properties. Remote-export controls also
do not certify local-storage exclusion. We have not identified a qualified
supported 1.0.90 CLI route that excludes forbidden content across every required
path. This is a scoped measurement no-go, not an impossibility claim. No broad
capture, provider substitution, SDK observer or new broker is proposed.

## Consumer handoff and next bounded decision

#741 receives the tested migration hazard: input settings bytes can change
before discovery, so a prelaunch file hash alone cannot stand in for effective
configuration. The result is not a verified permission/configuration snapshot.
#743 keeps discovered, disabled, delivered and invoked separate; all six
commands concern discovery only. #742 receives no resume or same-parent
continuation evidence. #740 remains open.

Before a new hosted skill or denial probe, review an exact settings-only
configuration and a justified storage boundary against the existing
[probe plan](../superpowers/plans/2026-10-04-copilot-role-capability-probes.md).
A telemetry-only test cannot silently waive the plan's wider persistence gate.
If the intended boundary is only the evidence exporter, that requires an explicit
methodology decision; it is not established by a successful hosted response.
No new live probe is authorized by this report.

## Reproduction and validation

From this task's worktree, choose a fresh output directory and run:

```bash
rtk proxy python3 docs/validation/fixtures/copilot-1.0.90-settings/inspect-settings.py \
  --native /opt/homebrew/lib/node_modules/@github/copilot/node_modules/@github/copilot-darwin-arm64/copilot \
  --cache /Users/froomiebot/Library/Caches/copilot/pkg/darwin-arm64/1.0.90 \
  --output /tmp/ork740-settings-fresh
```

The inspector pins bytes before execution, never imports runtime source,
refuses an existing output directory, bounds each discovery command/output,
and removes its own synthetic homes. Discovery paths are normalized explicitly;
raw stdout hashes bind the observer's unretained path-bearing outputs and will
vary across disposable roots. Exact inherited host-environment values are not
recorded. The normalized cases and selected source bytes can be reproduced;
their scope does not establish full effective-policy or storage coverage.

Verify retained manifest hashes/sizes and staged Git bytes, including a
`core.autocrlf=true` simulated checkout. Run documentation metadata/index checks,
the docs build, doc drift and the worktree close-out hook before handoff.
