# Zolana Mobile SDK

Standalone Flutter bindings for Zolana proof-input primitives, Poseidon
hashing, and local Mopro/gnark Groth16 proving on Android and iOS.

## Layout

- `crates/zolana-mobile`: Zolana API plus the Mopro gnark adapter surface.
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

The Flutter-facing crate pins `mopro-ffi` and the Groth16-only gnark 0.15 backend
to immutable commits on `sergeytimoshin/mopro` branch
`feat/gnark-0.15-mobile`. Protocol crates `zolana-hasher`, `zolana-keypair`, and
`zolana-transaction` are pinned to upstream revision
`e6139f658c6961101d716e107a15a5ca9cecd143`.

## Current boundary

Mopro accepts a flattened decimal witness JSON. The demo commits one witness for
the 2→3 fixture; producing a live witness from a wallet transfer request remains
the application integration boundary. Constraint solving, proof generation, and
verification all run on-device.
