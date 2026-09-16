# Zolana Flutter SDK

Local Groth16 proving through Mopro's gnark 0.15 native backend, with typed Dart
bindings to Zolana Rust primitives. Android and iOS are supported. The package
contains its Rust and Go sources; it does not require a sibling Zolana checkout.

## Requirements

- Flutter with Dart 3.13.3 or newer, Rust stable, and Go 1.25.7 or newer.
- Android: an installed NDK. Set `ANDROID_NDK_HOME` and `ANDROID_NDK_ROOT` to its
  directory if the SDK installation cannot be discovered automatically.
- iOS: Xcode with its license accepted, CocoaPods, and the Rust iOS target.
- Matching `.r1cs`, `.pk`, and `.vk` files from a trusted, checksum-verified
  circuit release. Do not load keys supplied by an untrusted proof request.

The Dart runtime, Rust runtime, and generated Flutter Rust Bridge code are pinned
to **2.11.1** together. Do not override only one of these versions. Regenerate
bindings when upgrading them.

## Prepare once, prove repeatedly

```dart
import 'package:zolana_mobile/zolana_mobile.dart';

await RustLib.init();
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
state, authorize a transaction, sign, or submit it. Keep those operations in the
wallet's existing Zolana transaction flow.

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

`shieldedAddress` and `poseidonHash` expose Rust primitives. `prepareTransfer`
creates an **offline synthetic draft**, not a spendable transaction or a witness
for real wallet funds. Do not use it to authorize a payment.

## Example and validation

The repository includes the Flutter example and checksum-pinned asset staging:

```sh
./scripts/stage-demo-assets.sh
cd packages/zolana_mobile/example
flutter pub get
flutter run --release -d YOUR_DEVICE_ID
```

From the repository root, `bash scripts/test-consumer.sh android` extracts the
package into a separate temporary consumer and builds it with fresh pub
resolution. `ZOLANA_TEST_DEVICE=emulator-5554 bash scripts/test-consumer.sh device`
also runs initialization, a real proof, sanitized failure, and lock/drain tests.
