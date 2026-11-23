#!/usr/bin/env node
import {
  existsSync,
  lstatSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  readlinkSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { dirname, isAbsolute, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const MANIFEST_NAME = '.orkworks-repo-skills.json';
const TARGET_DIRS = ['.claude/skills', '.agents/skills', '.github/skills'];
const DEFAULT_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');

function parseArgs(argv) {
  let root = DEFAULT_ROOT;
  let check = false;

  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === '--check') {
      check = true;
    } else if (arg === '--root') {
      const value = argv[i + 1];
      if (!value || value.startsWith('--')) {
        console.error('usage: node scripts/sync-repo-skills.mjs [--check] [--root <dir>]');
        process.exit(2);
      }
      root = resolve(value);
      i += 1;
    } else {
      console.error(`usage: node scripts/sync-repo-skills.mjs [--check] [--root <dir>] (unexpected argument: ${arg})`);
      process.exit(2);
    }
  }

  return { root, check };
}

function collectFiles(dirPath, prefix = '') {
  const files = new Map();
  const entries = readdirSync(dirPath, { withFileTypes: true }).sort((a, b) =>
    a.name.localeCompare(b.name),
  );

  for (const entry of entries) {
    const fullPath = join(dirPath, entry.name);
    const relPath = prefix ? `${prefix}/${entry.name}` : entry.name;
    const stat = lstatSync(fullPath);

    if (stat.isSymbolicLink()) {
      throw new Error(`Refusing symlink inside skill tree: ${fullPath}`);
    }
    if (stat.isDirectory()) {
      for (const [childRel, childFull] of collectFiles(fullPath, relPath)) {
        files.set(childRel, childFull);
      }
    } else if (stat.isFile()) {
      files.set(relPath, fullPath);
    } else {
      throw new Error(`Unsupported entry inside skill tree: ${fullPath}`);
    }
  }

  return files;
}

function discoverSourceSkills(root) {
  const skillsRoot = join(root, 'skills');
  if (!existsSync(skillsRoot)) {
    throw new Error(`Canonical skills directory not found: ${skillsRoot}`);
  }
  const rootStat = lstatSync(skillsRoot);
  if (rootStat.isSymbolicLink() || !rootStat.isDirectory()) {
    throw new Error(`Canonical skills path must be a real directory: ${skillsRoot}`);
  }

  const skills = new Map();
  const entries = readdirSync(skillsRoot, { withFileTypes: true }).sort((a, b) =>
    a.name.localeCompare(b.name),
  );

  for (const entry of entries) {
    if (entry.name.startsWith('.')) continue;
    const skillDir = join(skillsRoot, entry.name);
    const stat = lstatSync(skillDir);
    if (stat.isSymbolicLink()) {
      throw new Error(`Refusing symlink in canonical skills/: ${skillDir}`);
    }
    if (!stat.isDirectory()) continue;

    const skillDoc = join(skillDir, 'SKILL.md');
    if (!existsSync(skillDoc)) continue;
    const files = collectFiles(skillDir);
    skills.set(entry.name, { dir: skillDir, files });
  }

  return skills;
}

function readManifest(targetDir) {
  const manifestPath = join(targetDir, MANIFEST_NAME);
  if (!existsSync(manifestPath)) {
    return { present: false, skills: new Set() };
  }
  const stat = lstatSync(manifestPath);
  if (stat.isSymbolicLink() || !stat.isFile()) {
    throw new Error(`Managed skill manifest must be a regular file: ${manifestPath}`);
  }
  let parsed;
  try {
    parsed = JSON.parse(readFileSync(manifestPath, 'utf8'));
  } catch (error) {
    throw new Error(`Invalid JSON in ${manifestPath}: ${error.message}`);
  }
  if (
    !parsed ||
    parsed.version !== 1 ||
    !Array.isArray(parsed.skills) ||
    !parsed.skills.every(
      (name) => typeof name === 'string' && name.length > 0 && !name.includes('/') && !name.includes('\\') && name !== '.' && name !== '..',
    )
  ) {
    throw new Error(`Invalid managed skill manifest schema: ${manifestPath}`);
  }
  return { present: true, skills: new Set(parsed.skills) };
}

function isLegacyCanonicalSymlink(destPath, sourceDir) {
  const stat = lstatSync(destPath);
  if (!stat.isSymbolicLink()) return false;
  const linkTarget = readlinkSync(destPath);
  return resolve(dirname(destPath), linkTarget) === resolve(sourceDir);
}

function rewriteLinkDestination(rawDest, sourceDir, srcFull, dstFull) {
  const angleWrapped = rawDest.startsWith('<') && rawDest.endsWith('>');
  const inner = angleWrapped ? rawDest.slice(1, -1) : rawDest;
  if (!inner || inner.startsWith('#') || inner.startsWith('/') || /^[a-zA-Z][a-zA-Z0-9+.-]*:/.test(inner)) {
    return rawDest;
  }

  const split = /^([^?#]*)(.*)$/.exec(inner);
  const targetPath = split[1];
  const suffix = split[2];
  if (!targetPath) return rawDest;

  const resolvedTarget = resolve(dirname(srcFull), targetPath);
  const relToSkill = relative(sourceDir, resolvedTarget);
  if (relToSkill === '' || (!relToSkill.startsWith('..') && !isAbsolute(relToSkill))) {
    return rawDest;
  }

  const rewrittenPath = relative(dirname(dstFull), resolvedTarget).split(sep).join('/');
  const rewritten = `${rewrittenPath}${suffix}`;
  return angleWrapped ? `<${rewritten}>` : rewritten;
}

function rewriteInlineLinks(segment, sourceDir, srcFull, dstFull) {
  return segment.replace(/(!?\[[^\]]*\]\()(<[^>\n]+>|[^\s)\n]+)([^)\n]*\))/g, (_m, open, dest, tail) => {
    const rewritten = rewriteLinkDestination(dest, sourceDir, srcFull, dstFull);
    return `${open}${rewritten}${tail}`;
  });
}

function rewriteLineOutsideInlineCode(line, sourceDir, srcFull, dstFull) {
  let out = '';
  let i = 0;
  let proseStart = 0;

  while (i < line.length) {
    if (line[i] !== '`') {
      i += 1;
      continue;
    }

    let ticks = 0;
    while (i + ticks < line.length && line[i + ticks] === '`') ticks += 1;
    const fence = '`'.repeat(ticks);
    const closeIdx = line.indexOf(fence, i + ticks);
    if (closeIdx === -1) {
      i += ticks;
      continue;
    }

    out += rewriteInlineLinks(line.slice(proseStart, i), sourceDir, srcFull, dstFull);
    out += line.slice(i, closeIdx + ticks);
    i = closeIdx + ticks;
    proseStart = i;
  }

  out += rewriteInlineLinks(line.slice(proseStart), sourceDir, srcFull, dstFull);
  return out;
}

function rewriteMarkdownLinks(content, sourceDir, srcFull, dstFull) {
  const lines = content.split(/(\r?\n)/);
  let activeFenceChar = null;
  let activeFenceLen = 0;

  for (let idx = 0; idx < lines.length; idx += 2) {
    const line = lines[idx];
    const fenceMatch = /^ {0,3}(`{3,}|~{3,})/.exec(line);
    if (fenceMatch) {
      const char = fenceMatch[1][0];
      const len = fenceMatch[1].length;
      if (!activeFenceChar) {
        activeFenceChar = char;
        activeFenceLen = len;
      } else if (char === activeFenceChar && len >= activeFenceLen && /^ {0,3}(`{3,}|~{3,})[ \t]*$/.test(line)) {
        activeFenceChar = null;
        activeFenceLen = 0;
      }
      continue;
    }
    if (activeFenceChar) continue;

    const refMatch = /^( {0,3}\[[^\]]+\]:[ \t]*)(<[^>\n]+>|[^\s]+)(.*)$/.exec(line);
    if (refMatch) {
      const rewritten = rewriteLinkDestination(refMatch[2], sourceDir, srcFull, dstFull);
      lines[idx] = `${refMatch[1]}${rewritten}${refMatch[3]}`;
      continue;
    }

    lines[idx] = rewriteLineOutsideInlineCode(line, sourceDir, srcFull, dstFull);
  }

  return lines.join('');
}

function expectedSkillFileBytes(sourceDir, destDir, relFile, srcFull) {
  const raw = readFileSync(srcFull);
  if (!relFile.endsWith('.md')) return raw;
  const dstFull = join(destDir, relFile);
  const rewritten = rewriteMarkdownLinks(raw.toString('utf8'), sourceDir, srcFull, dstFull);
  return Buffer.from(rewritten, 'utf8');
}

function inspectTarget(root, relativeTarget, sourceSkills) {
  const targetDir = join(root, relativeTarget);
  const exists = existsSync(targetDir);
  if (exists && (lstatSync(targetDir).isSymbolicLink() || !lstatSync(targetDir).isDirectory())) {
    throw new Error(`Target skills path must be a real directory: ${targetDir}`);
  }

  const manifest = exists ? readManifest(targetDir) : { present: false, skills: new Set() };
  const issues = [];
  const collisions = [];
  const stale = [];

  for (const managedName of manifest.skills) {
    if (!sourceSkills.has(managedName)) {
      stale.push(managedName);
      issues.push(`${relativeTarget}/${managedName}: stale managed skill (removed from skills/)`);
    }
  }

  if (!exists) {
    issues.push(`${relativeTarget}: target directory does not exist`);
    return { targetDir, relativeTarget, manifest, stale, collisions, issues };
  }

  if (!manifest.present) {
    issues.push(`${relativeTarget}/${MANIFEST_NAME}: manifest missing`);
  }

  for (const [name, source] of sourceSkills) {
    const destPath = join(targetDir, name);
    let stat;
    try {
      stat = lstatSync(destPath);
    } catch {
      issues.push(`${relativeTarget}/${name}: missing`);
      continue;
    }

    if (stat.isSymbolicLink()) {
      if (isLegacyCanonicalSymlink(destPath, source.dir)) {
        issues.push(`${relativeTarget}/${name}: legacy symlink (run sync to replace with real copy)`);
      } else {
        const msg = `${relativeTarget}/${name}: unmanaged symlink collision (points outside skills/${name})`;
        collisions.push(msg);
        issues.push(msg);
      }
      continue;
    }

    if (!manifest.skills.has(name)) {
      const msg = `${relativeTarget}/${name}: unmanaged collision (existing skill is not managed by sync-repo-skills)`;
      collisions.push(msg);
      issues.push(msg);
      continue;
    }

    if (!stat.isDirectory()) {
      const msg = `${relativeTarget}/${name}: unmanaged collision (expected directory)`;
      collisions.push(msg);
      issues.push(msg);
      continue;
    }

    let destFiles;
    try {
      destFiles = collectFiles(destPath);
    } catch (error) {
      issues.push(`${relativeTarget}/${name}: ${error.message}`);
      continue;
    }

    const sourceKeys = [...source.files.keys()];
    const destKeys = [...destFiles.keys()];
    if (
      sourceKeys.length !== destKeys.length ||
      sourceKeys.some((key, index) => key !== destKeys[index])
    ) {
      issues.push(`${relativeTarget}/${name}: file list differs from skills/${name}`);
      continue;
    }

    for (const relFile of sourceKeys) {
      const expectedBuf = expectedSkillFileBytes(source.dir, destPath, relFile, source.files.get(relFile));
      const dstBuf = readFileSync(destFiles.get(relFile));
      if (!expectedBuf.equals(dstBuf)) {
        issues.push(`${relativeTarget}/${name}/${relFile}: content drifted from skills/${name}/${relFile}`);
      }
    }
  }

  return { targetDir, relativeTarget, manifest, stale, collisions, issues };
}

function copySkillFiles(source, destDir) {
  mkdirSync(destDir, { recursive: true });
  for (const [relFile, srcFull] of source.files) {
    const dstFull = join(destDir, relFile);
    mkdirSync(dirname(dstFull), { recursive: true });
    const bytes = expectedSkillFileBytes(source.dir, destDir, relFile, srcFull);
    writeFileSync(dstFull, bytes);
  }
}

function main() {
  const { root, check } = parseArgs(process.argv.slice(2));
  const sourceSkills = discoverSourceSkills(root);
  const plans = TARGET_DIRS.map((rel) => inspectTarget(root, rel, sourceSkills));

  if (check) {
    const allIssues = plans.flatMap((plan) => plan.issues);
    if (allIssues.length > 0) {
      for (const issue of allIssues) {
        console.error(`[sync-repo-skills] ${issue}`);
      }
      console.error('[sync-repo-skills] Run `node scripts/sync-repo-skills.mjs` to synchronize repo-owned skills.');
      process.exit(1);
    }
    console.log(`[sync-repo-skills] OK (${sourceSkills.size} repo skills verified across ${TARGET_DIRS.length} targets)`);
    return;
  }

  // Fail closed across all targets before writing anything.
  const allCollisions = plans.flatMap((plan) => plan.collisions);
  if (allCollisions.length > 0) {
    for (const collision of allCollisions) {
      console.error(`[sync-repo-skills] ${collision}`);
    }
    console.error('[sync-repo-skills] Aborted without modifying any target directory.');
    process.exit(1);
  }

  const managedNames = [...sourceSkills.keys()].sort((a, b) => a.localeCompare(b));

  for (const plan of plans) {
    mkdirSync(plan.targetDir, { recursive: true });

    for (const staleName of plan.stale) {
      const stalePath = join(plan.targetDir, staleName);
      if (existsSync(stalePath)) {
        rmSync(stalePath, { recursive: true, force: true });
      }
    }

    for (const [name, source] of sourceSkills) {
      const destPath = join(plan.targetDir, name);
      rmSync(destPath, { recursive: true, force: true });
      copySkillFiles(source, destPath);
    }

    writeFileSync(
      join(plan.targetDir, MANIFEST_NAME),
      `${JSON.stringify({ version: 1, skills: managedNames }, null, 2)}\n`,
      'utf8',
    );
  }

  console.log(`[sync-repo-skills] Synced ${managedNames.length} repo skills to ${TARGET_DIRS.join(', ')}`);
}

try {
  main();
} catch (error) {
  console.error(`[sync-repo-skills] ${error.message}`);
  process.exit(1);
}
