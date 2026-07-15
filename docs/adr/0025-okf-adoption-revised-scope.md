---
type: decision
status: accepted
title: "Adopt OKF with revised scope: manifest + frontmatter on specs/ADRs, skills manifest-only"
---

# Adopt OKF with revised scope: manifest + frontmatter on specs/ADRs, skills manifest-only

- Status: accepted
- Deciders: user
- Date: 2026-07-15
- Supersedes: 0024 (rejected proposal, revised here)

## Context

ADR 0024 proposed adopting the [Open Knowledge Format (OKF)](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md) — Google Cloud's June 2026 vendor-neutral spec for representing knowledge as markdown directories with YAML frontmatter — across `specs/`, `docs/adr/`, and `skills/`. Review rejected it on two concrete findings:

1. `skills/*/SKILL.md` files already carry YAML frontmatter (`name`/`description`) owned by the Agent Skills standard, which harnesses actively parse. Adding OKF keys to the same block risks breaking strict skill parsers, and two external specs would co-own one frontmatter block.
2. The proposed drift check was advisory-only, so frontmatter `status`, the in-body ADR `Status:` line, and the `docs/adr/README.md` index would have been three unenforced copies of one fact.

The owner has since decided the underlying goal — making repo knowledge easier for agents to discover and consume — is worth pursuing, and directed adoption. Both findings are addressable by scoping rather than abandoning: keep OKF frontmatter off files another spec owns, and make the consistency check (added alongside ADR 0024's rejection as `.claude/hooks/check-adr-consistency.sh`) cover the new frontmatter so drift is caught, not just possible.

## Decision

Adopt OKF with this scope:

- **Bundle manifest** `okf.yaml` at the repo root: enumerates the knowledge surfaces (`specs/`, `docs/adr/`, `skills/`, `AGENTS.md`, `docs/agents/`), each with a `type` and a short description. This is the single discovery entry point — an agent that has never seen this repo reads one file and knows what knowledge exists and where. Note this manifest is a repo-specific construct, not part of the OKF spec: OKF v0.1 defines no manifest file (its optional reserved discovery file is `index.md`, and a strictly conformant bundle requires frontmatter with `type` on *every* markdown file in its tree, which a whole repo can't satisfy). `okf.yaml` scopes which subtrees follow OKF conventions.
- **Inline OKF frontmatter** on `specs/*.md` and `docs/adr/[0-9]*.md` only: `type` (`spec` | `decision`), `status`, `title`, and for ADRs `superseded_by` where a successor ADR exists. These files have no competing frontmatter owner.
- **Skills are manifest-only.** `skills/*/SKILL.md` files are typed and described in `okf.yaml` but their frontmatter is untouched — the Agent Skills standard owns it.
- **Consistency check extended.** `.claude/hooks/check-adr-consistency.sh` now also verifies each ADR's frontmatter `status` matches its in-body `Status:` line, so the duplicated fact is machine-checked on every `docs/adr/` change via the existing `doc-check.sh` wiring.
- **New ADRs/specs must include frontmatter.** The ADR template gains the frontmatter block so future decisions carry it from birth.

## Consequences

- Agents (including non-Claude harnesses and future Peon/Taskmaster features) can discover the repo's knowledge from one manifest and read per-file type/status without parsing prose conventions.
- ADR status now lives in three places (frontmatter, body, index), but all three are cross-checked by a script that runs whenever `docs/adr/` changes — drift is caught at the session boundary rather than accumulating.
- The Agent Skills frontmatter collision from ADR 0024's review is avoided entirely: no OKF keys on `SKILL.md`.
- OKF is still a young spec (v0.1). The bet is now small and reversible: if OKF stalls, `okf.yaml` and the frontmatter are inert metadata that can be dropped in one commit; nothing at runtime depends on them.
- The manifest is one more file to keep current when knowledge directories are added or restructured; `doc-check.sh` triggers on those paths already, and AGENTS.md's doc-currency rule applies.
