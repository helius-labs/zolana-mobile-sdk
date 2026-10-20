#!/usr/bin/env bash
set -euo pipefail

# Stages the example app's proving assets: the 2 -> 2 transfer's zopro key and
# the fixture requests. The key downloads from the proving keys' CDN (or
# copies from $ZOPRO_KEYS_DIR when set) and is checked against the wallet's
# lockfile.
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
asset_dir="$repo_root/packages/zolana_mobile/example/assets/proving"
key_name="transfer_confidential_2_2.zopro"
# The lockfile's prefix; its last part names the key set's directory, as the
# wallet lays out its key directory.
key_prefix="proving-keys/3a1c88f90f609eb1/"
key="$repo_root/.cache/proving/$(basename "$key_prefix")/$key_name"
key_checksum="767f1ad5a935e2fcba0094053922ae8e71020e1ea97051f84386440c18b3fa0a"
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

if [[ ! -f "$key" ]] || [[ "$(sha256 "$key")" != "$key_checksum" ]]; then
  temporary_key="$(mktemp)"
  trap 'rm -f "$temporary_key"' EXIT
  if [[ -n "${ZOPRO_KEYS_DIR:-}" ]]; then
    cp "$ZOPRO_KEYS_DIR/$key_name" "$temporary_key"
  else
    base_url="${ZOLANA_PROVING_KEYS_URL:-https://d3gbdb0egjwcw9.cloudfront.net}"
    echo "downloading $key_name"
    curl -fsSL "$base_url/$key_prefix$key_name" -o "$temporary_key"
  fi
  if [[ "$(sha256 "$temporary_key")" != "$key_checksum" ]]; then
    echo "the proving key's checksum does not match the lockfile" >&2
    exit 1
  fi
  mkdir -p "$(dirname "$key")"
  mv "$temporary_key" "$key"
  trap - EXIT
fi

verify "$key" "$key_checksum"
verify "$repo_root/fixtures/prove-request-2x2.json" "$request_checksum"

mkdir -p "$asset_dir"
cp "$key" "$asset_dir/$key_name"
# The fixture requests of every shape, for the proof timing harness.
cp "$repo_root"/fixtures/prove-request-*.json "$asset_dir/"
echo "staged demo proving assets in $asset_dir"
