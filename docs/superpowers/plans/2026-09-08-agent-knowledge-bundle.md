# Agent Knowledge Bundle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reduce the root `AGENTS.md` to a concise repository router while organizing durable agent guidance as an OKF v0.2-style Markdown knowledge bundle under `docs/agents/`.

**Architecture:** Keep load-bearing repository rules in `AGENTS.md`, including product boundaries, authoritative-spec rules, scoped-instruction routing, branch/PR requirements, verification requirements, and the single-active-context invariant. Move explanatory context and detailed reference material into focused concept documents, each discoverable through `docs/agents/index.md`. Add only Markdown/YAML metadata; do not add a parser, validator, dependency, or runtime integration.

**Tech Stack:** Markdown, YAML frontmatter, Git, VitePress, `scripts/doc-check.sh`, and the existing docs build.

**Spec:** `docs/superpowers/specs/2026-09-08-agent-knowledge-bundle-design.md`

## Global Constraints

- `AGENTS.md` remains authoritative for repository-wide rules and must retain load-bearing workflow and product constraints.
- `docs/agents/index.md` is the progressive-disclosure entry point for the bundle.
- Every `docs/agents/*.md` concept has `type`, `title`, `description`, and `tags` frontmatter; the root index may use only OKF's `okf_version` metadata.
- Do not invent provenance, authorship, freshness, or verification claims.
- Preserve the existing unstaged `Assumption discipline` change in `AGENTS.md`; do not discard or silently rewrite it.
- Do not change product behavior, source code, dependencies, CI behavior, or authoritative product specifications.

---

### Task 1: Create the OKF bundle index and project context

**Files:**
- Create: `docs/agents/index.md`
- Create: `docs/agents/project-context.md`

**Interfaces:**
- Consumes: Root sections `Identity`, `State of the repo`, `Package manager`, `CI routing`, and `Containerized dev environment (optional)`.
- Produces: A stable bundle entry point and a linked project-context concept.

- [ ] **Step 1: Create `docs/agents/index.md`**

Use an OKF bundle-root `okf_version: "0.2"` header, then add grouped links for `project-context.md`, `product-boundaries.md`, `architecture.md`, `apm.md`, `development-workflow.md`, `subagent-model-policy.md`, `harness-integration-contracts.md`, `harness-instruction-coverage.md`, `domain-entities.md`, `codex-api-troubleshooting.md`, and `peon-timeout-troubleshooting.md`. Each entry must include a short description.

- [ ] **Step 2: Create `docs/agents/project-context.md`**

Add this frontmatter:

```yaml
---
type: Agent Guide
title: Project context
description: Product identity, repository state, CI routing, and development environments.
tags: [orkworks, repository, ci, development]
status: stable
---
```

Create the detailed CI workflow descriptions and optional container commands here from the corresponding root-guide material. Root-guide de-duplication is intentionally owned by Task 2 because Task 1 must not edit the user-owned `AGENTS.md` change. Keep the package-manager rule concise and link to `apps/desktop/AGENTS.md` for desktop-specific commands.

- [ ] **Step 3: Verify the index targets and record forward references**

Run `for path in docs/agents/*.md; do test -f "$path" || exit 1; done` and confirm all links to existing files resolve. The links to `product-boundaries.md` and `development-workflow.md` are intentional forward references to Task 2; Task 4 performs the final all-links check after those files exist. The bundle-wide frontmatter invariant is completed by Task 3, not by this intermediate task.

- [ ] **Step 4: Commit the self-contained context bundle**

Run `git add docs/agents/index.md docs/agents/project-context.md && git commit -m "docs: add agent knowledge bundle index"`.

### Task 2: Add product and workflow concepts and the root router

**Files:**
- Create: `docs/agents/product-boundaries.md`
- Create: `docs/agents/development-workflow.md`
- Modify: `AGENTS.md`

**Interfaces:**
- Consumes: Root sections `Issue board`, `Authoritative specs`, `Assumption discipline`, `Key naming`, `Key conventions from specs`, and `Product design principles`.
- Produces: Focused reference concepts and a root `## Documentation` section.

- [ ] **Step 1: Create `docs/agents/product-boundaries.md`**

Use `type: Product Reference`, title `Product boundaries and terminology`, description `Product scope, naming rules, non-goals, and load-bearing UX constraints.`, tags `[orkworks, product-scope, terminology, ux]`, and `status: stable`. Move detailed terminology, spec-derived conventions, and UX explanations here; preserve ADR and spec links. Leave concise root summaries of non-goals and the single-active-context rule.

- [ ] **Step 2: Create `docs/agents/development-workflow.md`**

Use `type: Process Guide`, title `Development workflow reference`, description `Detailed issue prioritization, assumption discipline, decision tracking, and maintenance checks.`, tags `[orkworks, workflow, agents, documentation]`, and `status: stable`. Move explanatory detail for issue prioritization, assumption discipline, ADR maintenance, doc currency, worktree currency, and keeping `AGENTS.md`/`README.md` current here. Keep root-level obligations and links authoritative.

- [ ] **Step 3: Add the root `## Documentation` section**

Insert this section after `## Authoritative specs`, without removing the existing unstaged `## Assumption discipline` section:

```markdown
## Documentation

Durable project context is organized as an OKF v0.2-style knowledge bundle in
[`docs/agents/index.md`](docs/agents/index.md). Read the index and the relevant
concept before changing documentation, architecture, integrations, or other
work whose context is not fully captured in this root guide. The root guide
remains authoritative for repository-wide rules; the bundle provides focused
reference detail and progressive disclosure.
```

- [ ] **Step 4: Deduplicate only explanatory prose**

Replace migrated detail with concise summaries and links. Keep issue-board access and priority rules, authoritative-spec requirements, branch/PR rules, required skills, scoped instruction routing, verification commands, and hard product boundaries in `AGENTS.md`.

- [ ] **Step 5: Commit the product/workflow concepts**

Run `git diff --check`, then `git add AGENTS.md docs/agents/product-boundaries.md docs/agents/development-workflow.md && git commit -m "docs: route agent guidance through knowledge bundle"`.

### Task 3: Normalize existing agent concepts with OKF metadata

**Files:** `docs/agents/apm.md`, `architecture.md`, `codex-api-troubleshooting.md`, `domain-entities.md`, `harness-instruction-coverage.md`, `harness-integration-contracts.md`, `peon-timeout-troubleshooting.md`, and `subagent-model-policy.md`.
- Also modify: `AGENTS.md` for the metadata-protocol summary; preserve the existing unstaged `Assumption discipline` hunk and do not stage it.

**Interfaces:**
- Consumes: Existing specialized agent guidance and the bundle index.
- Produces: OKF-compatible concepts that preserve existing bodies and links.

- [ ] **Step 1: Add frontmatter to every existing concept**

Add `type`, `title`, `description`, `tags`, and `status: stable` before each existing H1. Use `Architecture Reference` for `architecture.md`, `Troubleshooting Guide` for the two troubleshooting files, `Domain Reference` for `domain-entities.md`, `Integration Reference` for the two harness files, and `Policy` for `subagent-model-policy.md`; use accurate one-sentence descriptions and focused tags.

- [ ] **Step 2: Move metadata-protocol detail into `architecture.md`**

Transfer the detailed paths, bounds, lifecycle descriptions, authentication rules, and ADR references from root `Metadata protocol` into `docs/agents/architecture.md`. Leave a concise root summary with the load-bearing invariants and a link to the concept.

- [ ] **Step 3: Normalize cross-links**

Use relative Markdown links for links between `docs/agents/` concepts. Preserve external URLs and ADR links unchanged.

- [ ] **Step 4: Verify metadata shape**

Run `for file in docs/agents/*.md; do sed -n '1,8p' "$file"; done`. Confirm every non-index concept begins with the required fields and the index begins with only the bundle-root `okf_version` metadata.

- [ ] **Step 5: Commit normalized concepts and the root metadata summary**

Stage the intended `AGENTS.md` metadata-summary edits and all changed `docs/agents/` concepts without staging the pre-existing `Assumption discipline` hunk. Verify the staged and unstaged diffs before committing, then run `git commit -m "docs: add metadata to agent concepts"`.

### Task 4: Validate the complete bundle and root guide

**Files:** `AGENTS.md`, `docs/agents/index.md`, and any concept whose link or summary needs correction.

**Interfaces:**
- Consumes: All concepts and routing created by Tasks 1–3.
- Produces: A coherent, linkable docs bundle ready for review.

- [ ] **Step 1: Review coverage and size**

Run `rg -n '^## ' AGENTS.md` and `wc -l AGENTS.md docs/agents/*.md`. Confirm every removed root section has a linked concept and every load-bearing rule remains directly available from `AGENTS.md`.

- [ ] **Step 2: Run documentation drift checks**

Run `bash scripts/doc-check.sh`. Resolve every flagged documentation file before continuing.

- [ ] **Step 3: Build the docs site**

Run `pnpm --dir docs run docs:build`, the existing VitePress build script in `docs/package.json`. Expected result: VitePress completes without dead-link errors.

- [ ] **Step 4: Review the final diff**

Run:

```bash
git status --short
git diff -- AGENTS.md
git diff --check
```

Confirm the pre-existing `Assumption discipline` change remains present and no unrelated files changed.

- [ ] **Step 5: Run the worktree currency check**

Run `bash .claude/hooks/worktree-check.sh`. Record stale or merged worktrees for their owners; do not remove another owner’s worktree.

## Plan self-review

- Spec coverage: Tasks 1–4 cover the index, concept metadata, focused new concepts, root router, preservation of load-bearing rules, links, doc drift, docs build, and rollback-safe diff review.
- Placeholder scan: No unresolved placeholder or unspecified implementation step remains.
- Link consistency: The index targets match the existing and planned `docs/agents/*.md` files, and the metadata keys match the design spec.
