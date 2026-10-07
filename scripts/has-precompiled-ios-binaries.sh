#!/usr/bin/env bash
# Exits 0 when the release for the current crate hash has signed binaries for
# every iOS target given, so cargokit downloads them instead of building the
# crate. CI sets up the compiler cache only when this fails.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
package="$repo_root/packages/zolana_mobile"
build_tool="$package/cargokit/build_tool"
temporary="$(mktemp -d)"
trap 'rm -rf "$temporary"' EXIT

(cd "$build_tool" && dart pub get > /dev/null)
cat > "$temporary/crate_hash.dart" <<'DART'
import 'package:build_tool/src/crate_hash.dart';

void main(List<String> args) => print(CrateHash.compute(args.single));
DART
hash="$(dart --packages="$build_tool/.dart_tool/package_config.json" "$temporary/crate_hash.dart" "$package/rust")"
prefix="$(sed -n 's/^ *url_prefix: *//p' "$package/rust/cargokit.yaml")"
for target in "$@"; do
  curl -fsSL -o /dev/null "$prefix$hash/${target}_libmopro_flutter_bindings.a.sig"
done
