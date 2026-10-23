# Zolana Mobile SDK

A Flutter SDK for private Zolana payments on Android and iOS. `ZolanaWallet`
registers, shields, sends privately and unshields, building every
transaction with the Zolana Rust client and proving it on the device with
zopro, a Groth16 prover in Rust for gnark's keys (on the GPU where there is
one), or through the application's backend with the proof verified on the
device. The Solana key stays with the application's signer.

## Layout

- `packages/zolana_mobile/native/zolana-mobile`: the native wallet, the
  on-device prover and the proving-key store.
- `packages/zolana_mobile`: the Flutter plugin, and an example devnet wallet.
- `fixtures`: a 2→2 `/prove` request built with the Zolana prover's test
  helpers, and the Helius prover's response to it.
- `scripts`: asset staging.

The generated native library keeps Mopro's internal
`mopro_flutter_bindings` stem. Applications import the public Dart package as
`package:zolana_mobile/zolana_mobile.dart`.

## Demo

The example is a private wallet on Zolana devnet with two built-in demo accounts,
A and B, so one phone can pay itself privately. Their keys are public in
`example/lib/demo_keys.dart`: fund them with devnet SOL only, for example from
faucet.solana.com. The wallet screen switches between SOL and TEST, a devnet
SPL Token mint the demo accounts hold 1,000 of (`example/lib/demo_assets.dart`;
account A is its mint authority). The first send downloads the proving key for
its shape into app storage; later sends reuse it.

Install Flutter and the platform toolchain. Then:

```sh
./scripts/stage-demo-assets.sh   # the proof benchmark's 2→2 key
cd packages/zolana_mobile/example
flutter pub get
flutter run --release -d YOUR_DEVICE_ID --dart-define=ZOLANA_API_KEY=...
```

To send privately from A to B on a device or simulator, proving there, merge
B's notes, and shield, send and unshield the demo token:

```sh
flutter test integration_test/devnet_wallet_test.dart -d DEVICE \
  --dart-define=ZOLANA_E2E=true --dart-define=ZOLANA_API_KEY=...
```

`ZOLANA_API_KEY` is a Helius key: the example reaches Solana RPC and the Zolana
indexer on Helius devnet with it. It is compiled into that build only; do not
commit it.

The build uses the precompiled native libraries (see below). When no release
matches the sources, it builds them from source with Rust. Android source builds need the NDK version Flutter selects (28.2.13676358
for Flutter 3.47), installed through Android Studio's SDK Manager. Set both
variables to its directory before running Flutter:

```sh
export ANDROID_NDK_HOME="$HOME/Library/Android/sdk/ndk/28.2.13676358"
export ANDROID_NDK_ROOT="$ANDROID_NDK_HOME"
```

`stage-demo-assets.sh` downloads the benchmark's checksum-locked zopro key.
The wallet itself downloads the zopro key of each shape on demand.

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
cargo test -p zolana-mobile proves_the_staged_request -- --ignored
```

## Rust dependencies

The Flutter-facing crate pins `mopro-ffi` to `sergeytimoshin/mopro` revision
`c1071f96fa5a28dca8e567dd3944331047450a2b`. The prover is `zopro-prover`: the
request to the circuit's inputs, the arithmetic, and the proof verified and
encoded as gnark's verifier reads it; gnark itself runs only offline, where
the circuits are compiled and the keys made. Protocol crates `zolana-hasher`,
`zolana-keypair`, and `zolana-transaction` are pinned to one Zolana revision,
which must be on Zolana `main`.

## Precompiled binaries

The `Precompiled binaries` workflow runs on every push to `main`:

1. It computes the crate hash of `packages/zolana_mobile/rust` and its path
   dependency `native/zolana-mobile`.
2. It builds the library for Android (`aarch64`, `armv7`, `x86_64`) and iOS
   (device, `arm64` and `x86_64` simulator).
3. It signs every file and uploads it to the release `precompiled_<hash>`.
   Targets that the release already has are skipped.
4. It builds the example app for Android and iOS without Rust, from the
   uploaded files only.

The signing key is the repository secret `CARGOKIT_PRIVATE_KEY`, 64 bytes as
hex. Its public key is pinned in `packages/zolana_mobile/rust/cargokit.yaml`. To
rotate the key, run `dart run build_tool gen-key` in
`packages/zolana_mobile/cargokit/build_tool`, then replace the secret and the
public key together. Releases signed with the old key stop verifying.

## Current boundary

`ZolanaWallet` builds and proves transactions; the application signs them, and
either the wallet or the application sends them. `LocalProver.prove`
remains for callers that assemble their own `/prove` requests. The wallet holds
SOL and the SPL tokens the shielded pool has registered, keeps its state in
memory, and merges notes on request.

The end-to-end wallet tests register, deposit, prove a private transfer on this
machine, check the recipient's balance, and unshield from a second, stale
session of the sender; merge notes; and deposit, transfer and withdraw an SPL
token. Zolana devnet runs the pinned revision; the demo accounts avoid devnet
airdrop limits once funded. `ZOLANA_E2E_MINT` names the token: the demo token
`Y8FDajuDMQWu7d5ES3jbZ67NzZta266WuFgQ92N4yd6`, which account A mints; without
it the test creates a mint and registers it with the shielded pool:

```sh
ZOLANA_E2E_RPC_URL="https://beta-devnet.helius-rpc.com/?api-key=$API_KEY" \
ZOLANA_E2E_INDEXER_URL="https://beta-devnet.helius-rpc.com/v1/zolana?api-key=$API_KEY" \
ZOLANA_E2E_PROVER_URL="https://beta-devnet.helius-rpc.com/v1/zolana?api-key=$API_KEY" \
ZOLANA_E2E_SENDER_SEED=a0a60f24c56c18101be405cc2ddb750d88961c1ee781bd17cd174fe1f5ff55dd \
ZOLANA_E2E_RECIPIENT_SEED=7138835c906af341f4eec548684b5204d5b617b19573a0dd758050982601bb67 \
ZOLANA_E2E_MINT=Y8FDajuDMQWu7d5ES3jbZ67NzZta266WuFgQ92N4yd6 \
cargo test -p zolana-mobile --release --test wallet_flow -- --ignored --nocapture --test-threads 1
```

A local cluster started by `just` in the zolana repository works as well.

See `packages/zolana_mobile/README.md` for the ownership, lock/discard, completion,
and prepared-key lifetime contract. Closing drains work; it does not promise
instant native cancellation or secure erasure of all Dart copies.

## Distribution checks

```sh
bash scripts/test-consumer.sh android
bash scripts/test-consumer.sh ios
ZOLANA_TEST_DEVICE=emulator-5554 bash scripts/test-consumer.sh device
```

These build a separate extracted-package consumer with fresh Dart dependency
resolution. The device check initializes the native bridge, generates a real
proof, checks safe errors, and closes a prover during a warm proof. CI runs the
device check on Android emulators and an iOS simulator.
