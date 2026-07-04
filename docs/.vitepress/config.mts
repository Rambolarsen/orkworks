import { readdirSync } from 'node:fs'
import { createRequire } from 'node:module'
import { basename, resolve } from 'node:path'
import { defineConfig } from 'vitepress'

const require = createRequire(import.meta.url)

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
    '.codex/**',
    '.vscode/**',
    'DESIGN-IS-*/**',
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
  // srcDir sits outside this package, so pages under specs/ etc. cannot
  // reach docs/node_modules via Node resolution during the SSR build.
  // Pin vue imports to absolute paths instead.
  vite: {
    resolve: {
      alias: [
        { find: /^vue$/, replacement: require.resolve('vue') },
        { find: /^vue\/server-renderer$/, replacement: require.resolve('vue/server-renderer') },
      ],
    },
  },
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
