#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
temporary="$(mktemp -d "${TMPDIR:-/tmp}/zolana-mobile-consumer.XXXXXX")"
trap 'rm -rf "$temporary"' EXIT
package="$temporary/package"
consumer="$temporary/consumer"
mkdir -p "$package"

tar -C "$repo_root/packages/zolana_mobile" \
  --exclude=target --exclude=build --exclude=.dart_tool \
  -czf "$temporary/zolana_mobile.tar.gz" \
  pubspec.yaml README.md CHANGELOG.md LICENSE lib android ios rust native cargokit
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
cp "$repo_root/packages/zolana_mobile/example/assets/proving/transfer_confidential_2_3."{pk,vk,r1cs} "$consumer/assets/proving/"
cp "$repo_root/fixtures/prove-request-2x3.json" "$consumer/assets/proving/"
cp "$repo_root/scripts/consumer_fixture/prover_test.dart" "$consumer/integration_test/"
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
  *) echo 'usage: test-consumer.sh [android|ios|device]' >&2; exit 2 ;;
esac
