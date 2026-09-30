#!/usr/bin/env bash
# Applies the fix for flutter/flutter#181771 to the Flutter SDK on PATH, until
# a Flutter release ships it (flutter/flutter#193142). Without it, `flutter
# test` on an iOS simulator can launch the app before `log stream` is ready,
# miss the Dart VM Service URL the app logs on startup, and wait forever.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
patch="$repo_root/scripts/patches/flutter-ios-simulator-log-stream.patch"
flutter_root="${FLUTTER_ROOT:-$(cd "$(dirname "$(readlink -f "$(command -v flutter)")")/.." && pwd)}"

if git -C "$flutter_root" apply --reverse --check "$patch" 2>/dev/null; then
  echo "The Flutter tool in $flutter_root is already patched"
  exit 0
fi
if ! git -C "$flutter_root" apply --check "$patch"; then
  echo "The Flutter tool patch does not apply to $(flutter --version | head -1)." >&2
  echo "If that release includes flutter/flutter#193142, delete the patch and this script." >&2
  exit 1
fi
git -C "$flutter_root" apply "$patch"
# The flutter launcher rebuilds its snapshot when the stamp is missing.
rm -f "$flutter_root/bin/cache/flutter_tools.stamp"
flutter --version > /dev/null
echo "Patched the Flutter tool in $flutter_root"
