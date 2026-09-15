#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
asset_dir="$repo_root/packages/zolana_mobile/example/assets/proving"
key_name="transfer_confidential_2_3.key"
key="$repo_root/.cache/proving/$key_name"
witness="$repo_root/fixtures/witness-2x3.json"
key_checksum="915ffeaf13ae754f737235c85106778a465d10711a47dd134156b7ddc2c4d2b6"
pk_checksum="d5abbf3d961ea14545dc2d2e8e86b38ef2be59f76eca200e84771e58e45a1766"
vk_checksum="9b39f5bd57f144fd03caf3456e26a5af3fbb6a8128f0f6c21284ee3617a3d2ba"
r1cs_checksum="aa0ab29a7e79904a319477a5eaca08dd729d80f434eb55fff5e62ea60d2ed738"
witness_checksum="198ac36808679a6f44cd0c87a869ab7dc4b99ebcdcea6421c06a4888d949b01d"

sha256() {
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{ print $1 }'
  else
    sha256sum "$1" | awk '{ print $1 }'
  fi
}

verify() {
  local path="$1"
  local expected="$2"
  if [[ "$(sha256 "$path")" != "$expected" ]]; then
    echo "checksum mismatch: $path" >&2
    exit 1
  fi
}

slice() {
  local source="$1"
  local offset="$2"
  local length="$3"
  local output="$4"
  (
    set +o pipefail
    tail -c "+$((offset + 1))" "$source" | head -c "$length"
  ) > "$output"
}

if [[ ! -f "$key" ]] || [[ "$(sha256 "$key")" != "$key_checksum" ]]; then
  base_url="${ZOLANA_PROVING_KEYS_URL:-https://d3gbdb0egjwcw9.cloudfront.net}"
  temporary_key="$(mktemp)"
  trap 'rm -f "$temporary_key"' EXIT
  echo "downloading $key_name"
  curl -fsSL "$base_url/proving-keys/a5ff0c508cac3f51/$key_name" -o "$temporary_key"
  if [[ "$(sha256 "$temporary_key")" != "$key_checksum" ]]; then
    echo "downloaded proving key checksum does not match the lockfile" >&2
    exit 1
  fi
  mkdir -p "$(dirname "$key")"
  mv "$temporary_key" "$key"
  trap - EXIT
fi

verify "$key" "$key_checksum"
verify "$witness" "$witness_checksum"

temporary_assets="$(mktemp -d)"
trap 'rm -rf "$temporary_assets"' EXIT
slice "$key" 12 10118929 "$temporary_assets/transfer_confidential_2_3.pk"
slice "$key" 10118941 364 "$temporary_assets/transfer_confidential_2_3.vk"
slice "$key" 10119305 6234528 "$temporary_assets/transfer_confidential_2_3.r1cs"
verify "$temporary_assets/transfer_confidential_2_3.pk" "$pk_checksum"
verify "$temporary_assets/transfer_confidential_2_3.vk" "$vk_checksum"
verify "$temporary_assets/transfer_confidential_2_3.r1cs" "$r1cs_checksum"

mkdir -p "$asset_dir"
cp "$temporary_assets"/* "$asset_dir/"
cp "$witness" "$asset_dir/witness-2x3.json"
echo "staged demo proving assets in $asset_dir"
