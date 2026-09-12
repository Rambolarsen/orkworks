#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
helper="$repo_root/scripts/finish-pr.sh"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/orkworks-finish-pr.XXXXXX")"
trap 'rm -rf "$fixture"' EXIT

repo="$fixture/repo"
origin="$fixture/origin.git"
worktree="$fixture/worktree"
bin="$fixture/bin"
mkdir -p "$bin"

git init -q "$repo"
git -C "$repo" config user.email test@example.invalid
git -C "$repo" config user.name "Finish PR test"
printf 'baseline\n' > "$repo/README.md"
git -C "$repo" add README.md
git -C "$repo" commit -qm baseline
git -C "$repo" branch -M main
git init --bare -q "$origin"
git -C "$repo" remote add origin "$origin"
git -C "$repo" push -q -u origin main

git -C "$repo" switch -q -c feature
printf 'feature\n' > "$repo/feature.txt"
git -C "$repo" add feature.txt
git -C "$repo" commit -qm feature
feature_sha="$(git -C "$repo" rev-parse HEAD)"
git -C "$repo" push -q -u origin feature
git -C "$repo" switch -q main
git -C "$repo" worktree add -q "$worktree" feature

cat > "$bin/gh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

if [ "${1:-}" = 'repo' ]; then
  printf '%s\n' 'Rambolarsen/orkworks'
  exit 0
fi

case "${GH_MODE:?}" in
  open)
    printf '{"state":"OPEN","baseRefName":"main","headRefName":"feature","headRefOid":"%s","headRepository":{"nameWithOwner":"Rambolarsen/orkworks"},"mergeCommit":null}\n' "${GH_HEAD_REF_OID:?}"
    ;;
  merged)
    printf '{"state":"MERGED","baseRefName":"main","headRefName":"feature","headRefOid":"%s","headRepository":{"nameWithOwner":"Rambolarsen/orkworks"},"mergeCommit":{"oid":"deadbeef"}}\n' "${GH_HEAD_REF_OID:?}"
    ;;
  dirty)
    printf '{"state":"MERGED","baseRefName":"main","headRefName":"dirty-feature","headRefOid":"%s","headRepository":{"nameWithOwner":"Rambolarsen/orkworks"},"mergeCommit":{"oid":"deadbeef"}}\n' "${GH_HEAD_REF_OID:?}"
    ;;
  foreign)
    printf '{"state":"MERGED","baseRefName":"main","headRefName":"dirty-feature","headRefOid":"%s","headRepository":{"nameWithOwner":"someone-else/other-repo"},"mergeCommit":{"oid":"deadbeef"}}\n' "${GH_HEAD_REF_OID:?}"
    ;;
  *)
    echo "unknown GH_MODE" >&2
    exit 1
    ;;
esac
EOF
chmod +x "$bin/gh"

if (cd "$repo" && PATH="$bin:$PATH" GH_MODE=open GH_HEAD_REF_OID="$feature_sha" "$helper" '#435') > "$fixture/open.out" 2>&1; then
  echo 'finish-pr unexpectedly accepted an open PR' >&2
  exit 1
fi
test -d "$worktree"
git -C "$repo" show-ref --verify --quiet refs/heads/feature

(cd "$repo" && PATH="$bin:$PATH" GH_MODE=merged GH_HEAD_REF_OID="$feature_sha" "$helper" '#435') > "$fixture/merged.out"
test ! -e "$worktree"
if git -C "$repo" show-ref --verify --quiet refs/heads/feature; then
  echo 'finish-pr left the merged local branch behind' >&2
  exit 1
fi
grep -Fq 'PR #435 is merged into main' "$fixture/merged.out"

git -C "$repo" switch -q -c dirty-feature
printf 'dirty\n' > "$repo/dirty.txt"
git -C "$repo" add dirty.txt
git -C "$repo" commit -qm dirty-feature
dirty_sha="$(git -C "$repo" rev-parse HEAD)"
git -C "$repo" push -q -u origin dirty-feature
git -C "$repo" switch -q main
git -C "$repo" worktree add -q "$worktree" dirty-feature
printf 'uncommitted\n' > "$worktree/uncommitted.txt"

if (cd "$repo" && PATH="$bin:$PATH" GH_MODE=dirty GH_HEAD_REF_OID="$dirty_sha" "$helper" 436) > "$fixture/dirty.out" 2>&1; then
  echo 'finish-pr unexpectedly removed a dirty worktree' >&2
  exit 1
fi
test -d "$worktree"
git -C "$repo" show-ref --verify --quiet refs/heads/dirty-feature
grep -Fq 'uncommitted changes' "$fixture/dirty.out"

if (cd "$repo" && PATH="$bin:$PATH" GH_MODE=dirty GH_HEAD_REF_OID="$feature_sha" "$helper" 437) > "$fixture/advanced.out" 2>&1; then
  echo 'finish-pr unexpectedly removed a branch that advanced after merge' >&2
  exit 1
fi
test -d "$worktree"
grep -Fq 'no longer matches PR #437' "$fixture/advanced.out"

if (cd "$repo" && PATH="$bin:$PATH" GH_MODE=foreign GH_HEAD_REF_OID="$dirty_sha" "$helper" 438) > "$fixture/foreign.out" 2>&1; then
  echo 'finish-pr unexpectedly accepted a foreign-repository PR' >&2
  exit 1
fi
test -d "$worktree"
grep -Fq 'owned by someone-else/other-repo' "$fixture/foreign.out"

echo 'finish-pr fixtures passed'
