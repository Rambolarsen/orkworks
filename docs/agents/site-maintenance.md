# Site maintenance

The public site helps prospective users try OrkWorks. It uses VitePress and
GitHub Pages; repository Markdown remains the source of truth. The homepage
lives in `docs/index.md` and its presentation in `docs/.vitepress/theme/`.
User guides live in `docs/user/`. Technical material is linked from
`docs/developers.md`.

## Facts that update automatically

`docs/.vitepress/site.data.mts` loads the built-in coding-tool registry at
build time. Retired entries and generic shells are excluded from the advertised
coding-tool list. This describes current source, not necessarily an older
installer, and does not promise equal integration capability across tools.

When `DOCS_FETCH_RELEASES=1`, the build fetches public GitHub release metadata.
Drafts and prereleases are excluded; only recognized macOS arm64 DMG and Windows
x64 EXE assets with download URLs in this repository become download buttons.
The package version never becomes an assumed public release. With no public
release, the site directs users to source installation. API failures fail the
deployment build, preserving the previously deployed site. Offline/local and
PR builds link to the releases page without claiming they checked availability.

The Docs workflow builds current `main` when docs, specs, the registry, desktop
package version, or Node version change; on release publication/edit/deletion;
and daily at 05:23 UTC to recover missed events. It never publishes an app
release. Facts are rendered into static HTML; visitors do not call GitHub’s API.

## Prose needs evidence and review

PR CI builds the site with dead-link checking and runs the release/tool-list
tests on every PR. `scripts/doc-check.sh` warns when desktop behavior, backend
code, registry, or packaging changes arrive without a public-guide update.
These warnings remain informational; they do not prove semantic accuracy.

The Documentation audit workflow runs Tuesdays at 06:41 UTC or on manual
dispatch. It uses the existing `CLAUDE_CODE_OAUTH_TOKEN` secret and can consume
model usage. It checks the last 14 days of changes and proposes at most three
evidenced corrections to existing user guides. Deterministic steps reject
out-of-scope files, new files, removals, renames, or excess edits before building
the docs and opening one
draft `docs-currency-audit` PR. An existing open proposal prevents another.
Missing credentials are reported in the job summary. It never merges its PR.
Actions must be allowed to create pull requests in repository settings.
PRs created with `GITHUB_TOKEN` do not automatically trigger other workflows;
the audit runs its build before pushing, and a maintainer must arrange normal
PR validation before merging (for example by pushing a reviewed change).

## Verify changes

```bash
node --test docs/.vitepress/site-facts.test.mjs scripts/docs-audit-scope.test.mjs
cd docs
pnpm install --frozen-lockfile
pnpm docs:build
pnpm docs:dev
```

Check the homepage and onboarding at desktop and narrow mobile widths, keyboard
focus, and the source-install fallback. Use `DOCS_FETCH_RELEASES=1 pnpm docs:build`
to test the deployment lookup. The homepage visual is an explicitly labeled
illustration, not a screenshot of a released build. Replace it with a reviewed,
non-sensitive app capture when one is available; do not automatically publish
private session content.
