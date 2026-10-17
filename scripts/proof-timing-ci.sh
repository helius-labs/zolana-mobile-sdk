#!/usr/bin/env bash
# The proof timing report of a CI run: each phone's timing log under
# ZOLANA_TIMING_DIR/<label>/, against that phone's last report in the
# history of `main` (scripts/proof-timing-history.sh), into the job summary,
# a report.json per phone, ZOLANA_TIMING_DIR/report.md, and one comment on
# the pull request, updated in place. A run by hand comments on the pull
# request of its branch.
#
#   proof-timing-ci.sh
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
timing="${ZOLANA_TIMING_DIR:?set ZOLANA_TIMING_DIR}"
if ! compgen -G "$timing/*/*.log" > /dev/null; then
  echo "::notice::no proof timing logs under $timing"
  exit 0
fi

after=""
number=""
if [[ -n "${GITHUB_REPOSITORY:-}" ]]; then
  # A pull request's run is of a merge commit; name the head it tests.
  case "${GITHUB_EVENT_NAME:-}" in
    pull_request)
      number="$(jq -r .pull_request.number "$GITHUB_EVENT_PATH")"
      head="$(jq -r .pull_request.head.sha "$GITHUB_EVENT_PATH")" ;;
    workflow_dispatch)
      number="$(gh pr list --repo "$GITHUB_REPOSITORY" --head "$GITHUB_REF_NAME" --state open --json number --jq '.[0].number // empty')"
      head="$GITHUB_SHA" ;;
    *) head="$GITHUB_SHA" ;;
  esac
  if [[ -n "$number" ]]; then
    after="this pull request at [\`${head:0:7}\`](https://github.com/$GITHUB_REPOSITORY/commit/$head)"
  else
    after="\`$GITHUB_REF_NAME\` at [\`${head:0:7}\`](https://github.com/$GITHUB_REPOSITORY/commit/$head)"
  fi
  # Each phone against its last report of main before this commit: a rerun
  # after the history took this commit compares with the one before.
  for dir in "$timing"/*/; do
    label="$(basename "$dir")"
    if last="$(gh api "repos/$GITHUB_REPOSITORY/contents/history/$label.jsonl?ref=proof-timing" \
        -H 'Accept: application/vnd.github.raw' 2> /dev/null \
        | jq -c --arg sha "$GITHUB_SHA" 'select(.commit != $sha)' | tail -n 1)" && [[ -n "$last" ]]; then
      printf '%s\n' "$last" > "$dir/baseline.json"
    else
      echo "::notice::no proof timing history for $label on main yet"
    fi
  done
fi

python3 "$repo_root/scripts/proof-timing-report.py" \
  --logs "$timing" \
  ${GITHUB_REPOSITORY:+--repo "$GITHUB_REPOSITORY"} \
  ${after:+--after "$after"} \
  --summary "${GITHUB_STEP_SUMMARY:-/dev/null}" > "$timing/report.md"
cat "$timing/report.md"

if [[ -n "$number" ]]; then
  marker="<!-- proof-timing -->"
  body="$(printf '%s\n%s' "$marker" "$(cat "$timing/report.md")")"
  existing="$(gh api "repos/$GITHUB_REPOSITORY/issues/$number/comments" --paginate --jq ".[] | select(.body | startswith(\"$marker\")) | .id" | head -1 || true)"
  if [[ -n "$existing" ]]; then
    gh api -X PATCH "repos/$GITHUB_REPOSITORY/issues/comments/$existing" -f body="$body" > /dev/null || echo "::notice::could not update the proof timing comment"
  else
    gh api -X POST "repos/$GITHUB_REPOSITORY/issues/$number/comments" -f body="$body" > /dev/null || echo "::notice::could not post the proof timing comment"
  fi
fi
