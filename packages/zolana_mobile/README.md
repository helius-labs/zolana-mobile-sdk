# Zolana Flutter SDK

A private Zolana wallet for Flutter that proves on the device, plus the local
Groth16 prover it is built on: Mopro's native gnark backend (gnark 0.16.3). Android
and iOS are supported. The package contains its Rust and Go sources; it does not
require a sibling Zolana checkout.

## Requirements

- Flutter with Dart 3.13.0 or newer. CI checks Flutter 3.47.2 (Dart 3.13.2) and
  3.47.4.
- Android: the NDK named by the app's `android.ndkVersion`.
- iOS: Xcode with its license accepted and CocoaPods.
- Android API 24 or newer and iOS 15 or newer, the Flutter minimums. The Go
  prover archive targets iOS 13. CI runs the clean consumer on Android API 26
  and 35 emulators, and checks that every object in the iOS native library runs
  on iOS 16.
- Matching `.r1cs`, `.pk`, and `.vk` files from a trusted, checksum-verified
  circuit release. Do not load keys supplied by an untrusted proof request.

Rust and Go are needed only for a build from source, see
[Native libraries](#native-libraries).

The Dart runtime, Rust runtime, and generated Flutter Rust Bridge code are pinned
to **2.11.1** together. Do not override only one of these versions. Regenerate
bindings when upgrading them.

## Native libraries

The plugin downloads precompiled native libraries from the GitHub releases of
this repository:

- Android: `arm64-v8a`, `armeabi-v7a` and `x86_64`.
- iOS: device (`arm64`) and simulator (`arm64`, `x86_64`).

Each file is signed. Cargokit checks the signature with the Ed25519 public key
in `rust/cargokit.yaml` and ignores a file with an invalid signature.

The release name contains a hash of the Rust and Go sources of the plugin. If no
release matches the sources, for example after a local change, the plugin builds
the library from source. A build from source needs:

- Rust stable with the target of the platform, and Go 1.27.1 or newer.
- Android: an installed NDK. Set `ANDROID_NDK_HOME` and `ANDROID_NDK_ROOT` to its
  directory if the SDK installation cannot be discovered automatically.

To always build from source, add `cargokit_options.yaml` to the app directory:

```yaml
use_precompiled_binaries: false
```

## Private wallet

`ZolanaWallet` runs the whole flow: registration, deposit, private transfer and
withdrawal. It builds every transaction with the Zolana Rust client and proves
it on the device. The Solana secret key never enters the
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
    mints: const [],                  // SPL mints balances() lists
  ),
);
await wallet.register();               // once, so others can pay this wallet
await wallet.deposit(BigInt.from(500000000));
await wallet.transfer(recipient: registeredAccount, amount: BigInt.from(100000000));
```

- **Proving keys** download on first use from the Zolana key host into
  `provingKeyDir`, and are used only after they match the proving-key lockfile of
  the pinned Zolana revision. The wallet picks the circuit shape, so the first
  transfer of a new shape downloads its key (8–240 MB); keep the directory
  across launches.
- **Tokens**: pass `mint` (base58) for an SPL Token or Token-2022 asset; no
  `mint` is SOL. Amounts are in base units. A mint works once the shielded pool
  has registered it, otherwise calls fail with `asset_not_supported`.
  `balances()` lists the private balance of SOL and of each mint in
  `WalletConfig.mints` or named in a call; notes in other mints are left out.
  A token withdrawal
  goes to the recipient's associated token account: when it fails with
  `recipient_token_account_missing`, `prepareTokenAccount` creates the account.
  Transaction fees are paid in SOL by this account.
- **RPC headers**: `WalletConfig.rpcHeaders` adds HTTP headers to every Solana
  RPC request, for example an auth token for your RPC proxy. The values are
  marked sensitive, so they are not printed in logs. The indexer does not take
  custom headers yet.
- **Registration**: `registrationStatus()` is `notRegistered`, `registered` or
  `conflict`. A conflict means the account's user record holds other keys: set
  by another app, by a key rotation, or derived by a signer whose signatures
  change between calls. `register()` then fails with `registration_conflict` and
  never replaces them. The wallet can still spend and withdraw its own notes.
- **Recipients** are Solana accounts that have registered. A transfer to an
  unregistered account fails with `recipient_not_registered`; use `withdraw`
  for a public payment.
- **Privacy**: deposits and withdrawals are public. Private transfers reveal
  neither amount nor recipient. The indexer learns this wallet's view tags, so
  `allowInsecureHttp` is only for a local test cluster.
- **History**: `activity()` lists the transactions that moved this wallet's
  notes, newest first, as `shielded`, `unshielded`, `sent`, `received` or
  `internal` (merges and transfers to itself), one entry per asset. The indexer
  does not mark withdrawals: a spend whose outputs are all this wallet's own is
  `unshielded`, one with another wallet's output `sent`.
- **Freshness**: the wallet keeps no chain state. `balances()`,
  `privateBalance()`, `activity()` and every spend read the wallet's notes from
  the indexer when they run, and `transfer`, `deposit` and `withdraw` return once the indexer
  has the transaction, so notes spent by another session or device are never
  picked.
- **Errors** are `ZolanaWalletException`s with a code or a client error
  description, never key material.

- **Lock and account switch**: call `wallet.close()`. Operations not yet
  started fail with `wallet_closed`, and so does a `prepare` call that is
  proving at the time: the application never receives a transaction after the
  lock. Nothing is signed or submitted after the call, so lock the UI without
  awaiting `close()`. It waits for the running native step (a proof cannot be
  interrupted) and for an open signer prompt, so cancel your prompt on lock.
  Then it releases the native wallet: its keys and the proving key it loaded.
  Open the next account after `close()` completes. A transaction already
  submitted is not recalled.

### Signing and sending in the application

`register`, `deposit`, `transfer` and `withdraw` prepare, sign with the
`SolanaSigner` and submit. An application that approves, signs and sends
transactions itself uses the `prepare` methods and reports back:

```dart
final tx = await wallet.prepareTransfer(recipient: account, amount: amount);
// Show tx.summary, sign tx.message with each of tx.signers, send it.
await wallet.confirm(tx, signature); // base58 transaction signature
```

`confirm` checks that `signature` is the fee payer's signature over
`tx.message`, waits until Solana confirms it and, for shielded-pool
transactions, until the indexer has it. `submit(tx, signatures)`
sends through the wallet's RPC instead.

- `prepareTransfer` and `prepareWithdrawal` take an optional `feePayer`, for
  example the application's backend. It pays the network fee instead of
  this account and signs first: `tx.signers` is `[feePayer, owner]`, and the
  fee payer's signature is the transaction signature. The proof binds the fee
  payer, so choose it at prepare. `transfer` and `withdraw` always use this
  account.
- The message carries a recent blockhash and expires after about 150 blocks
  (60–90 s). Prepare again when it expires.
- Prepare one spend at a time. Until a prepared spend is confirmed, the next
  one can select the same notes; the program then rejects the second
  transaction. No funds are lost.

Current limits: notes are not merged, so a spend takes at most 5 notes on one
tree and fails with `merge_required` beyond that; one prepared prover is
loaded per process, so close a `LocalProver` before the wallet proves.

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

Load the native library with `initZolanaMobile()`, not `RustLib.init()`. On iOS
the pod links the Rust library into the plugin framework, which the default
loader does not look in.

The structured adapter supports `transfer-confidential`, `transfer-ring`,
`transfer-ring-authority`, `merge`, and `merge-ring`, with a matching supported
key shape. Other circuit types fail closed. The bundled demo stages only 2→3;
provide the matching trusted keys for other shapes.

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

## Poseidon

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

`ZOLANA_API_KEY` is a Helius key: the example reaches Solana RPC and the Zolana
indexer on Helius devnet with it. It is compiled into that build only; never
commit it.

### From Xcode

Install Go 1.27.1 or newer (`brew install go`) and Flutter; the pod builds the
Rust and Go sources and finds Go outside Xcode's `PATH` (or set `GO`). Then, with
the workspace closed:

```sh
cd packages/zolana_mobile/example
flutter pub get
flutter build ios --config-only --simulator --dart-define=ZOLANA_API_KEY=...
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
