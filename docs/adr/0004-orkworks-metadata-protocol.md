---
type: decision
status: superseded
title: "`.orkworks/` metadata protocol directory structure"
superseded_by: "0018"
---

# `.orkworks/` metadata protocol directory structure

- Status: superseded by [0018](./0018-global-metadata-store.md)
- Deciders: OrkWorks team
- Date: 2026-06-15

## Context

OrkWorks needs a way to persist session metadata, events, and capacity state. This data must be accessible to agents, Peon, and the OrkWorks backend. It should live alongside the user's code in the repository workspace without interfering with existing project files.

## Decision

We will use a `.orkworks/` directory inside each workspace/repo containing:
- `.orkworks/sessions/<id>.json` — agent-written session state
- `.orkworks/events/<id>.ndjson` — append-only event log per session
- `.orkworks/capacity/<id>.json` — capacity per model/harness

Files are plain JSON/NDJSON, human-readable, and easily watched for changes by the backend.

## Consequences

- Agents can write status directly without an API; OrkWorks watches files
- NDJSON event logs are append-only and naturally streamable
- No external database dependency — works fully local and offline
- `.orkworks/` should be gitignored by default (session data is transient)
- Protocol is extensible: new file types can be added under `.orkworks/`
