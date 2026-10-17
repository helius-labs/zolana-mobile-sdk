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
    # Each run rebuilds the app, and hosted macOS runners sometimes fail
    # that build (no Xcode build settings) or the launch: a run that failed
    # before its first proof is tried once more, one that failed while
    # proving is not. The app stays installed between and after the runs,
    # with its keys.
    timing_run() {
      local run="$1" log="$ZOLANA_TIMING_DIR/$label-$1.log" attempt
      for attempt in 1 2; do
        if flutter test integration_test/proof_timing_test.dart -d "$ZOLANA_TEST_DEVICE" --no-uninstall \
            --dart-define=ZOLANA_TIMING_RUN="$run" --dart-define=ZOLANA_TIMING_LABEL="$label" ${shapes[@]+"${shapes[@]}"} \
            | tee "$log"; then
          return 0
        fi
        if grep -q 'ZOLANA_TIMING {' "$log" || [[ $attempt == 2 ]]; then
          return 1
        fi
        echo "::warning::the $run timing run did not start; trying once more"
      done
    }
    timing_run fresh
    timing_run restart ;;
  bench-apk)
    # The bench app and its instrumentation for a device farm, arm64, into
    # ZOLANA_BENCH_APK_DIR: the fresh and restart runs are two instrumentation
    # runs on one install, which the harness tells apart by the keys on the
    # device (scripts/devicefarm/testspec.yml).
    : "${ZOLANA_BENCH_APK_DIR:?Set ZOLANA_BENCH_APK_DIR for the two APKs}"
    label="${ZOLANA_TIMING_LABEL:-device}"
    defines=(ZOLANA_TIMING_LABEL="$label")
    if [[ -n "${ZOLANA_TIMING_SHAPES:-}" ]]; then
      defines+=(ZOLANA_TIMING_SHAPES="$ZOLANA_TIMING_SHAPES")
    fi
    mkdir -p android/app/src/androidTest/java/com/example/zolana_consumer
    cat > android/app/src/androidTest/java/com/example/zolana_consumer/MainActivityTest.java <<'JAVA'
package com.example.zolana_consumer;

import androidx.test.rule.ActivityTestRule;
import dev.flutter.plugins.integration_test.FlutterTestRunner;
import org.junit.Rule;
import org.junit.runner.RunWith;

@RunWith(FlutterTestRunner.class)
public class MainActivityTest {
  @Rule
  public ActivityTestRule<MainActivity> rule =
      new ActivityTestRule<>(MainActivity.class, true, false);
}
JAVA
    # The test classpath must take the versions the integration_test plugin
    # already puts on the app's.
    plugin="$(ls "${FLUTTER_ROOT:-$(dirname "$(dirname "$(readlink -f "$(command -v flutter)")")")}"/packages/integration_test/android/build.gradle* | head -1)"
    runner="$(grep -o 'androidx.test:runner:[0-9.]*' "$plugin" | head -1)"
    espresso="$(grep -o 'androidx.test.espresso:espresso-core:[0-9.]*' "$plugin" | head -1)"
    RUNNER="${runner:-androidx.test:runner:1.3.0}" ESPRESSO="${espresso:-androidx.test.espresso:espresso-core:3.3.0}" python3 - <<'PY'
import os, pathlib
p = pathlib.Path("android/app/build.gradle.kts")
t = p.read_text()
anchor = '        applicationId = "com.example.zolana_consumer"\n'
assert t.count(anchor) == 1, "applicationId"
t = t.replace(anchor, anchor + '        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"\n')
t += f'''
dependencies {{
    androidTestImplementation("{os.environ["RUNNER"]}")
    androidTestImplementation("{os.environ["ESPRESSO"]}")
}}
'''
p.write_text(t)
PY
    args=()
    encoded=()
    for define in "${defines[@]}"; do
      args+=(--dart-define="$define")
      encoded+=("$(printf '%s' "$define" | base64 | tr -d '\n')")
    done
    flutter build apk --debug --target-platform android-arm64 \
      --target=integration_test/proof_timing_test.dart "${args[@]}"
    # gradlew on its own needs a JDK; Flutter's build found Android
    # Studio's, which a Mac with the SDK alone has.
    if [[ -z "${JAVA_HOME:-}" ]] && ! command -v java > /dev/null; then
      studio="/Applications/Android Studio.app/Contents/jbr/Contents/Home"
      [[ -d "$studio" ]] && export JAVA_HOME="$studio"
    fi
    (cd android && ./gradlew --quiet app:assembleAndroidTest \
      -Ptarget=integration_test/proof_timing_test.dart \
      -Ptarget-platform=android-arm64 \
      -Pdart-defines="$(IFS=,; echo "${encoded[*]}")")
    mkdir -p "$ZOLANA_BENCH_APK_DIR"
    cp build/app/outputs/flutter-apk/app-debug.apk "$ZOLANA_BENCH_APK_DIR/app.apk"
    cp build/app/outputs/apk/androidTest/debug/app-debug-androidTest.apk "$ZOLANA_BENCH_APK_DIR/app-androidTest.apk"
    ls -la "$ZOLANA_BENCH_APK_DIR" ;;
  *) echo 'usage: test-consumer.sh [android|ios|device|bench|bench-apk]' >&2; exit 2 ;;
esac
