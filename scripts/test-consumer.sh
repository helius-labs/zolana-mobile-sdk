#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# ZOLANA_CONSUMER_DIR builds the consumer at a fixed path, deleted first and
# kept afterwards. CI uses it because a compiler cache keys Rust outputs on the
# build paths, and to check the built native library.
if [[ -n "${ZOLANA_CONSUMER_DIR:-}" ]]; then
  temporary="$ZOLANA_CONSUMER_DIR"
  rm -rf "$temporary"
  mkdir -p "$temporary"
else
  temporary="$(mktemp -d "${TMPDIR:-/tmp}/zolana-mobile-consumer.XXXXXX")"
  trap 'rm -rf "$temporary"' EXIT
fi
package="$temporary/package"
consumer="$temporary/consumer"
mkdir -p "$package"

tar -C "$repo_root/packages/zolana_mobile" \
  --exclude=target --exclude=build --exclude=.dart_tool \
  -czf "$temporary/zolana_mobile.tar.gz" \
  pubspec.yaml README.md CHANGELOG.md LICENSE lib hook android rust native cargokit
tar -xzf "$temporary/zolana_mobile.tar.gz" -C "$package"
cargo metadata --manifest-path "$package/rust/Cargo.toml" --format-version 1 > "$temporary/metadata.json"

flutter create --platforms=android,ios --project-name=zolana_consumer "$consumer"
cat > "$consumer/pubspec.yaml" <<'YAML'
name: zolana_consumer
publish_to: none
environment:
  sdk: ^3.13.0
dependencies:
  flutter:
    sdk: flutter
  zolana_mobile:
    path: ../package
dev_dependencies:
  flutter_test:
    sdk: flutter
  integration_test:
    sdk: flutter
flutter:
  assets:
    - assets/proving/
YAML
mkdir -p "$consumer/assets/proving" "$consumer/integration_test"
cp "$repo_root/packages/zolana_mobile/example/assets/proving/transfer_confidential_2_2."{pk,vk,r1cs} "$consumer/assets/proving/"
cp "$repo_root"/fixtures/prove-request-*.json "$consumer/assets/proving/"
cp "$repo_root/scripts/consumer_fixture/prover_test.dart" "$consumer/integration_test/"
# The proof timing harness, run on the consumer so that CI builds one app.
cp "$repo_root/packages/zolana_mobile/example/integration_test/proof_timing_test.dart" "$consumer/integration_test/"
cd "$consumer"
flutter pub get
flutter analyze integration_test
printf 'Consumer package built from %s\n' "$temporary/zolana_mobile.tar.gz"
case "${1:-android}" in
  android) flutter build apk --release ;;
  ios) flutter build ios --simulator ;;
  device)
    : "${ZOLANA_TEST_DEVICE:?Set ZOLANA_TEST_DEVICE to an Android emulator or iOS simulator ID}"
    flutter test integration_test/prover_test.dart -d "$ZOLANA_TEST_DEVICE" ;;
  bench)
    # The proof timing harness, twice on the same install: from an empty key
    # directory, kept installed, and then as a restart. The output goes to
    # ZOLANA_TIMING_DIR for scripts/proof-timing-ci.sh.
    : "${ZOLANA_TEST_DEVICE:?Set ZOLANA_TEST_DEVICE to an Android emulator or iOS simulator ID}"
    : "${ZOLANA_TIMING_DIR:?Set ZOLANA_TIMING_DIR for the proof timing logs}"
    mkdir -p "$ZOLANA_TIMING_DIR"
    label="${ZOLANA_TIMING_LABEL:-device}"
    shapes=()
    if [[ -n "${ZOLANA_TIMING_SHAPES:-}" ]]; then
      shapes=(--dart-define=ZOLANA_TIMING_SHAPES="$ZOLANA_TIMING_SHAPES")
    fi
    flutter test integration_test/proof_timing_test.dart -d "$ZOLANA_TEST_DEVICE" --no-uninstall \
      --dart-define=ZOLANA_TIMING_RUN=fresh --dart-define=ZOLANA_TIMING_LABEL="$label" ${shapes[@]+"${shapes[@]}"} \
      | tee "$ZOLANA_TIMING_DIR/$label-fresh.log"
    flutter test integration_test/proof_timing_test.dart -d "$ZOLANA_TEST_DEVICE" \
      --dart-define=ZOLANA_TIMING_RUN=restart --dart-define=ZOLANA_TIMING_LABEL="$label" ${shapes[@]+"${shapes[@]}"} \
      | tee "$ZOLANA_TIMING_DIR/$label-restart.log" ;;
  *) echo 'usage: test-consumer.sh [android|ios|device|bench]' >&2; exit 2 ;;
esac
