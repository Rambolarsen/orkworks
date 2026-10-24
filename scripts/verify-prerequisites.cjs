const { existsSync, readFileSync } = require('node:fs');
const { createRequire } = require('node:module');
const { join, resolve } = require('node:path');

const root = resolve(__dirname, '..');
const fail = message => {
  console.error(`[verify] ${message}`);
  process.exit(1);
};

const expected = readFileSync(join(root, '.nvmrc'), 'utf8').trim();
if (!/^\d+$/.test(expected)) fail('Expected a Node major version in .nvmrc.');
if (process.versions.node.split('.')[0] !== expected) {
  fail(`Node ${expected} is required by .nvmrc; current Node is ${process.versions.node}. Select the pinned Node version before verification.`);
}

for (const surface of ['apps/desktop', 'docs']) {
  const directory = join(root, surface);
  const manifest = JSON.parse(readFileSync(join(directory, 'package.json'), 'utf8'));
  const dependencies = { ...manifest.dependencies, ...manifest.devDependencies };
  const missing = Object.keys(dependencies).filter(name =>
    !existsSync(join(directory, 'node_modules', name, 'package.json')));
  if (missing.length) {
    fail(`Missing ${surface} dependencies: ${missing.join(', ')}. Run pnpm --dir ${surface} install --frozen-lockfile with Node from .nvmrc.`);
  }
}

try {
  const desktopRequire = createRequire(join(root, 'apps/desktop/package.json'));
  desktopRequire('fs-ext');
} catch (error) {
  fail(`fs-ext cannot load in host Node ${process.versions.node} (${error.code || 'load error'}). After selecting Node from .nvmrc, run pnpm --dir apps/desktop rebuild fs-ext. No dependencies were rebuilt by this check.`);
}
