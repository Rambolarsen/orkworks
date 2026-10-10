import assert from 'node:assert/strict'
import { test } from 'node:test'
import { createMarkdownRenderer } from 'vitepress'
import { repositoryOnlyLinks } from './repository-links.mjs'

const md = await createMarkdownRenderer('/tmp', { config: repositoryOnlyLinks })

test('site links to unpublished OKF index entries go to their repository source', () => {
  const plans = md.render('[Plans](plans/index.md)', { relativePath: 'docs/superpowers/index.md' })
  assert.match(plans, /href="https:\/\/github.com\/Rambolarsen\/orkworks\/blob\/main\/docs\/superpowers\/plans\/index.md"/)
  const template = md.render('[Template](template.md)', { relativePath: 'docs/adr/index.md' })
  assert.match(template, /href="https:\/\/github.com\/Rambolarsen\/orkworks\/blob\/main\/docs\/adr\/template.md"/)
})

test('published, fragment and external links retain normal rendering', () => {
  const html = md.render('[Guide](guide.md) [Section](#section) [Other](https://example.com/template.md)', {
    relativePath: 'docs/user/index.md',
  })
  assert.match(html, /href=".\/guide.html"/)
  assert.match(html, /href="#section"/)
  assert.match(html, /href="https:\/\/example.com\/template.md"/)
})
