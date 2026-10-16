#!/usr/bin/env bash
# The proof timing report of one CI job: the logs of the two timing runs
# under ZOLANA_TIMING_DIR, against the last report of the same label in the
# history of `main` (scripts/proof-timing-history.sh), into the job summary,
# a report.json for the artifact, and one comment on the pull request,
# updated in place.
#
#   proof-timing-ci.sh <label>
set -euo pipefail

label="$1"
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
timing="${ZOLANA_TIMING_DIR:?set ZOLANA_TIMING_DIR}"
if ! compgen -G "$timing/*.log" > /dev/null; then
  echo "::notice::no proof timing logs under $timing"
  exit 0
fi

baseline=""
before=""
after=""
if [[ -n "${GITHUB_REPOSITORY:-}" ]]; then
  # A pull request's run is of a merge commit; name the head it tests.
  if [[ "${GITHUB_EVENT_NAME:-}" == "pull_request" ]]; then
    head="$(jq -r .pull_request.head.sha "$GITHUB_EVENT_PATH")"
    after="this pull request at [\`${head:0:7}\`](https://github.com/$GITHUB_REPOSITORY/commit/$head)"
  else
    after="\`main\` at [\`${GITHUB_SHA:0:7}\`](https://github.com/$GITHUB_REPOSITORY/commit/$GITHUB_SHA)"
  fi
  if last="$(gh api "repos/$GITHUB_REPOSITORY/contents/history/$label.jsonl?ref=proof-timing" \
      -H 'Accept: application/vnd.github.raw' 2> /dev/null | tail -n 1)" && [[ -n "$last" ]]; then
    mkdir -p "$timing/baseline"
    jq '.report' <<< "$last" > "$timing/baseline/report.json"
    baseline="$timing/baseline/report.json"
    commit="$(jq -r '.commit' <<< "$last")"
    before="\`main\` at [\`${commit:0:7}\`](https://github.com/$GITHUB_REPOSITORY/commit/$commit)"
  else
    echo "::notice::no proof timing history for $label on main yet"
  fi
fi

python3 "$repo_root/scripts/proof-timing-report.py" \
  --logs "$timing" \
  ${baseline:+--baseline "$baseline" --before "$before"} \
  ${after:+--after "$after"} \
  --out "$timing/report.json" \
  --summary "${GITHUB_STEP_SUMMARY:-/dev/null}" \
  --title "Proof timing: $label" > "$timing/report.md"
cat "$timing/report.md"

# One comment per label on the pull request, replaced on each run.
if [[ "${GITHUB_EVENT_NAME:-}" == "pull_request" ]]; then
  number="$(jq -r .pull_request.number "$GITHUB_EVENT_PATH")"
  marker="<!-- proof-timing:$label -->"
  body="$(printf '%s\n%s' "$marker" "$(cat "$timing/report.md")")"
  existing="$(gh api "repos/$GITHUB_REPOSITORY/issues/$number/comments" --paginate --jq ".[] | select(.body | startswith(\"$marker\")) | .id" | head -1 || true)"
  if [[ -n "$existing" ]]; then
    gh api -X PATCH "repos/$GITHUB_REPOSITORY/issues/comments/$existing" -f body="$body" > /dev/null || echo "::notice::could not update the proof timing comment"
  else
    gh api -X POST "repos/$GITHUB_REPOSITORY/issues/$number/comments" -f body="$body" > /dev/null || echo "::notice::could not post the proof timing comment"
  fi
fi
