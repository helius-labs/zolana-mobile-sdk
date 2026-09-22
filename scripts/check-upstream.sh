#!/usr/bin/env bash
# Fails when the vendored protocol, the Rust pins, or the staged proving key
# drift from the Zolana revision pinned in Cargo.toml. Set ZOLANA_SRC to an
# existing checkout to skip the clone.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
protocol="$repo_root/packages/zolana_mobile/native/rust-gnark/go/protocol"
inventory="$protocol/UPSTREAM_FILES.sha256"
stage_script="$repo_root/scripts/stage-demo-assets.sh"

revision="$(awk -F'"' '/^\[workspace.metadata.upstream\]/ { found = 1 } found && /^revision/ { print $2; exit }' "$repo_root/Cargo.toml")"
if [[ ! "$revision" =~ ^[0-9a-f]{40}$ ]]; then
  echo "missing [workspace.metadata.upstream] revision in Cargo.toml" >&2
  exit 1
fi

failed=0
fail() {
  echo "drift: $*" >&2
  failed=1
}

sha256() {
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 | awk '{ print $1 }'
  else
    sha256sum | awk '{ print $1 }'
  fi
}

source_dir="${ZOLANA_SRC:-}"
if [[ -z "$source_dir" ]]; then
  source_dir="$(mktemp -d "${TMPDIR:-/tmp}/zolana-upstream.XXXXXX")"
  trap 'rm -rf "$source_dir"' EXIT
  git -C "$source_dir" init -q
  git -C "$source_dir" remote add origin https://github.com/helius-labs/zolana.git
  git -C "$source_dir" fetch -q --depth=1 --filter=blob:none origin "$revision"
fi
upstream() {
  git -C "$source_dir" show "$revision:$1"
}

# The pinned revision must be on main, not a feature branch that can be
# rewritten or abandoned.
if [[ -z "${ZOLANA_SRC:-}" ]]; then
  git -C "$source_dir" fetch -q --filter=blob:none origin main
  if ! git -C "$source_dir" merge-base --is-ancestor "$revision" FETCH_HEAD; then
    fail "pinned revision $revision is not on zolana main"
  fi
fi

while read -r pinned path; do
  if [[ ! -f "$protocol/$path" ]]; then
    fail "inventory lists missing file $path"
    continue
  fi
  [[ "$(sha256 < "$protocol/$path")" == "$pinned" ]] || fail "vendored $path differs from its inventory hash"
  upstream_hash="$(upstream "prover/server/$path" 2>/dev/null | sha256)" || true
  [[ "$upstream_hash" == "$pinned" ]] || fail "vendored $path differs from prover/server/$path at $revision"
done < "$inventory"

while read -r path; do
  grep -q "  ${path}\$" "$inventory" || fail "vendored $path is not in UPSTREAM_FILES.sha256"
done < <(cd "$protocol" && find . -name '*.go' | sed 's#^\./##' | sort)

for module in github.com/consensys/gnark github.com/consensys/gnark-crypto; do
  want="$(upstream prover/server/go.mod | awk -v m="$module" '$1 == m { print $2 }')"
  for manifest in "$protocol/go.mod" "$protocol/../go.mod"; do
    got="$(awk -v m="$module" '$1 == m { print $2 }' "$manifest")"
    [[ "$got" == "$want" ]] || fail "${manifest#"$repo_root/"} pins $module $got, upstream uses $want"
  done
done

while read -r manifest; do
  while read -r rev; do
    [[ "$rev" == "$revision" ]] || fail "${manifest#"$repo_root/"} pins zolana rev $rev"
  done < <(grep -o 'helius-labs/zolana", rev = "[0-9a-f]*"' "$manifest" | awk -F'"' '{ print $3 }')
done < <(find "$repo_root" -name Cargo.toml -not -path '*/target/*' -not -path '*/.git/*')

lock="$(upstream prover/server/prover/provingkeys/proving-keys.lock)"
embedded_lock="$repo_root/packages/zolana_mobile/native/zolana-mobile/proving-keys.lock"
[[ "$(sha256 < "$embedded_lock")" == "$(printf '%s\n' "$lock" | sha256)" ]] ||
  fail "zolana-mobile/proving-keys.lock differs from the upstream proving-key lockfile"
want_url="$(upstream prover/server/prover/common/key_downloader.go | grep -o 'defaultProvingKeysBaseURL = "[^"]*"' | cut -d'"' -f2)"
got_url="$(grep -o 'DEFAULT_PROVING_KEYS_URL: &str = "[^"]*"' "$repo_root/packages/zolana_mobile/native/zolana-mobile/src/keys.rs" | cut -d'"' -f2)"
[[ "$got_url" == "$want_url" ]] || fail "default proving key URL $got_url, upstream uses $want_url"
key_name="$(awk -F'"' '/^key_name=/ { print $2 }' "$stage_script")"
key_checksum="$(awk -F'"' '/^key_checksum=/ { print $2 }' "$stage_script")"
key_prefix="$(grep -o 'proving-keys/[0-9a-f]*/' "$stage_script" | head -1)"
want_checksum="$(printf '%s' "$lock" | python3 -c "import json, sys; print(json.load(sys.stdin)['keys'][sys.argv[1]]['sha256'])" "$key_name")"
want_prefix="$(printf '%s' "$lock" | python3 -c "import json, sys; print(json.load(sys.stdin)['prefix'])")/"
[[ "$key_checksum" == "$want_checksum" ]] || fail "$key_name checksum $key_checksum, upstream lock has $want_checksum"
[[ "$key_prefix" == "$want_prefix" ]] || fail "proving key prefix $key_prefix, upstream lock has $want_prefix"

if [[ "$failed" -ne 0 ]]; then
  echo "vendored sources are out of sync with zolana $revision" >&2
  exit 1
fi
echo "vendored sources match zolana $revision"
