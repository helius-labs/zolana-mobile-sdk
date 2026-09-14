#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
revision="e6139f658c6961101d716e107a15a5ca9cecd143"
key="${1:-$repo_root/.cache/proving/transfer_confidential_2_3.key}"
request="${ZOLANA_DEMO_PROOF_REQUEST:-$repo_root/fixtures/prove-request-2x3.json}"
assignment="$repo_root/fixtures/assignment-2x3.bin"

if [[ ! -f "$key" ]]; then
  echo "missing proving key: $key" >&2
  echo "run ./scripts/stage-demo-assets.sh first" >&2
  exit 1
fi
if [[ ! -f "$request" ]]; then
  echo "missing proof request: $request" >&2
  exit 1
fi

checkout="$(mktemp -d)"
temporary_assignment="$(mktemp)"
trap 'rm -rf "$checkout"; rm -f "$temporary_assignment"' EXIT

git clone --quiet --filter=blob:none --no-checkout \
  https://github.com/helius-labs/zolana "$checkout"
git -C "$checkout" checkout --quiet "$revision"

echo "solving the pinned 2→3 proof request with Go/gnark"
(
  cd "$checkout/prover/server"
  go run ./cmd/export-solved-assignment \
    --key "$key" \
    --request "$request" \
    --out "$temporary_assignment"
)
mv "$temporary_assignment" "$assignment"
echo "wrote $assignment"
