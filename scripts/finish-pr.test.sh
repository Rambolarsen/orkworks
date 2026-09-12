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

if [ "${1:-}" = 'pr' ] && [ "${2:-}" = 'list' ]; then
  if [ "${GH_MODE:?}" = 'reused' ]; then
    printf '999\n'
  else
    if [ "${GH_MODE:?}" = 'race' ]; then
      git -C "${GH_REPO:?}" update-ref "refs/heads/${GH_HEAD_REF_NAME:?}" "${GH_RACE_OID:?}"
    fi
    :
  fi
  exit 0
fi

case "${GH_MODE:?}" in
  open)
    printf '{"number":435,"state":"OPEN","baseRefName":"main","headRefName":"%s","headRefOid":"%s","headRepository":{"nameWithOwner":"Rambolarsen/orkworks"},"isCrossRepository":false,"mergeCommit":null}\n' "${GH_HEAD_REF_NAME:-feature}" "${GH_HEAD_REF_OID:?}"
    ;;
  merged)
    printf '{"number":435,"state":"MERGED","baseRefName":"main","headRefName":"%s","headRefOid":"%s","headRepository":{"nameWithOwner":"Rambolarsen/orkworks"},"isCrossRepository":false,"mergeCommit":{"oid":"deadbeef"}}\n' "${GH_HEAD_REF_NAME:-feature}" "${GH_HEAD_REF_OID:?}"
    ;;
  dirty)
    printf '{"number":436,"state":"MERGED","baseRefName":"main","headRefName":"%s","headRefOid":"%s","headRepository":{"nameWithOwner":"Rambolarsen/orkworks"},"isCrossRepository":false,"mergeCommit":{"oid":"deadbeef"}}\n' "${GH_HEAD_REF_NAME:-dirty-feature}" "${GH_HEAD_REF_OID:?}"
    ;;
  foreign)
    printf '{"number":438,"state":"MERGED","baseRefName":"main","headRefName":"%s","headRefOid":"%s","headRepository":{"nameWithOwner":"someone-else/other-repo"},"isCrossRepository":false,"mergeCommit":{"oid":"deadbeef"}}\n' "${GH_HEAD_REF_NAME:-dirty-feature}" "${GH_HEAD_REF_OID:?}"
    ;;
  reverse-foreign)
    printf '{"number":439,"state":"MERGED","baseRefName":"main","headRefName":"%s","headRefOid":"%s","headRepository":{"nameWithOwner":"Rambolarsen/orkworks"},"isCrossRepository":true,"mergeCommit":{"oid":"deadbeef"}}\n' "${GH_HEAD_REF_NAME:-feature}" "${GH_HEAD_REF_OID:?}"
    ;;
  missing-oid)
    printf '{"number":440,"state":"MERGED","baseRefName":"main","headRefName":"%s","headRepository":{"nameWithOwner":"Rambolarsen/orkworks"},"isCrossRepository":false,"mergeCommit":{"oid":"deadbeef"}}\n' "${GH_HEAD_REF_NAME:-feature}"
    ;;
  reused)
    printf '{"number":441,"state":"MERGED","baseRefName":"main","headRefName":"%s","headRefOid":"%s","headRepository":{"nameWithOwner":"Rambolarsen/orkworks"},"isCrossRepository":false,"mergeCommit":{"oid":"deadbeef"}}\n' "${GH_HEAD_REF_NAME:-feature}" "${GH_HEAD_REF_OID:?}"
    ;;
  race)
    printf '{"number":442,"state":"MERGED","baseRefName":"main","headRefName":"%s","headRefOid":"%s","headRepository":{"nameWithOwner":"Rambolarsen/orkworks"},"isCrossRepository":false,"mergeCommit":{"oid":"deadbeef"}}\n' "${GH_HEAD_REF_NAME:-feature}" "${GH_HEAD_REF_OID:?}"
    ;;
  no-origin)
    printf '{"number":443,"state":"MERGED","baseRefName":"main","headRefName":"%s","headRefOid":"%s","headRepository":{"nameWithOwner":"Rambolarsen/orkworks"},"isCrossRepository":false,"mergeCommit":{"oid":"deadbeef"}}\n' "${GH_HEAD_REF_NAME:-feature}" "${GH_HEAD_REF_OID:?}"
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

if (cd "$repo" && PATH="$bin:$PATH" GH_MODE=reverse-foreign GH_HEAD_REF_NAME=dirty-feature GH_HEAD_REF_OID="$dirty_sha" "$helper" 439) > "$fixture/reverse-foreign.out" 2>&1; then
  echo 'finish-pr unexpectedly accepted a PR whose base repository is foreign' >&2
  exit 1
fi
test -d "$worktree"
git -C "$repo" show-ref --verify --quiet refs/heads/dirty-feature
grep -Fq 'cross-repository' "$fixture/reverse-foreign.out"

if (cd "$repo" && PATH="$bin:$PATH" GH_MODE=missing-oid GH_HEAD_REF_NAME=dirty-feature "$helper" 440) > "$fixture/missing-oid.out" 2>&1; then
  echo 'finish-pr unexpectedly accepted a PR without a head OID' >&2
  exit 1
fi
test -d "$worktree"
git -C "$repo" show-ref --verify --quiet refs/heads/dirty-feature
grep -Fq 'missing the head commit OID' "$fixture/missing-oid.out"

git -C "$repo" switch -q main
git -C "$repo" switch -q -c reused-feature
printf 'reused\n' > "$repo/reused.txt"
git -C "$repo" add reused.txt
git -C "$repo" commit -qm reused-feature
reused_sha="$(git -C "$repo" rev-parse HEAD)"
git -C "$repo" switch -q main
git -C "$repo" worktree add -q "$fixture/reused-worktree" reused-feature
if (cd "$repo" && PATH="$bin:$PATH" GH_MODE=reused GH_HEAD_REF_NAME=reused-feature GH_HEAD_REF_OID="$reused_sha" "$helper" 441) > "$fixture/reused.out" 2>&1; then
  echo 'finish-pr unexpectedly removed a branch used by another open PR' >&2
  exit 1
fi
test -d "$fixture/reused-worktree"
git -C "$repo" show-ref --verify --quiet refs/heads/reused-feature
grep -Fq 'still used by open PR #999' "$fixture/reused.out"

git -C "$repo" switch -q main
git -C "$repo" switch -q -c race-feature
printf 'race\n' > "$repo/race.txt"
git -C "$repo" add race.txt
git -C "$repo" commit -qm race-feature
race_sha="$(git -C "$repo" rev-parse HEAD)"
git -C "$repo" switch -q -c race-builder
git -C "$repo" commit --allow-empty -qm race-update
race_update_sha="$(git -C "$repo" rev-parse HEAD)"
git -C "$repo" switch -q main
git -C "$repo" branch -D -q race-builder
git -C "$repo" worktree add -q "$fixture/race-worktree" race-feature

if (cd "$repo" && PATH="$bin:$PATH" GH_MODE=race GH_REPO="$repo" GH_HEAD_REF_NAME=race-feature GH_HEAD_REF_OID="$race_sha" GH_RACE_OID="$race_update_sha" "$helper" 442) > "$fixture/race.out" 2>&1; then
  echo 'finish-pr unexpectedly deleted a branch changed during cleanup' >&2
  exit 1
fi
test ! -e "$fixture/race-worktree"
test "$(git -C "$repo" rev-parse refs/heads/race-feature)" = "$race_update_sha"
grep -Fq 'changed during cleanup' "$fixture/race.out"

git -C "$repo" switch -q -c current-feature
printf 'current\n' > "$repo/current.txt"
git -C "$repo" add current.txt
git -C "$repo" commit -qm current-feature
current_sha="$(git -C "$repo" rev-parse HEAD)"
if (cd "$repo" && PATH="$bin:$PATH" GH_MODE=merged GH_HEAD_REF_NAME=current-feature GH_HEAD_REF_OID="$current_sha" "$helper" 443) > "$fixture/current.out" 2>&1; then
  echo 'finish-pr unexpectedly removed the current checkout branch' >&2
  exit 1
fi
git -C "$repo" show-ref --verify --quiet refs/heads/current-feature
grep -Fq 'checked out in the current worktree' "$fixture/current.out"
git -C "$repo" switch -q main

git -C "$repo" switch -q -c multi-feature
printf 'multiple\n' > "$repo/multiple.txt"
git -C "$repo" add multiple.txt
git -C "$repo" commit -qm multi-feature
multi_sha="$(git -C "$repo" rev-parse HEAD)"
git -C "$repo" switch -q main
git -C "$repo" worktree add -q "$fixture/multi-one" multi-feature
git -C "$repo" worktree add --force -q "$fixture/multi-two" multi-feature
if (cd "$repo" && PATH="$bin:$PATH" GH_MODE=merged GH_HEAD_REF_NAME=multi-feature GH_HEAD_REF_OID="$multi_sha" "$helper" 444) > "$fixture/multiple.out" 2>&1; then
  echo 'finish-pr unexpectedly accepted multiple matching worktrees' >&2
  exit 1
fi
test -d "$fixture/multi-one"
test -d "$fixture/multi-two"
git -C "$repo" show-ref --verify --quiet refs/heads/multi-feature
grep -Fq 'multiple local worktrees' "$fixture/multiple.out"

git -C "$repo" remote remove origin
git -C "$repo" switch -q -c no-origin-feature
printf 'no origin\n' > "$repo/no-origin.txt"
git -C "$repo" add no-origin.txt
git -C "$repo" commit -qm no-origin-feature
no_origin_sha="$(git -C "$repo" rev-parse HEAD)"
git -C "$repo" switch -q main
git -C "$repo" worktree add -q "$fixture/no-origin-worktree" no-origin-feature
(cd "$repo" && PATH="$bin:$PATH" GH_MODE=no-origin GH_HEAD_REF_NAME=no-origin-feature GH_HEAD_REF_OID="$no_origin_sha" "$helper" 445) > "$fixture/no-origin.out"
test ! -e "$fixture/no-origin-worktree"
if git -C "$repo" show-ref --verify --quiet refs/heads/no-origin-feature; then
  echo 'finish-pr left the no-origin merged branch behind' >&2
  exit 1
fi

echo 'finish-pr fixtures passed'
