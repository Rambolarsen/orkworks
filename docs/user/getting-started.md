# Getting started

OrkWorks is local-first mission control for AI coding sessions. It observes
your coding-tool sessions (Claude Code, Codex, OpenCode, Gemini CLI, Aider)
and recommends what should happen next — it does not replace those tools.

## Install

Download the latest alpha for your platform from
[GitHub Releases](https://github.com/Rambolarsen/orkworks/releases).

Or run from source:

```bash
git clone https://github.com/Rambolarsen/orkworks.git
cd orkworks/apps/desktop
corepack enable
pnpm install
pnpm dev
```

`pnpm dev` starts the desktop app and automatically launches the Rust
sidecar (`orkworksd`) that manages sessions and metadata.

## Your first session

1. Open OrkWorks and add a workspace (a Git repository you work in).
2. Create a new session and pick a coding tool.
3. Work in the embedded terminal as you normally would — OrkWorks observes
   the session and surfaces its state (attention needed, capacity, last
   activity) in the sessions list.
4. Switch between sessions from the sessions list. One session is active at
   a time by design: switching sessions is the context switch.

## Where your data lives

All metadata is local, under `~/.orkworks/`. Nothing is sent anywhere.

## Learn more

- [OrkWorks MVP spec](/specs/orkworks-mvp) — full product scope
- [Architecture decision records](/docs/adr/README) — why it's built this way
