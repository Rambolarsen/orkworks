import { posix } from 'node:path'

// OKF source indexes stay portable; the public site sends repository-only
// entries to GitHub instead of exposing links to pages excluded from the site.
export function repositoryOnlyLinks(md) {
  const render = md.renderer.rules.link_open
  md.renderer.rules.link_open = (tokens, index, options, env, self) => {
    const token = tokens[index]
    const href = token.attrGet('href')
    if (href && env.relativePath && !/^(?:[a-z][a-z\d+.-]*:|\/\/|#)/i.test(href)) {
      const [path, fragment] = href.split('#')
      const target = posix.resolve('/', posix.dirname(env.relativePath), path).slice(1)
      if (target === 'docs/adr/template.md' || target.startsWith('docs/superpowers/plans/')) {
        token.attrSet('href', `https://github.com/Rambolarsen/orkworks/blob/main/${target}${fragment ? `#${fragment}` : ''}`)
      }
    }
    return render(tokens, index, options, env, self)
  }
}
