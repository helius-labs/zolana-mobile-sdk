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

`ZolanaWallet` runs the full private payment flow (register, deposit, sync,
transfer, withdraw) with every proof generated on the device and the Solana key
held by the application's signer. The example app has a wallet screen for a
Zolana test cluster and a local proof benchmark.

The generated native library keeps Mopro's internal
`mopro_flutter_bindings` stem. Applications import the public Dart package as
`package:zolana_mobile/zolana_mobile.dart`.

## Demo

Install Flutter, Rust, Go 1.27.1 or newer, and the platform toolchain. Then stage
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
`c1071f96fa5a28dca8e567dd3944331047450a2b`. The gnark backend from that revision
is vendored with local fixes documented in its provenance file so the pub package
includes its complete source build; it is built against gnark 0.16.3, the version
the Zolana prover uses. Protocol crates `zolana-hasher`, `zolana-keypair`, and
`zolana-transaction`, the vendored Go circuits, and the staged proving key are all
pinned to Zolana `main` revision `88c5cdc0e457a963580ced47f842d94901eb0e30`
(`[workspace.metadata.upstream]` in `Cargo.toml`).

`scripts/check-upstream.sh` fails if any of these drift from that revision; CI
runs it on every push. To move to a newer Zolana revision, re-vendor
`go/protocol` from `prover/server`, regenerate its `UPSTREAM_FILES.sha256`, bump
the Cargo pins and the key checksums in `stage-demo-assets.sh`, and regenerate
the 2→3 fixtures as described in the rust-gnark provenance file.

## Current boundary

`ZolanaWallet` builds, proves and submits transactions; the application signs
them. `LocalProver.proveRequest` remains for callers that assemble their own
`/prove` requests. The wallet is SOL only, keeps its state in memory, and does
not merge notes yet.

The end-to-end wallet test registers, deposits, proves a private transfer on
this machine and checks the recipient's balance. Zolana devnet runs the pinned
revision; the example's demo accounts avoid devnet airdrop limits once funded:

```sh
ZOLANA_E2E_RPC_URL=https://api.devnet.solana.com \
ZOLANA_E2E_INDEXER_URL=https://d2xah7tnhdhcom.cloudfront.net \
ZOLANA_E2E_SENDER_SEED=a0a60f24c56c18101be405cc2ddb750d88961c1ee781bd17cd174fe1f5ff55dd \
ZOLANA_E2E_RECIPIENT_SEED=7138835c906af341f4eec548684b5204d5b617b19573a0dd758050982601bb67 \
cargo test -p zolana-mobile --release --test wallet_flow -- --ignored --nocapture
```

A local cluster started by `just` in the zolana repository works as well.

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
