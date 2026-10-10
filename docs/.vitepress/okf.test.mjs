import assert from 'node:assert/strict'
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { spawnSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { test } from 'node:test'
import { validateBundle } from './okf.mjs'

const concept = `---
type: Reference
title: Test concept
description: Describes the test concept.
tags: [test]
---
# Test concept
`
const index = `---
okf_version: "0.2"
---
# Knowledge

- [Concept](concept.md) — Describes the test concept.
`

function fixture(t, files = {}) {
  const root = mkdtempSync(join(tmpdir(), 'orkworks-okf-'))
  t.after(() => rmSync(root, { recursive: true, force: true }))
  for (const [name, text] of Object.entries({ 'index.md': index, 'concept.md': concept, ...files })) {
    mkdirSync(join(root, name, '..'), { recursive: true })
    writeFileSync(join(root, name), text)
  }
  return root
}

test('accepts valid YAML extensions and indexes with relative directory links', t => {
  const root = fixture(t, {
    'index.md': index + '- [Nested](nested/) — More knowledge.\n',
    'concept.md': concept.replace('tags: [test]', 'tags: [test]\nlayout: page\ncustom: {owner: team}'),
    'nested/index.md': '# Nested\n\n- [Other](other.md) — Another concept.\n',
    'nested/other.md': concept,
    'node_modules/bad.md': 'Ignored dependency',
    '.vitepress/cache/bad.md': 'Ignored build cache',
  })
  assert.deepEqual(validateBundle(root), [])
})

for (const [name, content, expected] of [
  ['missing frontmatter', '# No metadata', /frontmatter/],
  ['unterminated frontmatter', '---\ntype: Reference', /frontmatter/],
  ['invalid YAML', concept.replace('type: Reference', 'type: [broken'), /YAML/],
  ['duplicate keys', concept.replace('type: Reference', 'type: Reference\ntype: Plan'), /YAML/],
  ['non-mapping YAML', '---\n- Reference\n---\n# Body', /mapping/],
  ['missing type', concept.replace('type: Reference\n', ''), /type/],
  ['numeric type', concept.replace('type: Reference', 'type: 12'), /type/],
  ['empty title', concept.replace('title: Test concept', 'title: " "'), /title/],
  ['missing description', concept.replace('description: Describes the test concept.\n', ''), /description/],
  ['non-list tags', concept.replace('tags: [test]', 'tags: test'), /tags/],
  ['empty tag', concept.replace('tags: [test]', 'tags: [""]'), /tags/],
  ['unknown lifecycle status', concept.replace('tags: [test]', 'tags: [test]\nstatus: accepted'), /status/],
]) {
  test(`rejects ${name} with the concept path`, t => {
    const errors = validateBundle(fixture(t, { 'concept.md': content }))
    assert.ok(errors.some(e => e.includes('concept.md') && expected.test(e)), errors.join('\n'))
  })
}

test('checks new files recursively and reports missing index entries', t => {
  const errors = validateBundle(fixture(t, { 'new.md': concept, 'nested/new.md': '# Missing metadata' }))
  assert.ok(errors.some(e => e.includes('new.md') && /index entry/.test(e)))
  assert.ok(errors.some(e => e.includes('nested\/new.md') && /frontmatter/.test(e)))
  assert.ok(errors.some(e => e.includes('nested\/index.md') && /missing/.test(e)))
})

test('rejects invalid root versions and concept metadata in reserved indexes', t => {
  const errors = validateBundle(fixture(t, {
    'index.md': index.replace('"0.2"', '"9.0"') + '- [Nested](nested/) — More.\n',
    'nested/index.md': concept,
  }))
  assert.ok(errors.some(e => e.startsWith('index.md:') && /okf_version/.test(e)))
  assert.ok(errors.some(e => e.includes('nested\/index.md') && /frontmatter/.test(e)))
})

test('rejects broken index links and entries without descriptions', t => {
  const errors = validateBundle(fixture(t, {
    'index.md': index.replace(' — Describes the test concept.', '') + '- [Gone](gone.md) — Missing.\n',
  }))
  assert.ok(errors.some(e => /description/.test(e)))
  assert.ok(errors.some(e => /gone.md/.test(e)))
})

test('checks log date headings and allows optional lifecycle metadata', t => {
  const root = fixture(t, {
    'concept.md': concept.replace('tags: [test]', 'tags: [test]\nstatus: deprecated'),
    'index.md': index + '- [History](log.md) — Update history.\n',
    'log.md': '# Updates\n\n## 2026-10-10\n- Created the bundle.\n',
  })
  assert.deepEqual(validateBundle(root), [])
  writeFileSync(join(root, 'log.md'), '# Updates\n\n## Yesterday\n- Updated.\n')
  assert.ok(validateBundle(root).some(e => /log.md/.test(e) && /date/.test(e)))
})

test('CLI fails for malformed docs and succeeds after correction', t => {
  const root = fixture(t, { 'concept.md': '# Missing metadata' })
  const cli = fileURLToPath(new URL('./okf.mjs', import.meta.url))
  const fail = spawnSync(process.execPath, [cli, root], { encoding: 'utf8' })
  assert.equal(fail.status, 1)
  assert.match(fail.stderr, /concept.md/)
  writeFileSync(join(root, 'concept.md'), concept)
  const pass = spawnSync(process.execPath, [cli, root], { encoding: 'utf8' })
  assert.equal(pass.status, 0, pass.stderr)
})

test('recognizes escaped brackets in index labels', t => {
  assert.deepEqual(validateBundle(fixture(t, {
    'index.md': index.replace('[Concept]', String.raw`[Concept \[example\]]`),
  })), [])
})

test('does not exclude nested authored directories named like root tooling', t => {
  const errors = validateBundle(fixture(t, { 'nested/.vitepress/guide.md': '# Missing metadata' }))
  assert.ok(errors.some(e => e.includes('nested/.vitepress/guide.md') && /frontmatter/.test(e)))
})
