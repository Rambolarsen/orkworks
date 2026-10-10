import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs'
import { basename, dirname, join, relative, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { parseDocument } from 'yaml'

const nonempty = value => typeof value === 'string' && value.trim().length > 0
const mapping = value => value !== null && typeof value === 'object' && !Array.isArray(value)

// Repository authoring profile for OKF 0.2, not a general-purpose OKF consumer.
// Only tooling/dependency directories are excluded; new documentation is checked.
export function validateBundle(bundleRoot) {
  const root = resolve(bundleRoot)
  const errors = []
  const files = new Map()
  const directories = new Set()
  const report = (path, message) => errors.push(`${relative(root, path)}: ${message}`)

  function walk(directory) {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      if (entry.name === 'node_modules' ||
          (directory === root && ['.vitepress', '.git'].includes(entry.name))) continue
      const path = join(directory, entry.name)
      if (entry.isDirectory()) walk(path)
      else if (entry.name.endsWith('.md')) {
        files.set(path, readFileSync(path, 'utf8'))
        for (let parent = directory; ; parent = dirname(parent)) {
          directories.add(parent)
          if (parent === root) break
        }
      }
    }
  }
  walk(root)
  directories.add(root)

  for (const [path, text] of files) {
    const name = basename(path)
    const reserved = ['index.md', 'log.md'].includes(name)
    const match = text.match(/^---\r?\n([\s\S]*?)\r?\n---(?:\r?\n|$)/)
    let metadata
    if (match) {
      const document = parseDocument(match[1], { uniqueKeys: true })
      if (document.errors.length) {
        report(path, `invalid YAML: ${document.errors.map(e => e.message).join('; ')}`)
        continue
      }
      try { metadata = document.toJS({ maxAliasCount: 100 }) }
      catch (error) { report(path, `invalid YAML: ${error.message}`); continue }
      if (!mapping(metadata)) { report(path, 'frontmatter must be a YAML mapping'); continue }
    } else if (!reserved || text.startsWith('---')) {
      report(path, 'missing or unterminated YAML frontmatter')
      continue
    }

    const body = match ? text.slice(match[0].length) : text
    if (reserved) {
      if (name === 'index.md' && path === join(root, 'index.md')) {
        if (!metadata || metadata.okf_version !== '0.2' || Object.keys(metadata).length !== 1) {
          report(path, 'root index frontmatter must contain only okf_version: "0.2"')
        }
      } else if (metadata) report(path, 'reserved file must not contain frontmatter')
      if (!/^# .+/m.test(body)) report(path, 'reserved file needs a Markdown heading')
      if (name === 'log.md') {
        for (const heading of body.matchAll(/^## (.+)$/gm)) {
          const date = heading[1]
          if (!/^\d{4}-\d{2}-\d{2}$/.test(date) || Number.isNaN(Date.parse(date)) ||
              new Date(date).toISOString().slice(0, 10) !== date) {
            report(path, `log date heading must be YYYY-MM-DD: ${date}`)
          }
        }
      }
      continue
    }

    for (const key of ['type', 'title', 'description']) {
      if (!nonempty(metadata[key])) report(path, `${key} must be a non-empty string`)
    }
    if (!Array.isArray(metadata.tags) || !metadata.tags.length || !metadata.tags.every(nonempty)) {
      report(path, 'tags must be a non-empty YAML list of non-empty strings')
    }
    if ('status' in metadata && !['draft', 'stable', 'deprecated'].includes(metadata.status)) {
      report(path, 'status must be draft, stable, or deprecated; use workflow_status for plan/approval state')
    }
  }

  for (const directory of directories) {
    const indexPath = join(directory, 'index.md')
    if (!files.has(indexPath)) { report(indexPath, 'missing directory index'); continue }
    const body = files.get(indexPath).replace(/^---\r?\n[\s\S]*?\r?\n---(?:\r?\n|$)/, '')
    const listed = new Set()
    // The authoring profile uses one ordinary relative Markdown link per list
    // entry, followed by a description (which may continue on the next line).
    for (const entry of body.matchAll(/^\s*[-*] \[(?:\\.|[^\]\\])+\]\(([^)\s]+)\)([^\n]*(?:\n[ \t]+[^\n]+)*)/gm)) {
      const href = entry[1].split('#')[0]
      if (!href || /^[a-z][a-z\d+.-]*:|^\//i.test(href)) continue
      let target
      try { target = resolve(directory, decodeURIComponent(href)) }
      catch { report(indexPath, `invalid index link: ${href}`); continue }
      if (existsSync(target) && statSync(target).isDirectory()) target = join(target, 'index.md')
      if (!files.has(target)) report(indexPath, `index link does not resolve to a bundle document: ${href}`)
      listed.add(target)
      if (!entry[2].replace(/^\s*[—–-]\s*/, '').trim()) report(indexPath, `index entry needs a description: ${href}`)
    }
    for (const path of files.keys()) {
      if (dirname(path) === directory && path !== indexPath && !listed.has(path)) {
        report(path, `missing index entry in ${relative(root, indexPath)}`)
      }
    }
    for (const child of directories) {
      if (child !== directory && dirname(child) === directory && !listed.has(join(child, 'index.md'))) {
        report(indexPath, `missing index entry for ${relative(directory, child)}/`)
      }
    }
  }
  return errors
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const root = process.argv[2] || fileURLToPath(new URL('..', import.meta.url))
  const errors = validateBundle(root)
  if (errors.length) {
    console.error(`[OKF] ${errors.length} documentation error(s):\n${errors.join('\n')}`)
    process.exitCode = 1
  } else console.log('[OKF] Documentation metadata and indexes passed')
}
