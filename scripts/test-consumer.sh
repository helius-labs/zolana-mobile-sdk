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
  # A dependency, not a dev one: Flutter leaves dev dependencies out of
  # release builds, and the bench app's release must carry the plugin.
  integration_test:
    sdk: flutter
dev_dependencies:
  flutter_test:
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
  bench-apk)
    # The bench app and its instrumentation for a device farm, arm64, as a
    # release (`flutter test` only has debug, where Dart runs interpreted and
    # Rust unoptimized), into ZOLANA_BENCH_DIR: the fresh and restart runs
    # are two instrumentation runs on one install, which the harness tells
    # apart by the keys on the device (scripts/devicefarm/testspec.yml).
    : "${ZOLANA_BENCH_DIR:?Set ZOLANA_BENCH_DIR for the two APKs}"
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
# Gradle builds the test APK for one build type, debug unless told; the
# template signs the release with the debug key, so the two APKs match.
block = "    defaultConfig {\n"
assert t.count(block) == 1, "defaultConfig"
t = t.replace(block, '    testBuildType = "release"\n\n' + block)
p.write_text(t)
PY
    # The instrumentation runs from classes the app carries but never uses,
    # which R8 drops from a release; keeping them leaves Dart's AOT code and
    # the native prover as they are.
    cat > android/app/proguard-rules.pro <<'PRO'
-keep class androidx.test.** { *; }
-keep class org.junit.** { *; }
-keep class junit.** { *; }
-keep class org.hamcrest.** { *; }
-keep class dev.flutter.plugins.integration_test.** { *; }
-dontwarn androidx.test.**
-dontwarn org.junit.**
PRO
    args=()
    encoded=()
    for define in "${defines[@]}"; do
      args+=(--dart-define="$define")
      encoded+=("$(printf '%s' "$define" | base64 | tr -d '\n')")
    done
    flutter build apk --release --target-platform android-arm64 \
      --target=integration_test/proof_timing_test.dart "${args[@]}"
    # gradlew on its own needs a JDK; Flutter's build found Android
    # Studio's, which a Mac with the SDK alone has.
    if [[ -z "${JAVA_HOME:-}" ]] && ! command -v java > /dev/null; then
      studio="/Applications/Android Studio.app/Contents/jbr/Contents/Home"
      [[ -d "$studio" ]] && export JAVA_HOME="$studio"
    fi
    (cd android && ./gradlew --quiet app:assembleReleaseAndroidTest \
      -Ptarget=integration_test/proof_timing_test.dart \
      -Ptarget-platform=android-arm64 \
      -Pdart-defines="$(IFS=,; echo "${encoded[*]}")")
    mkdir -p "$ZOLANA_BENCH_DIR"
    cp build/app/outputs/flutter-apk/app-release.apk "$ZOLANA_BENCH_DIR/app.apk"
    cp build/app/outputs/apk/androidTest/release/app-release-androidTest.apk "$ZOLANA_BENCH_DIR/app-androidTest.apk"
    ls -la "$ZOLANA_BENCH_DIR" ;;
  bench-ipa)
    # The bench app and its XCTest bundle for a device farm, unsigned (the
    # farm re-signs), into ZOLANA_BENCH_DIR. iOS test runs there get one
    # install each, so the harness makes the fresh and restart rows in one
    # process (`both`). The template's RunnerTests target runs the Dart
    # tests through the integration_test plugin of the host app, reached at
    # run time: importing the module would link a second copy into the test
    # bundle, blind to the app's engine.
    : "${ZOLANA_BENCH_DIR:?Set ZOLANA_BENCH_DIR for the app and test bundle}"
    label="${ZOLANA_TIMING_LABEL:-device}"
    args=(--dart-define=ZOLANA_TIMING_LABEL="$label" --dart-define=ZOLANA_TIMING_RUN=both)
    if [[ -n "${ZOLANA_TIMING_SHAPES:-}" ]]; then
      args+=(--dart-define=ZOLANA_TIMING_SHAPES="$ZOLANA_TIMING_SHAPES")
    fi
    cat > ios/RunnerTests/RunnerTests.swift <<'SWIFT'
import XCTest

// Runs the Dart integration tests of the host app through its
// integration_test plugin (FLTIntegrationTestRunner), without importing the
// module into this bundle.
final class RunnerTests: XCTestCase {
  func testIntegrationTests() {
    let runnerClass = NSClassFromString("FLTIntegrationTestRunner") as! NSObject.Type
    let runner = runnerClass.init()
    typealias Results = @convention(block) (Selector, Bool, String?) -> Void
    let results: Results = { _, success, message in
      XCTAssert(success, message ?? "a Dart test failed")
    }
    runner.perform(NSSelectorFromString("testIntegrationTestWithResults:"), with: results)
  }
}
SWIFT
    flutter build ios --release --no-codesign \
      --target=integration_test/proof_timing_test.dart "${args[@]}"
    (cd ios && xcodebuild build-for-testing -quiet \
      -workspace Runner.xcworkspace -scheme Runner -configuration Release -sdk iphoneos \
      -derivedDataPath ../build/ios_integ \
      CODE_SIGNING_ALLOWED=NO CODE_SIGNING_REQUIRED=NO CODE_SIGN_IDENTITY="")
    products=build/ios_integ/Build/Products/Release-iphoneos
    mkdir -p "$ZOLANA_BENCH_DIR"
    rm -rf "$products/Payload" && mkdir "$products/Payload" && cp -R "$products/Runner.app" "$products/Payload/"
    (cd "$products" && rm -f app.ipa && zip -qr app.ipa Payload)
    (cd "$products/Runner.app/PlugIns" && rm -f tests.zip && zip -qr tests.zip RunnerTests.xctest)
    cp "$products/app.ipa" "$ZOLANA_BENCH_DIR/app.ipa"
    cp "$products/Runner.app/PlugIns/tests.zip" "$ZOLANA_BENCH_DIR/tests.zip"
    ls -la "$ZOLANA_BENCH_DIR" ;;
  *) echo 'usage: test-consumer.sh [android|ios|device|bench-apk|bench-ipa]' >&2; exit 2 ;;
esac
