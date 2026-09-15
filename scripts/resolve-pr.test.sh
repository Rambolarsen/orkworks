#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
helper="$repo_root/scripts/resolve-pr.sh"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/orkworks-resolve-pr.XXXXXX")"
trap 'rm -rf "$fixture"' EXIT

repo="$fixture/repo"
bin="$fixture/bin"
mkdir -p "$bin"

git init -q "$repo"
git -C "$repo" config user.email test@example.invalid
git -C "$repo" config user.name "Resolve PR test"
printf 'baseline\n' > "$repo/README.md"
git -C "$repo" add README.md
git -C "$repo" commit -qm baseline
git -C "$repo" branch -M main
git -C "$repo" switch -q -c feature
git -C "$repo" commit --allow-empty -qm feature

cat > "$bin/gh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

printf '%s\n' "gh args: $*" >> "${GH_LOG:?}"

case "${1:-}" in
  pr)
    case "${2:-}" in
      view)
        ref="${3:?missing view ref}"
        if [ "${GH_MODE:?}" = 'view-miss' ] && [ "$ref" != '561' ]; then
          echo "no pull requests found for branch ${ref}" >&2
          exit 1
        fi
        printf '{"number":561,"url":"https://github.com/Rambolarsen/orkworks/pull/561","headRefName":"feature","baseRefName":"main","state":"OPEN"}\n'
        exit 0
        ;;
      list)
        case "${GH_LIST_MODE:?}" in
          one)
            printf '[{"number":561,"url":"https://github.com/Rambolarsen/orkworks/pull/561","headRefName":"feature","baseRefName":"main","state":"OPEN"}]\n'
            ;;
          many)
            printf '[{"number":561,"url":"https://github.com/Rambolarsen/orkworks/pull/561","headRefName":"feature","baseRefName":"main","state":"OPEN"},{"number":562,"url":"https://github.com/Rambolarsen/orkworks/pull/562","headRefName":"other","baseRefName":"main","state":"OPEN"}]\n'
            ;;
          none)
            printf '[]\n'
            ;;
          fail)
            echo 'gh: authentication required' >&2
            exit 4
            ;;
          *)
            echo "unknown GH_LIST_MODE" >&2
            exit 1
            ;;
        esac
        exit 0
        ;;
    esac
    ;;
esac

echo "unexpected gh invocation: $*" >&2
exit 1
EOF
chmod +x "$bin/gh"

# No argument resolves the current branch's PR.
output="$( (cd "$repo" && PATH="$bin:$PATH" GH_MODE=view-hit GH_LOG="$fixture/no-arg.log" "$helper") )"
grep -Fq 'number: 561' <<<"$output"
grep -Fq 'url: https://github.com/Rambolarsen/orkworks/pull/561' <<<"$output"
grep -Fq 'head: feature' <<<"$output"
grep -Fq 'base: main' <<<"$output"
grep -Fq 'state: OPEN' <<<"$output"
grep -Fq 'gh args: pr view' "$fixture/no-arg.log"

# Bare number and #-prefixed number both resolve directly.
(cd "$repo" && PATH="$bin:$PATH" GH_MODE=view-hit GH_LOG="$fixture/number.log" "$helper" 561) > /dev/null
(cd "$repo" && PATH="$bin:$PATH" GH_MODE=view-hit GH_LOG="$fixture/hash.log" "$helper" '#561') > /dev/null
grep -Fq 'gh args: pr view 561' "$fixture/number.log"
grep -Fq 'gh args: pr view 561' "$fixture/hash.log"

# Full URL resolves directly.
(cd "$repo" && PATH="$bin:$PATH" GH_MODE=view-hit GH_LOG="$fixture/url.log" "$helper" 'https://github.com/Rambolarsen/orkworks/pull/561') > /dev/null
grep -Fq 'gh args: pr view https://github.com/Rambolarsen/orkworks/pull/561' "$fixture/url.log"

# When the branch has no PR, resolution fails and asks the user, without searching.
if (cd "$repo" && PATH="$bin:$PATH" GH_MODE=view-miss GH_LOG="$fixture/no-pr.log" "$helper") > "$fixture/no-pr.out" 2>&1; then
  echo 'resolve-pr unexpectedly resolved a branch without a PR' >&2
  exit 1
fi
grep -Fq 'no pull requests found' "$fixture/no-pr.out"
grep -Fq 'ask the user' "$fixture/no-pr.out"
if grep -Fq 'pr list' "$fixture/no-pr.log"; then
  echo 'resolve-pr unexpectedly searched after an explicit view failure' >&2
  exit 1
fi

# A non-branch, non-number reference falls back to search (all states).
if ! (cd "$repo" && PATH="$bin:$PATH" GH_MODE=view-miss GH_LIST_MODE=one GH_LOG="$fixture/search.log" "$helper" 'fix the port bug') > "$fixture/search.out" 2>&1; then
  echo 'resolve-pr unexpectedly rejected a single search hit' >&2
  exit 1
fi
grep -Fq 'number: 561' "$fixture/search.out"
grep -Fq 'gh args: pr list --state all --search fix the port bug' "$fixture/search.log"

# A structured reference that fails direct lookup must not fall back to search.
for structured_miss_ref in 999 '#999' 'https://github.com/Rambolarsen/orkworks/pull/999'; do
  log="$fixture/structured-miss.log"
  rm -f "$log"
  if (cd "$repo" && PATH="$bin:$PATH" GH_MODE=view-miss GH_LIST_MODE=one GH_LOG="$log" "$helper" "$structured_miss_ref") > "$fixture/structured-miss.out" 2>&1; then
    echo "resolve-pr unexpectedly resolved the structured miss '$structured_miss_ref'" >&2
    exit 1
  fi
  grep -Fq 'ask the user' "$fixture/structured-miss.out"
  if grep -Fq 'pr list' "$log"; then
    echo "resolve-pr unexpectedly searched after structured lookup failure for $structured_miss_ref" >&2
    exit 1
  fi
done

# A #-prefixed branch name is looked up verbatim, not stripped to a branch name.
(cd "$repo" && PATH="$bin:$PATH" GH_MODE=view-hit GH_LOG="$fixture/hash-branch.log" "$helper" '#feature') > /dev/null
grep -Fq 'gh args: pr view #feature' "$fixture/hash-branch.log"

# A failed gh pr list surfaces its diagnostic instead of claiming zero matches.
if (cd "$repo" && PATH="$bin:$PATH" GH_MODE=view-miss GH_LIST_MODE=fail GH_LOG="$fixture/list-fail.log" "$helper" 'fix the port bug') > "$fixture/list-fail.out" 2>&1; then
  echo 'resolve-pr unexpectedly treated a failed search as zero matches' >&2
  exit 1
fi
grep -Fq 'gh: authentication required' "$fixture/list-fail.out"
if grep -Fq 'no pull requests matched' "$fixture/list-fail.out"; then
  echo 'resolve-pr masked a failed search as zero matches' >&2
  exit 1
fi

# Multiple search hits fail with an ask-the-user message.
if (cd "$repo" && PATH="$bin:$PATH" GH_MODE=view-miss GH_LIST_MODE=many GH_LOG="$fixture/many.log" "$helper" 'fix the port bug') > "$fixture/many.out" 2>&1; then
  echo 'resolve-pr unexpectedly resolved ambiguous search hits' >&2
  exit 1
fi
grep -Fq 'multiple pull requests' "$fixture/many.out"
grep -Fq 'ask the user' "$fixture/many.out"

# Zero search hits fail the same way.
if (cd "$repo" && PATH="$bin:$PATH" GH_MODE=view-miss GH_LIST_MODE=none GH_LOG="$fixture/none.log" "$helper" 'no such thing') > "$fixture/none.out" 2>&1; then
  echo 'resolve-pr unexpectedly resolved an empty search' >&2
  exit 1
fi
grep -Fq 'ask the user' "$fixture/none.out"

# Missing required argument handling: usage error, not a gh call.
if (cd "$repo" && PATH="$bin:$PATH" GH_MODE=view-hit GH_LOG="$fixture/usage.log" bash -c "'$helper' --help") > "$fixture/usage.out" 2>&1; then
  echo 'resolve-pr unexpectedly accepted --help as a reference' >&2
  exit 1
fi
grep -Fq 'usage: resolve-pr' "$fixture/usage.out"
if [ -e "$fixture/usage.log" ]; then
  echo 'usage error should not call gh' >&2
  exit 1
fi

echo 'resolve-pr fixtures passed'
