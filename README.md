# Zolana Mobile SDK

Standalone Flutter bindings for Zolana transaction construction, Poseidon
hashing, and local Arkworks/Groth16 proving on Android and iOS.

## Layout

- `crates/zolana-groth16-gnark`: Rust prover for Zolana gnark keys.
- `crates/zolana-mobile`: narrow Rust API exposed to Flutter.
- `packages/zolana_mobile`: generated Flutter plugin and proof-only example.
- `fixtures`: committed 2→3 proof request and solved assignment fixture.
- `scripts`: reproducible asset staging and assignment regeneration.

The transaction API remains available from the package, while the example UI
intentionally exposes only local proof generation.

The generated native library keeps Mopro's internal
`mopro_flutter_bindings` stem. Applications import the public Dart package as
`package:zolana_mobile/zolana_mobile.dart`.

## Demo

Install Flutter and the platform toolchain, then stage the ignored proving key:

```sh
./scripts/stage-demo-assets.sh
cd packages/zolana_mobile/example
flutter pub get
flutter run
```

`stage-demo-assets.sh` verifies the proving key checksum and copies the key and
committed assignment fixture into the example assets. To recreate the assignment
from `fixtures/prove-request-2x3.json`, run:

```sh
./scripts/regenerate-assignment.sh
```

The regeneration script checks out the pinned upstream Zolana revision in a
temporary directory and invokes its Go/gnark solver. It does not depend on a
neighboring monorepo checkout.

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
cargo test -p zolana-mobile proves_and_verifies_staged_assignment -- --ignored
```

## Rust dependencies

The Flutter-facing crate uses the local prover crate. Protocol crates
`zolana-hasher`, `zolana-keypair`, and `zolana-transaction` are pinned to upstream
revision `e6139f658c6961101d716e107a15a5ca9cecd143` for reproducibility.

## Current boundary

The local prover accepts a solved gnark assignment. Assignment solving remains a
Go/gnark build-time fixture step; proving and verification run entirely on-device.
