# Move metadata store from workspace directory to global config directory

- Status: accepted
- Deciders: OrkWorks team
- Date: 2026-06-24

## Context

ADR 0004 placed the `.orkworks/` metadata directory inside each workspace (`<workspace>/.orkworks/sessions/`, `.orkworks/events/`, `.orkworks/capacity/`, `.orkworks/workspace.json`). The rationale was that agents should be able to write metadata directly to the filesystem without an API, and the backend could watch files for changes.

In practice, all writes go through `MetadataStore` inside the Rust sidecar — no external agent or harness writes directly to `.orkworks/`. The directory is internal implementation detail, not an external protocol surface.

Placing it inside the workspace causes real friction:
- Visible in the user's file tree alongside project files
- Requires a `.gitignore` entry in every workspace
- Risk of accidental commit or CI interference
- Feels intrusive in a tool that should observe without leaving traces

The `~/.orkworks/` directory already exists globally for `harnesses.json`. The pattern is established and uncontroversial.

## Decision

Move the per-workspace metadata store from `<workspace>/.orkworks/` to `~/.orkworks/workspaces/<path-hash>/`, where `<path-hash>` is a stable hash of the workspace's absolute path (e.g., SHA-256 truncated to 16 hex chars).

- `~/.orkworks/workspaces/<hash>/sessions/<id>.json`
- `~/.orkworks/workspaces/<hash>/events/<id>.ndjson`
- `~/.orkworks/workspaces/<hash>/capacity/<id>.json`
- `~/.orkworks/workspaces/<hash>/workspace.json`
- `~/.orkworks/workspaces/<hash>/recommendations/<id>.json`

The `MetadataStore::new(root)` constructor already accepts an arbitrary root path — the only change is where the caller computes that root. The `set_workspace` handler will derive the hash from the workspace path and pass `~/.orkworks/workspaces/<hash>/` instead of `<workspace>/.orkworks/`.

File watching (currently watching `<workspace>/.orkworks/sessions/`) moves to the global sessions directory under the hash-based path.

On migration, if an old `<workspace>/.orkworks/workspace.json` exists and the new global path has no data, the sidecar copies existing session/event/capacity files into the global store. After a grace period (one release cycle), the migration code is removed.

## Consequences

- Workspaces stay clean — no `.orkworks/` directory left behind
- No `.gitignore` burden on users
- `~/.orkworks/` becomes the single source of truth for all OrkWorks data
- Existing `~/.orkworks/harnesses.json` fits naturally alongside workspace-scoped data
- Hash-based subdirectories prevent collision without encoding raw paths in filenames
- One-time migration copies existing data; no user action required
- File watcher watches a single global tree, simplifying watcher lifecycle
- ADR 0004 is superseded
