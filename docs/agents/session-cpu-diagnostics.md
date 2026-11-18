---
type: Troubleshooting Guide
title: Session CPU diagnostics
description: Measure a selected macOS daemon without changing session state and distinguish inference from projection cost.
tags: [performance, sessions, diagnostics, macos]
status: stable
---

# Session CPU diagnostics

Use this when OrkWorks stays busy after most coding sessions stop producing
output. A working-session count alone does not predict CPU: session-list
refreshes can inspect Git and branch documents for other remembered sessions.

## Select and measure

Identify the exact daemon in Activity Monitor or inspect executable names:

```bash
rtk proxy ps -axo pid,ppid,comm
rtk proxy python3 scripts/session-cpu-baseline.py <DAEMON_PID> --seconds 10
```

Replace `<DAEMON_PID>` with the selected `orkworksd` process ID. Do not select
another daemon based on stale metadata. The helper is macOS-only, accepts
1–60 seconds, requires Python 3, and prints JSON with process CPU seconds,
measured elapsed seconds and CPU percentage where one core is 100%.
It reads only PID, start time, cumulative CPU and executable name; it does not
read arguments, environment, credentials, session metadata or terminal output.
It checks start time and executable before and after measurement and rejects
changed or disappeared processes. Start-time resolution is one second, so this
is a diagnostic identity check, not an authorization primitive.

The interval includes the second `ps` call's overhead. Counters are quantized;
prefer ten-second or longer intervals to very short readings. A process may use
more than 100% across cores. This does not measure GPU utilization, memory
pressure, child processes or per-request latency. Process inspection can require
tool permission outside a sandbox; report denied access rather than substituting
zeros or changing sandbox settings.

Repeat with the same binary, workspace, session population and output workload.
Record the exact source/build when known, debug versus release, open and working
session counts, CPU measurement, and whether other tools were busy. A restart
clears buffers and invalidates a controlled before/after comparison. The helper
does not collect those contextual details automatically.

## Locate the work

For a separate five-second local stack capture:

```bash
rtk proxy sample <DAEMON_PID> 5 -file /private/tmp/orkworksd-cpu.sample.txt
rtk proxy ollama ps
```

Keep stacks local until reviewed: they contain paths and other machine details.
Sleeping threads often dominate sample counts. Inclusive stack counts overlap
and must not be summed into CPU percentages. Repeated `query_sessions` /
`SessionProjection` / Git / branch-document frames point toward list enrichment;
Peon inference requires separate evidence. Ollama's `100% GPU` describes model
placement, not measured utilization; a loaded model can be idle.

Do not stop sessions, unload models, replace binaries or restart OrkWorks merely
to obtain a cleaner measurement. Those changes require an owner-controlled
experiment. Active HTTP list probes also add projection work, so keep passive
CPU sampling separate from request-latency experiments.

## Current investigation

The [2026-11-18 plan](../superpowers/plans/2026-11-18-session-projection-cpu.md)
records the evidence and three delivery boundaries. The baseline helper is
[#1893](https://github.com/Rambolarsen/orkworks/issues/1893); inspection reuse is
[#1894](https://github.com/Rambolarsen/orkworks/issues/1894); separating refresh
cost is design-gated in [#1895](https://github.com/Rambolarsen/orkworks/issues/1895).
These are source diagnostics, not a claim that published installers contain a
CPU fix. Existing [Peon timeout diagnostics](peon-timeout-troubleshooting.md)
remain the route for provider timeouts.

Run the helper regressions with:

```bash
rtk proxy python3 -B scripts/session-cpu-baseline_test.py
```
