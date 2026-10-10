#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
asset_dir="$repo_root/packages/zolana_mobile/example/assets/proving"
key_name="transfer_confidential_2_2.key"
# The lockfile's prefix (check-upstream.sh compares it); its last part names
# the key set's directory, as the wallet lays out its key directory.
key_prefix="proving-keys/3a1c88f90f609eb1/"
key="$repo_root/.cache/proving/$(basename "$key_prefix")/$key_name"
key_checksum="e810d52962f4558b0bd50b768a89490759e15a0623d4c2c43318f541b8cc61ff"
pk_checksum="4eb03bbf45b0893075ed7093e5d10fe7c67867f0daeedeba7cc04dcee850a304"
vk_checksum="40b57a4bc6b638c0849248e97e997b74ec814444e7ca76a3cef082d33e65b26a"
r1cs_checksum="a357f10d2eb280be11181699d139864147463da495cc1a5add8761a9cb718c59"
request_checksum="d67272212c4b3d110f40d6a6daef0529e361312862822a895d9d8710d639b90d"

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
  curl -fsSL "$base_url/$key_prefix$key_name" -o "$temporary_key"
  if [[ "$(sha256 "$temporary_key")" != "$key_checksum" ]]; then
    echo "downloaded proving key checksum does not match the lockfile" >&2
    exit 1
  fi
  mkdir -p "$(dirname "$key")"
  mv "$temporary_key" "$key"
  trap - EXIT
fi

verify "$key" "$key_checksum"
verify "$repo_root/fixtures/prove-request-2x2.json" "$request_checksum"

temporary_assets="$(mktemp -d)"
trap 'rm -rf "$temporary_assets"' EXIT
slice "$key" 12 10092361 "$temporary_assets/transfer_confidential_2_2.pk"
slice "$key" 10092373 364 "$temporary_assets/transfer_confidential_2_2.vk"
slice "$key" 10092737 6314558 "$temporary_assets/transfer_confidential_2_2.r1cs"
verify "$temporary_assets/transfer_confidential_2_2.pk" "$pk_checksum"
verify "$temporary_assets/transfer_confidential_2_2.vk" "$vk_checksum"
verify "$temporary_assets/transfer_confidential_2_2.r1cs" "$r1cs_checksum"

mkdir -p "$asset_dir"
cp "$temporary_assets"/* "$asset_dir/"
cp "$repo_root/fixtures/prove-request-2x2.json" "$asset_dir/prove-request-2x2.json"
echo "staged demo proving assets in $asset_dir"
