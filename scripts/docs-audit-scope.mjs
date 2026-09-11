export function validateGuideChanges(nameStatus, untracked) {
  if (untracked.trim()) throw new Error('Audit produced untracked files');
  const changes = nameStatus.split('\n').filter(Boolean);
  if (changes.length > 3 || changes.some(line => !/^M\tdocs\/user\/[a-z-]+\.md$/.test(line))) {
    throw new Error('Audit exceeded its scope: only three existing user guides may change');
  }
  return changes.length;
}
