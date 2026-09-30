#!/usr/bin/env bash
# Fails when an object in a Mach-O archive or binary needs a newer iOS than the given version.
set -euo pipefail

file="${1:?usage: check-ios-minimum.sh <archive-or-binary> <max-ios-version>}"
limit="${2:?usage: check-ios-minimum.sh <archive-or-binary> <max-ios-version>}"

need="$(otool -l "$file" | awk '
  $1 == "cmd" { cmd = $2 }
  cmd == "LC_BUILD_VERSION" && $1 == "minos" { print $2 }
  cmd == "LC_VERSION_MIN_IPHONEOS" && $1 == "version" { print $2 }
' | sort -uV | tail -1)"

[[ -n "$need" ]] || { echo "$file: no iOS minimum found" >&2; exit 1; }
if [[ "$(printf '%s\n%s\n' "$need" "$limit" | sort -V | tail -1)" != "$limit" ]]; then
  echo "$file needs iOS $need, newer than $limit" >&2
  exit 1
fi
echo "$file needs iOS $need"
