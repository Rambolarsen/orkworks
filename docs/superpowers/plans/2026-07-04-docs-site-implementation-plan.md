# Docs Site (VitePress on GitHub Pages) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Static docs site rendering the repo's existing markdown in place, with search and navigation, deployed to `https://rambolarsen.github.io/orkworks/`.

**Architecture:** VitePress project rooted at `docs/` (config in `docs/.vitepress/`, own `package.json`, no pnpm workspace changes) with `srcDir` pointing at the repo root so `specs/`, `docs/adr/`, `docs/agents/`, `docs/user/`, and `docs/superpowers/specs/` render where they live. A new `.github/workflows/docs.yml` builds and deploys to GitHub Pages on doc-path pushes to `main`.

**Tech Stack:** VitePress 1.x, pnpm 11.9.0, Node 22 (`.nvmrc`), GitHub Pages via `actions/deploy-pages`.

**Spec:** `docs/superpowers/specs/2026-07-04-docs-site-design.md` · **Issue:** #128

**Note on TDD:** This is docs/config-only work (no `apps/desktop/src`, no `crates/`). The verification loop is `pnpm docs:build` passing with dead-link checking on, not unit tests.

---

### Task 1: Worktree setup

The primary checkout is shared with other agents — never branch-switch there. This touches `.github/workflows/`, so it is **not** docs-only and needs a branch + PR.

- [ ] **Step 1: Create the worktree**

```bash
cd /Users/froomiebot/workspace/orkworks
git fetch origin && git worktree add ../orkworks-docs-site -b docs-site origin/main
cd ../orkworks-docs-site
```

- [ ] **Step 2: Verify you are in the worktree on the right branch**

Run: `git branch --show-current && pwd`
Expected: `docs-site` and `/Users/froomiebot/workspace/orkworks-docs-site`

All subsequent tasks run from inside `/Users/froomiebot/workspace/orkworks-docs-site`.

---

### Task 2: VitePress scaffold

**Files:**
- Create: `docs/package.json`
- Modify: `.gitignore` (repo root)

- [ ] **Step 1: Create `docs/package.json`**

```json
{
  "name": "orkworks-docs",
  "private": true,
  "type": "module",
  "scripts": {
    "docs:dev": "vitepress dev",
    "docs:build": "vitepress build",
    "docs:preview": "vitepress preview"
  },
  "devDependencies": {
    "vitepress": "^1.6.3"
  }
}
```

- [ ] **Step 2: Install (from `docs/`, standalone — no workspace)**

Run: `cd docs && pnpm install && cd ..`
Expected: `docs/pnpm-lock.yaml` and `docs/node_modules/` created, no errors. The lockfile gets committed; `node_modules` does not.

- [ ] **Step 3: Add ignore entries to the root `.gitignore`**

Append:

```gitignore
# VitePress docs site
docs/node_modules/
docs/.vitepress/dist/
docs/.vitepress/cache/
```

- [ ] **Step 4: Commit**

```bash
git add docs/package.json docs/pnpm-lock.yaml .gitignore
git commit -m "feat: scaffold VitePress docs site package"
```

---

### Task 3: VitePress config

**Files:**
- Create: `docs/.vitepress/config.mts`

- [ ] **Step 1: Write `docs/.vitepress/config.mts`**

```ts
import { readdirSync } from 'node:fs'
import { basename, resolve } from 'node:path'
import { defineConfig } from 'vitepress'

// Project root is docs/ (where .vitepress lives); srcDir is the repo root
// so the existing markdown renders in place — the repo files stay the
// single source of truth for humans and agents alike.
const repoRoot = resolve(import.meta.dirname, '../..')

function mdPages(relDir: string): { text: string; link: string }[] {
  return readdirSync(resolve(repoRoot, relDir))
    .filter((f) => f.endsWith('.md') && !['README.md', 'template.md'].includes(f))
    .sort()
    .map((f) => ({
      text: basename(f, '.md'),
      link: `/${relDir}/${basename(f, '.md')}`,
    }))
}

export default defineConfig({
  title: 'OrkWorks',
  description: 'Local-first mission control for AI coding sessions',
  base: '/orkworks/',
  srcDir: '..',
  srcExclude: [
    'README.md',
    'AGENTS.md',
    'CLAUDE.md',
    'apps/**',
    'crates/**',
    'skills/**',
    '.agents/**',
    '.claude/**',
    '.github/**',
    '.opencode/**',
    '**/node_modules/**',
    'docs/.vitepress/**',
    'docs/superpowers/plans/**',
    'docs/adr/template.md',
  ],
  // Serve docs/index.md as the site home page.
  rewrites: {
    'docs/index.md': 'index.md',
  },
  // Dead-link checking stays ON (build fails on dead links). These entries
  // only apply to links that are already dead: links into code files, and
  // links to repo markdown deliberately excluded from the site.
  ignoreDeadLinks: [
    /^https?:\/\/localhost/,
    /\.(rs|ts|tsx|mts|mjs|js|json|ya?ml|toml|sh|lock|css|html)$/,
    /(^|\/)(AGENTS|CLAUDE|README)\.md/,
    /superpowers\/plans\//,
  ],
  themeConfig: {
    nav: [
      { text: 'User Guide', link: '/docs/user/getting-started' },
      { text: 'Specs', link: '/specs/orkworks-mvp' },
    ],
    socialLinks: [
      { icon: 'github', link: 'https://github.com/Rambolarsen/orkworks' },
    ],
    search: { provider: 'local' },
    sidebar: [
      {
        text: 'User Guide',
        items: [{ text: 'Getting started', link: '/docs/user/getting-started' }],
      },
      {
        text: 'Specs',
        items: [
          { text: 'OrkWorks MVP', link: '/specs/orkworks-mvp' },
          { text: 'Native harness voice support', link: '/specs/native-harness-voice-support' },
          { text: 'Release pipeline', link: '/specs/release-pipeline' },
          { text: 'Review queue', link: '/specs/review-queue' },
          { text: 'Taskmaster', link: '/specs/taskmaster' },
        ],
      },
      {
        text: 'ADRs',
        collapsed: true,
        items: [
          { text: 'Index', link: '/docs/adr/README' },
          ...mdPages('docs/adr'),
        ],
      },
      {
        text: 'Agent docs',
        items: [
          { text: 'Architecture', link: '/docs/agents/architecture' },
          { text: 'Domain entities', link: '/docs/agents/domain-entities' },
          { text: 'APM', link: '/docs/agents/apm' },
        ],
      },
      {
        text: 'Design history',
        collapsed: true,
        items: mdPages('docs/superpowers/specs'),
      },
    ],
  },
})
```

- [ ] **Step 2: Commit**

```bash
git add docs/.vitepress/config.mts
git commit -m "feat: add VitePress config rendering repo markdown in place"
```

(The build is not expected to pass yet — the home page and user guide it links to don't exist until Tasks 4–5.)

---

### Task 4: Landing page

**Files:**
- Create: `docs/index.md`

- [ ] **Step 1: Write `docs/index.md`**

```md
---
layout: home
hero:
  name: OrkWorks
  text: Local-first mission control for AI coding sessions
  tagline: Peons observe individual sessions. Taskmaster recommends what should happen next — across coding tools, models, reviews, capacity, and Git context.
  actions:
    - theme: brand
      text: Getting started
      link: /docs/user/getting-started
    - theme: alt
      text: Read the MVP spec
      link: /specs/orkworks-mvp
features:
  - title: User Guide
    details: Install OrkWorks and run your first session.
    link: /docs/user/getting-started
  - title: Specs
    details: Authoritative product scope, architecture, and milestones.
    link: /specs/orkworks-mvp
  - title: ADRs
    details: The architecture decisions and why they were made.
    link: /docs/adr/README
---
```

- [ ] **Step 2: Commit**

```bash
git add docs/index.md
git commit -m "feat: add docs site landing page"
```

---

### Task 5: User guide seed

**Files:**
- Create: `docs/user/getting-started.md`

- [ ] **Step 1: Write `docs/user/getting-started.md`**

````md
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
````

- [ ] **Step 2: Commit**

```bash
git add docs/user/getting-started.md
git commit -m "feat: seed user guide with getting-started page"
```

---

### Task 6: Build, fix dead links, verify locally

- [ ] **Step 1: Run the build**

Run: `cd docs && pnpm docs:build`
Expected: either `build complete` or a list of `(!) Found dead link(s)` errors.

- [ ] **Step 2: Resolve every dead link the build reports**

Decision rule, per reported link:
- Target is a code file, an excluded markdown file, or a localhost URL and it slipped past the seeded `ignoreDeadLinks` patterns → add one **targeted** regex/string entry to `ignoreDeadLinks` in `docs/.vitepress/config.mts`. Do not broaden to catch-all patterns.
- Target is a doc that moved, was renamed, or the link has a typo → **fix the link in the source markdown**. That fix improves the docs for agents too.

Re-run `pnpm docs:build` until it exits 0.

- [ ] **Step 3: Spot-check the rendered site**

Run: `pnpm docs:preview` (from `docs/`), open `http://localhost:4173/orkworks/`.
Check: home page renders, sidebar shows all five groups, ADRs and Design history are collapsed, search finds "Taskmaster", a spec page and an ADR render correctly. Then stop the preview server.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "fix: resolve dead links for docs site build"
```

(If Step 2 required no changes, skip this commit.)

---

### Task 7: Deploy workflow

**Files:**
- Create: `.github/workflows/docs.yml`

- [ ] **Step 1: Write `.github/workflows/docs.yml`**

```yaml
# Builds the VitePress docs site and deploys it to GitHub Pages.
# Triggers only on doc paths — release.yml and pr-ci.yml are unaffected.
name: Docs

on:
  push:
    branches: [main]
    paths:
      - 'docs/**'
      - 'specs/**'
      - '.github/workflows/docs.yml'
  workflow_dispatch:

permissions:
  contents: read
  pages: write
  id-token: write

concurrency:
  group: pages
  cancel-in-progress: false

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v4
        with:
          version: 11.9.0
      - uses: actions/setup-node@v4
        with:
          node-version-file: .nvmrc
          cache: pnpm
          cache-dependency-path: docs/pnpm-lock.yaml
      - run: pnpm install --frozen-lockfile
        working-directory: docs
      - run: pnpm docs:build
        working-directory: docs
      - uses: actions/configure-pages@v5
      - uses: actions/upload-pages-artifact@v3
        with:
          path: docs/.vitepress/dist

  deploy:
    needs: build
    runs-on: ubuntu-latest
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    steps:
      - id: deployment
        uses: actions/deploy-pages@v4
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/docs.yml
git commit -m "feat: add GitHub Pages deploy workflow for docs site"
```

---

### Task 8: Enable GitHub Pages (build type: workflow)

- [ ] **Step 1: Enable Pages via the API**

Run:

```bash
gh api -X POST repos/Rambolarsen/orkworks/pages -f build_type=workflow
```

Expected: JSON response with `"build_type": "workflow"`.
If it fails with 409 (already exists), run instead:

```bash
gh api -X PUT repos/Rambolarsen/orkworks/pages -f build_type=workflow
```

- [ ] **Step 2: Verify**

Run: `gh api repos/Rambolarsen/orkworks/pages --jq .build_type`
Expected: `workflow`

---

### Task 9: Update AGENTS.md and README.md

**Files:**
- Modify: `AGENTS.md` (workflow classes list in "Package manager" section; new note near "Authoritative specs")
- Modify: `README.md` (docs site link near the top)

- [ ] **Step 1: Add the docs workflow to the AGENTS.md workflow-classes list**

In the list of workflow classes (currently `release.yml`, `pr-ci.yml`, `quality-audit.yml`), append:

```md
- `.github/workflows/docs.yml` builds the VitePress docs site and deploys it to GitHub Pages on doc-path pushes to `main`
```

Adjust the sentence introducing the list ("three distinct workflow classes" → "four distinct workflow classes").

- [ ] **Step 2: Add a docs-site note to AGENTS.md**

Directly after the "Authoritative specs" section, add:

```md
## Docs site

Repo markdown is rendered as a docs site at https://rambolarsen.github.io/orkworks/ (VitePress config in `docs/.vitepress/`, deployed by `.github/workflows/docs.yml`). The markdown files in the repo are the single source of truth — the site is a rendering layer only. User-facing documentation lives in `docs/user/`; agents read it like any other repo markdown. The build fails on dead links, so keep links valid when moving or renaming docs.
```

- [ ] **Step 3: Add the docs site link to README.md**

Near the top of `README.md` (after the opening description paragraph), add:

```md
**Documentation:** https://rambolarsen.github.io/orkworks/
```

- [ ] **Step 4: Commit**

```bash
git add AGENTS.md README.md
git commit -m "docs: document the docs site in AGENTS.md and README.md"
```

---

### Task 10: PR, merge, verify deploy, clean up

- [ ] **Step 1: Doc currency check**

Run: `bash .claude/hooks/doc-check.sh`
Expected: no unaddressed flagged files (AGENTS.md/README.md were updated in Task 9).

- [ ] **Step 2: Final local build check**

Run: `cd docs && pnpm docs:build && cd ..`
Expected: exit 0, `build complete`.

- [ ] **Step 3: Push and open the PR**

```bash
git push -u origin docs-site
gh pr create --title "Docs site: VitePress on GitHub Pages" --body "$(cat <<'EOF'
Implements docs/superpowers/specs/2026-07-04-docs-site-design.md.

Closes #128

- VitePress in docs/.vitepress/ rendering repo markdown in place (srcDir = repo root)
- Sidebar: User Guide (new docs/user/), Specs, ADRs, Agent docs, collapsed Design history
- Built-in local search; dead-link checking stays on
- .github/workflows/docs.yml deploys to Pages on doc-path pushes to main
- AGENTS.md/README.md updated

Review gate note: no apps/desktop or crates code touched, so the /code-review gate does not apply; PR CI runs the no-op check for non-code paths.

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

- [ ] **Step 4: Wait for PR checks, then squash-merge**

Run: `gh pr checks --watch`
Expected: all green (non-code paths get the lightweight no-op check).

```bash
gh pr merge --squash --delete-branch
```

- [ ] **Step 5: Verify the deploy**

Run: `gh run list --workflow=docs.yml --limit 1` and wait for it to complete (`gh run watch <id>`).
Expected: conclusion `success`.

Then: `curl -sI https://rambolarsen.github.io/orkworks/ | head -1`
Expected: `HTTP/2 200` (first-ever Pages deploy can take a couple of minutes to propagate; retry before assuming failure).

- [ ] **Step 6: Clean up the worktree**

```bash
cd /Users/froomiebot/workspace/orkworks
git worktree remove ../orkworks-docs-site
git worktree prune
git pull origin main
```

- [ ] **Step 7: Close the loop**

Confirm issue #128 auto-closed by the merge (`gh issue view 128 --json state --jq .state` → `CLOSED`).
