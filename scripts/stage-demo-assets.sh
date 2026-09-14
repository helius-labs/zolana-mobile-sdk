#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
asset_dir="$repo_root/packages/zolana_mobile/example/assets/proving"
key_name="transfer_confidential_2_3.key"
key="$repo_root/.cache/proving/$key_name"
assignment="$repo_root/fixtures/assignment-2x3.bin"
expected="915ffeaf13ae754f737235c85106778a465d10711a47dd134156b7ddc2c4d2b6"

sha256() {
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{ print $1 }'
  else
    sha256sum "$1" | awk '{ print $1 }'
  fi
}

if [[ ! -f "$key" ]] || [[ "$(sha256 "$key")" != "$expected" ]]; then
  base_url="${ZOLANA_PROVING_KEYS_URL:-https://d3gbdb0egjwcw9.cloudfront.net}"
  temporary_key="$(mktemp)"
  trap 'rm -f "$temporary_key"' EXIT
  echo "downloading $key_name"
  curl -fsSL "$base_url/proving-keys/a5ff0c508cac3f51/$key_name" -o "$temporary_key"
  if [[ "$(sha256 "$temporary_key")" != "$expected" ]]; then
    echo "downloaded proving key checksum does not match the lockfile" >&2
    exit 1
  fi
  mkdir -p "$(dirname "$key")"
  mv "$temporary_key" "$key"
  trap - EXIT
fi

if [[ ! -f "$assignment" ]]; then
  echo "missing assignment fixture; regenerating it"
  "$repo_root/scripts/regenerate-assignment.sh" "$key"
fi

mkdir -p "$asset_dir"
cp "$key" "$asset_dir/transfer_confidential_2_3.key"
cp "$assignment" "$asset_dir/assignment-2x3.bin"
echo "staged demo proving assets in $asset_dir"
