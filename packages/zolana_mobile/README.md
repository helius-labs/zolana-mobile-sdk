# Zolana Flutter SDK

A private Zolana wallet for Flutter that proves on the device, plus the local
Groth16 prover it is built on: Mopro's native gnark backend (gnark 0.16.3). Android
and iOS are supported. The package contains its Rust and Go sources; it does not
require a sibling Zolana checkout.

## Requirements

- Flutter with Dart 3.13.3 or newer, Rust stable, and Go 1.27.1 or newer.
- Android: an installed NDK. Set `ANDROID_NDK_HOME` and `ANDROID_NDK_ROOT` to its
  directory if the SDK installation cannot be discovered automatically.
- iOS: Xcode with its license accepted, CocoaPods, and the Rust iOS target.
- Matching `.r1cs`, `.pk`, and `.vk` files from a trusted, checksum-verified
  circuit release. Do not load keys supplied by an untrusted proof request.

The Dart runtime, Rust runtime, and generated Flutter Rust Bridge code are pinned
to **2.11.1** together. Do not override only one of these versions. Regenerate
bindings when upgrading them.

## Private wallet

`ZolanaWallet` runs the whole flow: registration, deposit, sync, private transfer
and withdrawal. It builds every transaction with the Zolana Rust client and wallet
crates and proves it on the device. The Solana secret key never enters the
package; your `SolanaSigner` (a platform keystore, a wallet adapter, a custodian)
signs two things:

1. once, the Zolana derivation message. The signature is the seed of the
   wallet's nullifier and viewing keys, which stay in memory for this session.
2. each transaction message, shown to the user with its `purpose` first.

```dart
class KeystoreSigner implements SolanaSigner {
  @override
  String get publicKey => /* base58 account */;

  @override
  Future<Uint8List> signMessage(Uint8List message, {required String purpose}) =>
      /* ask the user, then sign with Ed25519 */;
}

await initZolanaMobile();
final wallet = await ZolanaWallet.open(
  signer: KeystoreSigner(),
  config: WalletConfig(
    rpcUrl: rpcUrl,
    indexerUrl: indexerUrl,
    provingKeyDir: '${(await getApplicationSupportDirectory()).path}/keys',
    allowInsecureHttp: false,
  ),
);
await wallet.register();               // once, so others can pay this wallet
await wallet.deposit(BigInt.from(500000000));
await wallet.sync();
await wallet.transfer(recipient: registeredAccount, lamports: BigInt.from(100000000));
```

- **Proving keys** download on first use from the Zolana key host into
  `provingKeyDir`, and are used only after they match the proving-key lockfile of
  the pinned Zolana revision. The wallet picks the circuit shape, so the first
  transfer of a new shape downloads its key (8–240 MB); keep the directory
  across launches.
- **Recipients** are Solana accounts that have registered. A transfer to an
  unregistered account fails with `recipient_not_registered`; use `withdraw`
  for a public payment.
- **Privacy**: deposits and withdrawals are public. Private transfers reveal
  neither amount nor recipient. The indexer learns this wallet's view tags, so
  `allowInsecureHttp` is only for a local test cluster.
- **Freshness**: a transfer or withdrawal syncs before it selects notes, and a
  confirmed transaction is synced before `transfer`, `deposit` or `withdraw`
  returns, so notes spent by another session or device are never picked.
- **Errors** are `ZolanaWalletException`s with a code or a client error
  description, never key material.

Current limits: SOL only; wallet state is in memory, so reopen and `sync` after
a restart; notes are not merged; one prepared prover is loaded per process, so
close a `LocalProver` before the wallet proves.

## Prepare once, prove repeatedly

```dart
import 'package:zolana_mobile/zolana_mobile.dart';

await initZolanaMobile();
final prover = await LocalProver.load(
  r1csPath: circuitPath,
  provingKeyPath: provingKeyPath,
  verifyingKeyPath: verifyingKeyPath,
);
try {
  final job = prover.proveRequest(requestJson);
  final result = await job.result;
  useProofIfWalletSessionIsStillAuthorized(result.proofJson);
} finally {
  await prover.close();
}
```

`requestJson` is a structured Zolana `/prove` request built from real wallet
state, not a seed or an invented input balance. The adapter builds its witness
locally and returns the canonical `{ar, bs, krs}` proof JSON consumed by Zolana.
It verifies the proof locally before returning. The SDK does not acquire wallet
state, authorize a transaction, sign, or submit it; `ZolanaWallet` does all of
that except signing.

The structured adapter supports `transfer-confidential`, `transfer-ring`,
`transfer-ring-authority`, `merge`, and `merge-ring`, with a matching supported
key shape. Other circuit types fail closed. The bundled demo stages only 2→3;
provide the matching trusted keys for other shapes.

`proveWitness` is the lower-level API for an already-flattened witness: an exact
map of circuit variable names to canonical, non-negative decimal **strings**
below the BN254 scalar modulus. Numbers, duplicates, unknown/missing fields, and
non-canonical encodings are rejected rather than rounded or reduced.

Only one prepared native prover is loaded at a time per process, and each
`LocalProver` accepts one active job. Release it before loading another circuit.
The key/circuit files are deserialized once by `load`, not on every warm proof.
`loadMs` measures preparation separately from job timings. Parsed keys are held
until `close`; deleting or replacing their source files does not rotate an
already-loaded prover. Close and reload explicitly to change circuit versions.

## Wallet lock and cleanup

```dart
wallet.invalidateSession();
await prover.close();
```

The wallet must invalidate its own authorization/session immediately on lock.
`close()` synchronously rejects new jobs and marks the active result discarded.
Its future waits for native work to finish, then releases the prepared keys.
It is idempotent; a failed release can be retried.

`job.discard()` discards only that pending result; it does **not** stop native
computation. `job.done` settles after native work finishes, on success or error.
`job.result` rejects with `ProverErrorCode.discarded` when discarded while active.
Already-delivered results cannot be revoked: always check the current wallet
session/approval immediately before accepting a proof or signing/submitting.

Inputs are copied across Dart, Rust, and Go. Do not pass borrowed pointers to
wallet-owned memory. You may clear caller-owned buffers once you no longer need
them, but that does not erase copies already passed to the prover. Dart strings
and Go/FFI temporaries cannot provide a complete secure-erasure guarantee.
Completion and `close()` mean no more SDK work uses the job; they do not prove
that every secret byte has been overwritten. Immediate native cancellation and
guaranteed memory erasure on lock are not supported.

Keep private witnesses in memory. The example uses a public request fixture,
writes only public circuit/key files to a unique temporary directory, and drains
the prover before deleting those files. Error codes contain no witness values;
do not add raw native errors or witness JSON to application logs.

## Low-level and demo APIs

`generateGnarkProof`, `verifyGnarkProof`, and `proveAssignment` remain available
for compatibility. They are one-shot operations without the Dart job/session
contract. `proveAssignment` reads a caller-owned witness file and does not delete
it. Prefer `LocalProver` for wallet integration.

`poseidonHash` exposes the Rust Poseidon primitive.

## Example and validation

The example app is a private wallet on Zolana devnet with two built-in demo
accounts (their keys are public in `example/lib/demo_keys.dart`; devnet SOL only)
and a local proof benchmark. Stage the benchmark's key and run it:

```sh
./scripts/stage-demo-assets.sh
cd packages/zolana_mobile/example
flutter pub get
flutter run --release -d YOUR_DEVICE_ID --dart-define=ZOLANA_API_KEY=...
```

`ZOLANA_API_KEY` is optional: with a Helius key the Solana RPC is Helius devnet,
without one it is the public devnet endpoint. It is compiled into that build
only; never commit it.

### From Xcode

Install Go 1.27.1 or newer (`brew install go`) and Flutter; the pod builds the
Rust and Go sources and finds Go outside Xcode's `PATH` (or set `GO`). Then, with
the workspace closed:

```sh
cd packages/zolana_mobile/example
flutter pub get
flutter build ios --config-only --simulator [--dart-define=ZOLANA_API_KEY=...]
open ios/Runner.xcworkspace
```

Open the workspace, not `Runner.xcodeproj`, pick the Runner scheme and a
simulator, and run. Rerun `--config-only` after changing a define. If Xcode then
reports a missing `FlutterGeneratedPluginSwiftPackage`, it resolved packages
while Flutter was regenerating them: use File → Packages → Reset Package Caches.
Xcode build logs contain the defines base64-encoded; do not share logs from a
build with an API key.

From the repository root, `bash scripts/test-consumer.sh android` extracts the
package into a separate temporary consumer and builds it with fresh pub
resolution. `ZOLANA_TEST_DEVICE=emulator-5554 bash scripts/test-consumer.sh device`
also runs initialization, a real proof, sanitized failure, and lock/drain tests.
