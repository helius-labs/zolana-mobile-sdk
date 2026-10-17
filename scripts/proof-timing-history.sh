#!/usr/bin/env bash
# Adds this commit's proof timing reports to the history on the
# `proof-timing` branch, pushes it, and comments the report on the commit.
# CI runs it after each push to `main`, on the directory of
# scripts/proof-timing-ci.sh: a report.json per phone, <label>/report.json,
# and report.md.
#
#   proof-timing-history.sh <reports>
set -euo pipefail

reports="$1"
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if ! compgen -G "$reports/*/report.json" > /dev/null; then
  echo "::notice::no proof timing reports under $reports"
  exit 0
fi

history="$(mktemp -d)/history"
cd "$repo_root"
if git fetch -q --depth=1 origin proof-timing 2> /dev/null; then
  git worktree add -q --detach "$history" FETCH_HEAD
else
  git worktree add -q --detach "$history"
  git -C "$history" checkout -q --orphan proof-timing
  git -C "$history" rm -rfq .
fi

python3 "$repo_root/scripts/proof-timing-history.py" \
  --reports "$reports" \
  --history "$history" \
  --commit "$GITHUB_SHA" \
  --date "$(git log -1 --format=%cI "$GITHUB_SHA")" \
  --repo "$GITHUB_REPOSITORY"

cd "$history"
git add -A
if git diff --cached --quiet; then
  exit 0
fi
git -c user.name='github-actions[bot]' \
  -c user.email='41898282+github-actions[bot]@users.noreply.github.com' \
  commit -q -m "proof timing of ${GITHUB_SHA:0:7}"
git push -q origin HEAD:refs/heads/proof-timing

# The report on the commit, before (the commit of main before it) and after
# (this commit).
gh api -X POST "repos/$GITHUB_REPOSITORY/commits/$GITHUB_SHA/comments" -f body="$(cat "$reports/report.md")" > /dev/null \
  || echo "::notice::could not comment the proof timing on the commit"
