# Domain glossary

Terms used in architecture reviews and design discussions that aren't
self-evident from code alone. See `docs/adr/` for the decisions that
introduced them.

## External status report

A session status observation reported to `orkworksd` from outside the
sidecar's own process/terminal observation — a harness hook (`report_attention`)
or a manual debug injection (`apply_debug_attention`). Carries an explicit
`source` and `confidence`. Contrast with **process-observed transition**.

## Process-observed transition

A session status change the sidecar decides for itself by observing its own
runtime state, rather than being told by an external report — committed
terminal input implying `working`, or the peon idle-timer sweep detecting
silence past its timeout. Always `source: "process"`, `confidence: 1.0`.
Contrast with **external status report**.

See ADR 0027 (`docs/adr/0027-observed-status-attention-owning-module.md`),
which gives each of these its own entry point in
`crates/orkworksd/src/runtime/observed_status.rs`.
