# Docs Site (Companion Webpage) Design

**Date:** 2026-07-04
**Status:** Approved

## Problem

OrkWorks documentation is spread across `specs/`, `docs/adr/`, `docs/agents/`, and `docs/superpowers/specs/` with no unified navigation, no full-text search, and a clunky reading experience on github.com. There is also no user-facing documentation yet — and any new user docs should be equally consumable by AI agents working in the repo.

## Decision

Build a static docs site with **VitePress**, deployed to **GitHub Pages**, rendering the existing repo markdown **in place**. The markdown files remain the single source of truth; the site is a rendering layer only. Agents keep reading the same files directly — no content duplication, no sync problem.

VitePress was chosen over MkDocs Material (best-in-class UX but adds a Python toolchain to a Node+Rust repo) and over plain curated index pages (solves navigation only). VitePress matches the existing Vite/pnpm toolchain and ships local full-text search, dark mode, and good typography out of the box.

## Architecture

- VitePress config lives in `docs/.vitepress/`, with a minimal `docs/package.json` for the `vitepress` devDependency and `docs:dev` / `docs:build` scripts. This stays isolated from `apps/desktop` — no pnpm workspace changes.
- `srcDir` points at the repo root so the site includes `specs/`, `docs/adr/`, `docs/agents/`, `docs/user/`, and `docs/superpowers/specs/` where they already live. `srcExclude` filters out `apps/`, `crates/`, `node_modules`, `.agents/`, and other non-doc trees.
- Base path is `/orkworks/`; site URL is `https://rambolarsen.github.io/orkworks/`.

## Content and navigation

- Landing page: `docs/index.md` (VitePress home layout) — what OrkWorks is, links into the sections.
- Sidebar groups:
  - **User Guide** — new `docs/user/` directory, seeded with one getting-started page (install, first session). Grows over time; agents can read it like any other repo markdown.
  - **Specs** — the five authoritative specs in `specs/`.
  - **ADRs** — `docs/adr/`.
  - **Agent docs** — `docs/agents/`.
  - **Design history** — `docs/superpowers/specs/`, in a collapsed sidebar group. Included and searchable, but visually secondary: these are working artifacts, not documentation.
- Search: VitePress built-in local search. No external service.

## Deploy

New `.github/workflows/docs.yml`:

- Triggers on push to `main` touching doc paths (`docs/**`, `specs/**`, `*.md` doc files) and the VitePress config.
- Builds the site and deploys with the official GitHub Pages actions (`actions/configure-pages`, `actions/upload-pages-artifact`, `actions/deploy-pages`).
- `release.yml` and `pr-ci.yml` are untouched.

## Dead links

VitePress fails the build on dead links. Keep that on — it keeps the docs honest for humans and agents alike. Fix or explicitly allowlist (via `ignoreDeadLinks` entries) the existing links that intentionally point at code files rather than docs.

## Verification

- `pnpm docs:build` passes locally before the workflow lands.
- The Pages deploy workflow is the ongoing check; a dead link or broken config fails the build on `main`.

## Non-goals

- No content migration or restructuring of existing docs.
- No versioned docs, no analytics, no custom domain, no comments/feedback widgets.
- No changes to how agents locate or read documentation.
