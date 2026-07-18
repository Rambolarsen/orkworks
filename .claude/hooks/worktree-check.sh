#!/usr/bin/env bash
# Runs at end of each Claude Code session.
# Reports the state of every worktree/branch in the repo (not just this
# session's own) so cleanup decisions aren't lost when multiple agents work
# in parallel across worktrees — see AGENTS.md "Parallel work".

command -v git >/dev/null 2>&1 || exit 0
git rev-parse --git-common-dir >/dev/null 2>&1 || exit 0

MAIN_BRANCH="main"
STALE_SECS=$((7 * 86400))
NOW=$(date +%s)

flags=()

check_worktree() {
  local path="$1" branch="$2"
  [ -z "$branch" ] && return
  [ "$branch" = "$MAIN_BRANCH" ] && return
  git show-ref --verify --quiet "refs/heads/$branch" || return

  if git merge-base --is-ancestor "$branch" "$MAIN_BRANCH" 2>/dev/null; then
    flags+=("$branch ($path): merged into $MAIN_BRANCH — remove worktree + delete branch")
    return
  fi

  local last_ts age has_pr=1
  last_ts=$(git log -1 --format=%ct "$branch" 2>/dev/null) || return
  age=$((NOW - last_ts))

  if command -v gh >/dev/null 2>&1; then
    gh pr list --state open --head "$branch" --json number 2>/dev/null | grep -q '"number"' && has_pr=0
  fi

  if [ "$age" -gt "$STALE_SECS" ] && [ "$has_pr" -ne 0 ]; then
    local days=$((age / 86400))
    flags+=("$branch ($path): ${days}d stale, no open PR — decide: rebase & progress, or close")
  fi
}

path="" branch=""
while IFS= read -r line; do
  case "$line" in
    "worktree "*) path="${line#worktree }" ;;
    "branch "*) branch="${line#branch refs/heads/}"; check_worktree "$path" "$branch" ;;
    "") path="" branch="" ;;
  esac
done < <(git worktree list --porcelain)

[ ${#flags[@]} -eq 0 ] && exit 0

# Informational only — never "block". This scans repo-wide state that has
# nothing to do with what this particular session did, so unlike doc-check.sh
# it can't self-resolve by fixing your own diff. Blocking here would trap
# every unrelated session in a loop until someone else's branch gets cleaned
# up. Surface it once and let the session end; the report just repeats next
# time until the flagged branches are actually resolved.
printf '\n[worktree-check] Repo-wide worktrees needing a decision (not necessarily yours — only act on branches you own):\n'
for f in "${flags[@]}"; do
  printf '  • %s\n' "$f"
done
printf '\n'
