# Electron + React + TypeScript desktop shell

- Status: superseded by ADR 0078
- Deciders: OrkWorks team
- Date: 2026-06-15

## Context

OrkWorks needs a cross-platform desktop application that can embed terminal sessions, display session metadata, and communicate with a local backend process. The app must feel native on macOS, Linux, and Windows without requiring separate platform-specific codebases.

## Decision

We will build the desktop shell using Electron with a React + TypeScript frontend. The UI will use a VS Code-like three-column layout: left sidebar for workspaces/sessions, center for the embedded terminal (xterm.js), and right sidebar for action overview, capacity, and recommendation panels.

## Consequences

- Single TypeScript codebase ships on all three desktop platforms
- Large ecosystem of Electron tooling and React component libraries
- VS Code-influenced layout is familiar to developers
- Electron's memory footprint is higher than a native app; acceptable for a developer tool
- React + TypeScript provides strong typing and component reuse across panels

## Superseded — 2026-10-08

[ADR 0078](0078-fixed-desktop-shell-and-central-navigation.md) replaces this
ADR's three-column layout decision with compact Sessions, one central surface,
and an optional contextual inspector. Electron, React, and TypeScript remain the
desktop stack; the supersession changes the shell organization only.
