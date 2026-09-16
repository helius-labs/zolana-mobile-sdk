# Zolana Mobile SDK

Standalone Flutter bindings for Zolana proof-input primitives, Poseidon
hashing, and local Mopro/gnark Groth16 proving on Android and iOS.

## Layout

- `packages/zolana_mobile/native/zolana-mobile`: Zolana API and native prover handles.
- `packages/zolana_mobile/native/rust-gnark`: vendored Mopro backend, including
  strict witness validation, prepared keys, and the structured request adapter.
- `packages/zolana_mobile`: generated Flutter plugin and proof-only example.
- `fixtures`: committed 2→3 request and flattened gnark witness fixture.
- `scripts`: checksum-locked Mopro asset staging.

The package includes an offline transfer-draft helper for exercising Zolana
proof-input types. It creates synthetic state and is not an on-chain transaction
builder. The example UI intentionally exposes only local proof generation.

The generated native library keeps Mopro's internal
`mopro_flutter_bindings` stem. Applications import the public Dart package as
`package:zolana_mobile/zolana_mobile.dart`.

## Demo

Install Flutter, Rust, Go 1.25.7 or newer, and the platform toolchain. Then stage
the ignored proving assets:

```sh
./scripts/stage-demo-assets.sh
cd packages/zolana_mobile/example
flutter pub get
flutter run
```

Android builds compile the gnark bridge from source and require an installed
NDK. Set both variables to the NDK directory shown by Android Studio's SDK
Manager before running Flutter, for example:

```sh
export ANDROID_NDK_HOME="$HOME/Library/Android/sdk/ndk/27.1.12297006"
export ANDROID_NDK_ROOT="$ANDROID_NDK_HOME"
```

`stage-demo-assets.sh` downloads one checksum-locked packed Zolana key and
splits its `.pk`, `.vk`, and `.r1cs` sections at verified offsets. It also stages
the committed flattened witness JSON. Demo users do not run `mopro init` or
`mopro build`.

## Validation

```sh
cargo test --workspace
cd packages/zolana_mobile
flutter pub get
flutter analyze
flutter test
cd example
flutter pub get
flutter analyze
```

After staging assets, run the real proof test with:

```sh
cargo test -p zolana-mobile proves_and_verifies_staged_mopro_witness -- --ignored
```

## Rust dependencies

The Flutter-facing crate pins `mopro-ffi` to `sergeytimoshin/mopro` revision
`c1071f96fa5a28dca8e567dd3944331047450a2b`. The gnark 0.15 backend from that revision
is vendored with local fixes documented in its provenance file so the pub package
includes its complete source build. Protocol crates `zolana-hasher`, `zolana-keypair`, and
`zolana-transaction` are pinned to upstream revision
`e6139f658c6961101d716e107a15a5ca9cecd143`.

## Current boundary

`LocalProver.proveRequest` accepts a structured Zolana `/prove` request and builds
its witness on-device. It returns a locally verified canonical proof for the
existing Zolana transaction flow. The example uses a public 2→3 request fixture,
not live wallet funds. Obtaining real wallet state, authorizing, signing, and
submitting transactions remains the wallet application's responsibility.

See `packages/zolana_mobile/README.md` for the ownership, lock/discard, completion,
and prepared-key lifetime contract. Closing drains work; it does not promise
instant native cancellation or secure erasure of all Dart/Go copies.

## Distribution checks

```sh
bash scripts/test-consumer.sh android
bash scripts/test-consumer.sh ios
ZOLANA_TEST_DEVICE=emulator-5554 bash scripts/test-consumer.sh device
```

These build a separate extracted-package consumer with fresh Dart dependency
resolution. The device check initializes the native bridge, generates a real
proof, checks safe errors, and closes a prover during a warm proof. CI configures
Android emulator and iOS simulator checks as well as consumer release builds.
