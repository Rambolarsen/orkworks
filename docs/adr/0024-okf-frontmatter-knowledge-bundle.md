# Adopt OKF-style frontmatter and a bundle manifest over specs/ADRs/skills

- Status: rejected
- Deciders: user
- Date: 2026-07-15

## Context

OrkWorks juggles multiple coding-tool integrations (Claude Code, Codex, OpenCode, Gemini CLI, Aider), each of which loads project context through its own convention — `CLAUDE.md`, `.agents/skills`, `opencode.json`, and so on. The actual knowledge these conventions point at — `specs/` (product scope), `docs/adr/` (architecture decisions), and `skills/` (workflow guardrails) — is plain markdown with no machine-readable metadata. Discovering "what kind of document is this," "is this ADR still current," or "which spec/ADR/skill exists at all" currently requires either grepping the tree or reading `AGENTS.md`'s prose description of the directory layout.

Google Cloud published the Open Knowledge Format (OKF) in June 2026: a vendor-neutral spec for representing knowledge as a directory of markdown files with YAML frontmatter, with `type` as the only required field. It formalizes the "LLM-wiki" pattern so that any agent — not just ones with bespoke integration code — can parse curated context directly from files.

Separately, ADR supersession in this repo is already tracked by hand: `docs/adr/README.md` carries "superseded by" text in its index table, and the ADR body itself is expected to have its `Status` line updated, but nothing checks that the two stay in sync (see the 0004→0018, 0015→0016→0017, 0021→0023 chains, each done as manual prose edits).

## Decision

Adopt OKF-style YAML frontmatter on `specs/`, `docs/adr/`, and `skills/`, plus a repo-root bundle manifest, without moving or rewriting existing document bodies:

- **Frontmatter fields, added to each file:**
  - `type` (required): `spec` | `decision` | `skill` | `workflow-guardrail`
  - `status`: mirrors the existing in-body `Status:` line for ADRs (`proposed | accepted | deprecated | superseded`); `authoritative` for specs; omitted for skills
  - `supersedes` / `superseded_by`: ADR IDs, empty/null by default
  - `title`: mirrors the H1
- **Scope:** `specs/*.md`, `docs/adr/[0-9]*.md`, `skills/*/SKILL.md`. `AGENTS.md` and `CLAUDE.md` are tagged `type: workflow-guardrail` in the manifest but do not get inline frontmatter (they are read directly by harnesses via fixed filenames, not discovered).
- **Bundle manifest:** a repo-root `okf.yaml` (or `.well-known/okf.yaml` if OKF tooling standardizes on that path) enumerating the three directories and their `type`, so any agent can discover the bundle without walking the tree.
- **Drift check:** extend `.claude/hooks/doc-check.sh` (or a sibling script run at the same point) to flag frontmatter that disagrees with filesystem reality — e.g. an ADR whose in-body `Status:` line and frontmatter `status` field differ, or a `superseded_by` pointing at a ADR number that doesn't exist.

This is additive metadata only. Existing rendering (VitePress docs site, GitHub markdown preview, harness-specific loading via `CLAUDE.md`/`.agents/skills`/`opencode.json`) is unaffected — frontmatter sits above the existing H1/body content those tools already read.

## Consequences

**Easier:**
- Any harness or external agent can enumerate this repo's specs/decisions/skills from one manifest instead of per-harness special-casing or tree-walking.
- ADR supersession becomes machine-checkable instead of relying on someone remembering to update both the in-body `Status:` line and the `docs/adr/README.md` index row.
- Positions the repo to interoperate with OKF tooling as it matures (the spec is v0.1, one month old at time of writing).

**Harder / new cost:**
- ~30 existing files need frontmatter added and kept in sync with their in-body `Status:` lines — two sources of truth for the same fact unless the drift check (above) is actually wired up and enforced.
- One more convention for a contributor or agent to learn, on top of the existing `docs/adr/README.md` index and in-body ADR status line.
- OKF v0.1 has no established tooling ecosystem yet; this is a bet on a month-old spec, not adoption of a mature standard. If OKF doesn't gain traction, the frontmatter becomes unused metadata that still has to be maintained.

**Not required by this ADR:** no directory restructuring, no change to how `CLAUDE.md`/`AGENTS.md`/`opencode.json` are loaded by harnesses, no new runtime code in `apps/desktop/` or `crates/orkworksd/`.

## Outcome

Rejected on review. `skills/*/SKILL.md` files already carry YAML frontmatter (`name`/`description`) under the Agent Skills standard that harnesses actively parse; layering OKF keys into the same frontmatter block risks collisions with that existing, externally-specified format, and this ADR did not account for it. The proposed `doc-check.sh` drift check is advisory only (a Stop-hook nudge, not an enforced gate), so the frontmatter `status` field, the in-body `Status:` line, and the `docs/adr/README.md` index row would become three unenforced copies of the same fact rather than a single source of truth. The one concrete problem motivating this ADR — ADR-supersession drift across the `docs/adr/README.md` index (see the 0004→0018, 0015→0017, 0021→0023 chains) — does not require adopting an external, one-month-old, vendor-published spec with no consumers in this repo's toolchain; it is fully addressed by a small standalone consistency check (see `.claude/hooks/check-adr-consistency.sh`, added alongside this ADR). Revisit OKF adoption if/when it gains tooling and a concrete consumer inside this repo's harness set.
