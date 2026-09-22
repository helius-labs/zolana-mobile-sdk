#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
asset_dir="$repo_root/packages/zolana_mobile/example/assets/proving"
key_name="transfer_confidential_2_3.key"
key="$repo_root/.cache/proving/$key_name"
witness="$repo_root/fixtures/witness-2x3.json"
key_checksum="55bc6b864955505d443f69a6c7ccde982ac4df75a2a48989611f41cd4475f9ed"
pk_checksum="47b4f31c9bbc3c6251a99cb94eccfc4f667223fe57d0eb95e6950d2df4a778df"
vk_checksum="77bdbb4dd66e6f32c71884fc6794192a0a42eb0fa25dc60aa242855ff4fcfa3f"
r1cs_checksum="1488963e03cf968dea4083a8e3f86395ccbdb7338afc31aa067163ff4df4c51c"
witness_checksum="9df02a206a7764d36138c6126dcf327857081f256e7bc188915e1c8540d56140"
request_checksum="fd7eb087ed714d78037a6beecbfe0bd6c96f34918fdf38b13d242c54056c9d29"

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
  curl -fsSL "$base_url/proving-keys/7765d8fa45c5416c/$key_name" -o "$temporary_key"
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
slice "$key" 12 10249347 "$temporary_assets/transfer_confidential_2_3.pk"
slice "$key" 10249359 364 "$temporary_assets/transfer_confidential_2_3.vk"
slice "$key" 10249723 6491080 "$temporary_assets/transfer_confidential_2_3.r1cs"
verify "$temporary_assets/transfer_confidential_2_3.pk" "$pk_checksum"
verify "$temporary_assets/transfer_confidential_2_3.vk" "$vk_checksum"
verify "$temporary_assets/transfer_confidential_2_3.r1cs" "$r1cs_checksum"

mkdir -p "$asset_dir"
cp "$temporary_assets"/* "$asset_dir/"
cp "$witness" "$asset_dir/witness-2x3.json"
cp "$repo_root/fixtures/prove-request-2x3.json" "$asset_dir/prove-request-2x3.json"
echo "staged demo proving assets in $asset_dir"
