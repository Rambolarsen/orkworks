---
type: Policy
title: Documentation format
description: OKF 0.2 authoring rules and automated checks for all repository documentation under docs/.
tags: [orkworks, documentation, okf, workflow]
status: stable
---

# Documentation format

All repository-authored Markdown under `docs/` is one Open Knowledge Format
(OKF) 0.2 bundle, rooted at [the documentation index](../index.md). This includes
public guides, agent references, ADRs, validation evidence, implementation plans,
and design history. Dependency files under `node_modules/` and VitePress tooling
and build output under `.vitepress/` are outside the bundle. Product specifications
under the repository's `specs/` directory retain their own authoring contract.

The format is pinned to the [upstream OKF 0.2 specification](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/62432a095456147ee71e70ac6e4dc0d2dea3ac30/okf/SPEC.md).
Update this policy, validator and fixtures together when deliberately changing
versions. Upstream changes do not silently change the repository's contract.

## Concept metadata

Every Markdown concept starts with parseable YAML delimited by `---` lines:

```yaml
---
type: Process Guide
title: A useful, specific title
description: One sentence describing what this document helps the reader do.
tags: [orkworks, workflow]
---
```

The repository requires non-empty `type`, `title`, `description`, and a non-empty
list of non-empty string `tags`. OKF itself requires only `type`; the additional
fields are this repository's authoring profile for discovery and routing.
Type names are descriptive, producer-defined strings. VitePress fields such as
`layout` and other extension fields are allowed and preserved.

Optional OKF `status` is `draft`, `stable`, or `deprecated`. Omit it when no
lifecycle judgment has been made; format conversion does not establish review,
completion, freshness, or product availability. Put plan execution and approval
states such as `active` or `accepted` in the extension field `workflow_status`.
Retain existing review evidence and authoritative status in document bodies.
Do not invent `generated`, `verified`, source attribution, or timestamps to
populate metadata. Historical design and implementation plans describe their
original scope; their presence does not establish current product behavior.

## Directory indexes

Every documentation directory has an `index.md` routing map. Link each immediate
concept and child directory with an ordinary relative Markdown list entry and
a short description. Keep concept paths stable and follow links only as needed.
Update the index in the same change that adds, moves, or removes a document.

Only the bundle-root `docs/index.md` has index frontmatter, containing exactly
`okf_version: "0.2"`. Nested indexes have no frontmatter. `index.md` and `log.md`
are reserved names, not concepts. Optional `log.md` files use date-grouped entries
with `## YYYY-MM-DD` headings and no concept metadata. Existing `README.md`
files remain concepts and retain their paths. The public landing-page concept
lives at `docs/home.md`; VitePress continues to serve it at the site's root URL.
Public-site links to repository-only plans and the ADR template are rendered as
GitHub source links; the source indexes retain portable relative paths.

## Checks and maintenance

```bash
pnpm --dir docs install --frozen-lockfile
pnpm --dir docs docs:check
node --test docs/.vitepress/okf.test.mjs
pnpm --dir docs docs:build
```

The checker scans all documentation, including new files and historical plans.
It fails on malformed or missing metadata, invalid lifecycle states, misplaced
index frontmatter, invalid log date headings, missing directory indexes,
unindexed documents/directories, and broken local index links. It permits
unknown concept types and extension keys. This is a repository authoring check,
not a general-purpose consumer or a complete semantic/provenance audit.

Both `docs:build` and `docs:dev` run it before VitePress starts. PR CI runs its
regression tests and builds the docs on every PR; deployment and the scheduled
documentation-audit workflow use the same gated build. VitePress also checks
site links. The informational doc-drift hook remains a separate currency nudge.
New files receive the same checks without a historical allowlist.
