# Zolana Mobile SDK

A Flutter SDK for private Zolana payments on Android and iOS. `ZolanaWallet`
registers, shields, syncs, sends privately and unshields, building every
transaction with the Zolana Rust client and proving it on the device with
Mopro/gnark Groth16. The Solana key stays with the application's signer.

## Layout

- `packages/zolana_mobile/native/zolana-mobile`: the native wallet, on-device
  prover, proving-key store, and lower-level prover handles.
- `packages/zolana_mobile/native/rust-gnark`: vendored Mopro backend, including
  strict witness validation, prepared keys, `.key` loading, and the structured
  request adapter.
- `packages/zolana_mobile`: the Flutter plugin, and an example devnet wallet.
- `fixtures`: a 2→3 `/prove` request captured from the Zolana client, and its
  flattened witness.
- `scripts`: asset staging and the upstream drift check.

The generated native library keeps Mopro's internal
`mopro_flutter_bindings` stem. Applications import the public Dart package as
`package:zolana_mobile/zolana_mobile.dart`.

## Demo

The example is a private wallet on Zolana devnet with two built-in demo accounts,
A and B, so one phone can pay itself privately. Their keys are public in
`example/lib/demo_keys.dart`: fund them with devnet SOL only, for example from
faucet.solana.com. The first send downloads the proving key for its shape into
app storage; later sends reuse it.

Install Flutter, Rust, Go 1.27.1 or newer, and the platform toolchain. Then:

```sh
./scripts/stage-demo-assets.sh   # the proof benchmark's 2→3 key
cd packages/zolana_mobile/example
flutter pub get
flutter run --release -d YOUR_DEVICE_ID --dart-define=ZOLANA_API_KEY=...
```

To send privately from A to B on a device or simulator, proving there:

```sh
flutter test integration_test/devnet_wallet_test.dart -d DEVICE \
  --dart-define=ZOLANA_E2E=true
```

`ZOLANA_API_KEY` is optional: a Helius key selects Helius devnet for Solana RPC,
without one the public devnet endpoint is used. It is compiled into that build
only; do not commit it.

Android builds compile the gnark bridge from source and need the NDK version
Flutter selects (28.2.13676358 for Flutter 3.47), installed through Android
Studio's SDK Manager:

```sh
export ANDROID_NDK_HOME="$HOME/Library/Android/sdk/ndk/28.2.13676358"
export ANDROID_NDK_ROOT="$ANDROID_NDK_HOME"
```

`stage-demo-assets.sh` downloads one checksum-locked packed Zolana key and
splits its `.pk`, `.vk`, and `.r1cs` sections at verified offsets for the
benchmark. The wallet itself downloads upstream `.key` files on demand and uses
them unsplit.

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
pinned to one Zolana revision, `[workspace.metadata.upstream]` in `Cargo.toml`,
which must be on Zolana `main`.

`scripts/check-upstream.sh` fails if any of these drift from that revision; CI
runs it on every push. To move to a newer Zolana revision, re-vendor
`go/protocol` from `prover/server`, regenerate its `UPSTREAM_FILES.sha256`, bump
the Cargo pins and the key checksums in `stage-demo-assets.sh`, and regenerate
the 2→3 fixtures as described in the rust-gnark provenance file.

## Current boundary

`ZolanaWallet` builds and proves transactions; the application signs them, and
either the wallet or the application sends them. `LocalProver.proveRequest`
remains for callers that assemble their own `/prove` requests. The wallet holds
SOL and the SPL tokens the shielded pool has registered, keeps its state in
memory, and does not merge notes yet.

The end-to-end wallet test registers, deposits, proves a private transfer on
this machine, checks the recipient's balance, and unshields from a second,
stale session of the sender. Zolana devnet runs the pinned revision; the demo
accounts avoid devnet airdrop limits once funded:

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
