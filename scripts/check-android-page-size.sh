#!/usr/bin/env bash
# Fails unless every native library is aligned for 16 KB memory pages, as
# Android devices with 16 KB pages and Google Play require: each LOAD segment
# aligned to at least 16 KB.
#
#   check-android-page-size.sh app.apk       # the 64-bit ABIs, as Google Play
#   check-android-page-size.sh lib.so ...    # each library given
#
# An APK is also checked with `zipalign -c -P 16` when build-tools are found.
# READELF overrides the readelf to use; otherwise llvm-readelf, readelf or the
# NDK's llvm-readelf.
set -euo pipefail

find_readelf() {
  if [[ -n "${READELF:-}" ]]; then
    echo "$READELF"
  elif command -v llvm-readelf >/dev/null 2>&1; then
    echo llvm-readelf
  elif command -v readelf >/dev/null 2>&1; then
    echo readelf
  else
    local ndk="${ANDROID_NDK_HOME:-${ANDROID_NDK_ROOT:-}}"
    compgen -G "$ndk/toolchains/llvm/prebuilt/*/bin/llvm-readelf" | head -1 || true
  fi
}

readelf_bin="$(find_readelf)"
if [[ -z "$readelf_bin" ]]; then
  echo "no readelf: install binutils or set READELF or ANDROID_NDK_HOME" >&2
  exit 1
fi

failed=0

check_library() {
  local file="$1" name="$2" aligns align
  aligns="$("$readelf_bin" -lW "$file" | awk '$1 == "LOAD" { print $NF }')"
  if [[ -z "$aligns" ]]; then
    echo "no LOAD segments: $name"
    failed=1
    return
  fi
  for align in $aligns; do
    if ((align < 0x4000)); then
      echo "not 16 KB aligned (LOAD align $align): $name"
      failed=1
      return
    fi
  done
  echo "16 KB aligned: $name"
}

check_apk() {
  local apk="$1" dir library zipalign
  dir="$(mktemp -d)"
  unzip -q -o "$apk" 'lib/arm64-v8a/*.so' 'lib/x86_64/*.so' -d "$dir" || true
  if ! compgen -G "$dir/lib/*/*.so" >/dev/null; then
    echo "no 64-bit native libraries in $apk"
    failed=1
  fi
  for library in "$dir"/lib/*/*.so; do
    [[ -e "$library" ]] && check_library "$library" "${library#"$dir/"}"
  done
  rm -r "$dir"

  zipalign="$(compgen -G "${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}/build-tools/*/zipalign" | sort | tail -1 || true)"
  if [[ -n "$zipalign" ]]; then
    if "$zipalign" -c -P 16 4 "$apk"; then
      echo "16 KB zip alignment: $apk"
    else
      echo "native libraries not 16 KB aligned in the zip: $apk"
      failed=1
    fi
  fi
}

if (($# == 0)); then
  echo "usage: $0 app.apk | lib.so ..." >&2
  exit 2
fi
for file in "$@"; do
  case "$file" in
    *.apk) check_apk "$file" ;;
    *) check_library "$file" "$file" ;;
  esac
done
exit "$failed"
