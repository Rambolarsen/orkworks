# Copilot 1.0.90 capture-control inspection packet

Static inspection on 2026-10-05 for [#740](https://github.com/Rambolarsen/orkworks/issues/740).
No model, server, addon import, runtime observer or content capture was started.

The [native identity packet](../copilot-1.0.90-native/runtime-identity.json)
binds these three input files to the package embedded in the installed native
executable and to identical selected cache files. The native package matched
the official npm 1.0.90 tarball in that earlier investigation. This packet's
inspector checks their exact SHA-256 values again before parsing or writing.
It does not attest all runtime dependencies or effective startup settings.

## Artifacts

- `capture-settings.json`: seven telemetry entries selected from a 182-entry
  JSON setting-description catalogue embedded at byte 69298825 of `runtime.node`;
  the telemetry canonical-key subobject begins at byte 77099388. These are
  compiled data, not executed getters or control-flow proof.
- `model-call-data.schema.json`: the three model-call payload definitions and
  their transitive local references from the embedded session-event schema.
  It is a payload union, not the session-event envelope or a live example.
- `api-observations.json`: the full list of `SessionOpenOptions` property names,
  four selected properties, `session.eventLog.read` method declaration and
  `UserSettingsGetResult` definition. Explicitly a projection, not a schema
  with a complete reference closure. These declarations do not establish a
  before-storage filter or effective capture-setting receipt.
- `config-help.txt`: exact stdout from the installed update-disabled help
  command. No user setting values were printed. It does not document the
  granular telemetry controls found in the compiled catalogue.
- `inspect-capture.py`: reproduction utility; reads files only, checks pinned
  identities with exceptions even under Python optimization, and emits the
  three static JSON artifacts. Never loads the native addon or executes CLI.

## Reproduce static selection

Use the existing 1.0.90 cache whose selected-file equality was established by
`../copilot-1.0.90-native/inspect-runtime.py`. Do not install or update Copilot
for this reproduction. Replace the placeholders with observer-owned paths:

```bash
rtk proxy python3 docs/validation/fixtures/copilot-1.0.90-capture/inspect-capture.py \
  --runtime '<existing-1.0.90-cache>/prebuilds/darwin-arm64/runtime.node' \
  --events '<existing-1.0.90-cache>/schemas/session-events.schema.json' \
  --api '<existing-1.0.90-cache>/schemas/api.schema.json' \
  --output '<observer-output-directory>'
```

Compare all three generated files byte-for-byte to the retained artifacts.
Different inputs must fail before creating output. The inspection was repeated
with normal Python, `-O`, and `PYTHONOPTIMIZE=1`; each produced identical valid
artifacts and rejected a changed runtime file without creating its output.

The separate help capture used an empty temporary settings directory and cwd:

```bash
rtk proxy env COPILOT_HOME='<temporary-settings-directory>' \
  copilot --no-auto-update help config
```

The manifest records portable equivalents of observed command arguments and
cwd, not raw argv. The Python inspector's retained copy is byte-identical to
the disposable script used initially. CLI stderr is not retained or used as
behavioral evidence. Neither temporary settings nor help output proves startup
isolation or profile enforcement. The manifest hashes every retained artifact
other than itself. `* -text` preserves their exact bytes in Git checkouts.
