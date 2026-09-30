#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
asset_dir="$repo_root/packages/zolana_mobile/example/assets/proving"
key_name="transfer_confidential_2_3.key"
key="$repo_root/.cache/proving/$key_name"
witness="$repo_root/fixtures/witness-2x3.json"
key_checksum="dc40be6315c921ff9c69651e51d891ca3cef936630d98873bb0a767c039ab0dd"
pk_checksum="1f12fee2a0e6e3be36715065b8db086f9bab30d7ed04165bb4895ed711b681ca"
vk_checksum="28a736d094b3cfc8e47aa1defea4cb43f26bb8090e98503f06c7d237b8f483cc"
r1cs_checksum="46e11bd66557ff189246cf80addd4aa9bf3bcbb9396b626473a26daadc695901"
witness_checksum="c36a782e954b5406b68c194ce91f154e01552286ba08823f1fd7535ac43c1ba7"
request_checksum="32632ad9047783faada219a5de476f9eeae018c11305404d2a2dfd3fabdce13c"

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
  curl -fsSL "$base_url/proving-keys/11b65848058386ad/$key_name" -o "$temporary_key"
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
verify "$repo_root/fixtures/prove-request-2x3.json" "$request_checksum"

temporary_assets="$(mktemp -d)"
trap 'rm -rf "$temporary_assets"' EXIT
slice "$key" 12 10346681 "$temporary_assets/transfer_confidential_2_3.pk"
slice "$key" 10346693 364 "$temporary_assets/transfer_confidential_2_3.vk"
slice "$key" 10347057 6581743 "$temporary_assets/transfer_confidential_2_3.r1cs"
verify "$temporary_assets/transfer_confidential_2_3.pk" "$pk_checksum"
verify "$temporary_assets/transfer_confidential_2_3.vk" "$vk_checksum"
verify "$temporary_assets/transfer_confidential_2_3.r1cs" "$r1cs_checksum"

mkdir -p "$asset_dir"
cp "$temporary_assets"/* "$asset_dir/"
cp "$witness" "$asset_dir/witness-2x3.json"
cp "$repo_root/fixtures/prove-request-2x3.json" "$asset_dir/prove-request-2x3.json"
echo "staged demo proving assets in $asset_dir"
