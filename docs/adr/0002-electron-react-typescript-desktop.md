---
type: decision
status: accepted
title: "Electron + React + TypeScript desktop shell"
---

# Electron + React + TypeScript desktop shell

- Status: accepted
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
