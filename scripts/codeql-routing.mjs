import { execFileSync } from 'node:child_process';
import { appendFileSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const languages = ['actions', 'c-cpp', 'javascript-typescript', 'python', 'rust'];
const shaPattern = /^[0-9a-f]{40}$/;

function unrelated(path) {
  if (typeof path !== 'string' || !path || path.startsWith('/') || path.includes('\\') || path.split('/').some(part => part === '..' || part === '.' || !part)) return false;
  if (path.split('/').includes('fixtures')) return false;
  if (path.startsWith('apps/desktop/src/')) return /\.(?:[cm]?[jt]sx?|vue|css|scss|svg|png|jpe?g|webp|ico)$/.test(path);
  return /^(?:docs|specs|skills)\/.+\.md$/.test(path);
}

export function selectCodeqlLanguages(paths, { complete = false, forceFull = false } = {}) {
  const skipRust = !forceFull && complete === true && Array.isArray(paths) && paths.length > 0 && paths.every(unrelated);
  return {
    language: skipRust ? languages.slice(0, -1) : [...languages],
    reason: forceFull ? 'forced-full' : !complete ? 'incomplete-diff' : skipRust ? 'unrelated-ui-or-prose' : 'rust-or-unknown-input',
  };
}

export function detectCodeqlChanges(eventName, event, sourceSha, cwd = process.cwd()) {
  if (['schedule', 'workflow_dispatch'].includes(eventName)) return { paths: [], complete: true, forceFull: true };
  const pr = eventName === 'pull_request' ? event.pull_request : null;
  const base = pr ? pr.base?.sha : eventName === 'push' ? event.before : null;
  const head = pr ? pr.head?.sha : sourceSha;
  if (base === '0'.repeat(40)) return { paths: [], complete: true, forceFull: true };
  if (!shaPattern.test(base ?? '') || !shaPattern.test(head ?? '')) return { paths: [], complete: false };
  try {
    const git = (...args) => execFileSync('git', args, { cwd, encoding: 'utf8', maxBuffer: 32 * 1024 * 1024, stdio: ['ignore', 'pipe', 'pipe'] });
    const from = pr ? git('merge-base', base, head).trim() : base;
    const output = git('diff', '--name-only', '--no-renames', '-z', from, head, '--');
    return { paths: output ? output.slice(0, -1).split('\0') : [], complete: !output || output.endsWith('\0') };
  } catch {
    return { paths: [], complete: false };
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  let changes = { paths: [], complete: false };
  try {
    changes = detectCodeqlChanges(process.env.GITHUB_EVENT_NAME, JSON.parse(readFileSync(process.env.GITHUB_EVENT_PATH, 'utf8')), process.env.GITHUB_SHA);
  } catch { /* Unreadable event inventory must never skip Rust. */ }
  const result = selectCodeqlLanguages(changes.paths, changes);
  if (process.env.GITHUB_OUTPUT) appendFileSync(process.env.GITHUB_OUTPUT, `matrix=${JSON.stringify({ language: result.language })}\nreason=${result.reason}\n`);
  console.log(JSON.stringify(result));
}
