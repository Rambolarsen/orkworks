# Recorded terminal-size cue

## Purpose

Make it clear why a dead session can have historical hard wraps without
changing its terminal output or recorded geometry.

## Design

`HistoricalTerminal` keeps using the recorded `cols` and `rows` whenever the
terminal-output endpoint supplies both values. Above the replay, it renders a
small informational cue: `Recorded at {cols} × {rows}`.

The cue is absent for legacy sessions that have no recorded size. It has no
controls and does not persist UI state.

## Boundaries

The renderer does not reflow, edit, or normalize replay records. The sidecar
API and terminal persistence format remain unchanged.

## Validation

A focused component/source test verifies that sized historical replays render
the cue and legacy replays do not claim a recorded size.
