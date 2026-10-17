# Zolana Flutter SDK

A private Zolana wallet for Flutter that proves on the device, plus the local
Groth16 prover it is built on: Mopro's native gnark backend (gnark 0.16.3). Android
and iOS are supported. The package contains its Rust and Go sources; it does not
require a sibling Zolana checkout.

**Performance.** The native prover at four threads on an Apple M5 Pro proves a warm spend (key loaded) in 0.17 s for one note to two outputs, 0.27 s for two, 0.55 s for four, 0.92 s for eight, 1.7 s for sixteen and 2.7 s for twenty-four; a restart adds the key's load from its mapped image (tens of milliseconds there), and the first run its download (9 MB to 180 MB). On phones, in release builds, the same proofs take 0.34 s, 0.55 s, 0.99 s, 1.9 s, 3.6 s and 5.8 s on an iPhone 12, and 1.2 s, 2.0 s, 3.6 s, 7.1 s, 15 s and 25 s on a Samsung Galaxy A16; a key loads from its image in 12 ms to 0.25 s on the iPhone and 55 ms to 0.6 s on the A16. CI benches release builds on a Samsung Galaxy A16 and an iPhone 12 for every pull request and keeps the history of `main` on the [`proof-timing`](https://github.com/helius-labs/zolana-mobile-sdk/tree/proof-timing) branch; "Proof timing" below describes the harness.

## Requirements

- Flutter with Dart 3.13.0 or newer. CI checks Flutter 3.47.2 (Dart 3.13.2) and
  3.47.4.
- Android: the NDK named by the app's `android.ndkVersion`.
- iOS: Xcode with its license accepted. The app may use Swift Package Manager
  or CocoaPods.
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

- Android: `arm64-v8a`, `armeabi-v7a` and `x86_64`, aligned for 16 KB memory
  pages as Google Play requires. Builds from source are aligned too, with any
  NDK. CI checks the alignment of the APK built from the released libraries
  and of one built from source.
- iOS: device (`arm64`) and simulator (`arm64`, `x86_64`), static libraries. The
  package's build hook links the library into `mopro_flutter_bindings.framework`,
  which Flutter bundles with the app.

Each file is signed. Cargokit checks the signature with the Ed25519 public key
in `rust/cargokit.yaml` and ignores a file with an invalid signature.

The release name contains a hash of the Rust and Go sources of the plugin. If no
release matches the sources, for example after a local change, the plugin builds
the library from source. A build from source needs:

- Rust stable with the target of the platform, and Go 1.27.1 or newer.
- Android: an installed NDK. Set `ANDROID_NDK_HOME` and `ANDROID_NDK_ROOT` to its
  directory if the SDK installation cannot be discovered automatically.
- iOS: rustup's default `~/.cargo` and `~/.rustup`. Flutter runs the build hook
  with few environment variables, without `CARGO_HOME` and `RUSTUP_HOME`.

To always build from source, add `cargokit_options.yaml` to the app directory:

```yaml
use_precompiled_binaries: false
```

iOS builds read a new options file after `flutter clean`.

## Private wallet

`ZolanaWallet` runs the whole flow: registration, deposit, private transfer and
withdrawal. It builds every transaction with the Zolana Rust client and proves
it on the device, or through the application's backend (see
[Backend proving](#backend-proving)). The Solana secret key never enters the
package; your `SolanaSigner` (a platform keystore, a wallet adapter, a custodian)
signs two things:

1. once, the Zolana derivation message. The signature is the seed of the
   wallet's nullifier and viewing keys, which stay in memory for this session.
   A wallet opened from saved keys (see below) skips this.
2. each prepared transaction's message, with the `PreparedTransaction` to show
   the user first: its `operation`, what it moves and to whom, and its
   `feePayer`.

```dart
class KeystoreSigner implements SolanaSigner {
  @override
  String get publicKey => /* base58 account */;

  @override
  Future<Uint8List> signMessage(
    Uint8List message, {
    PreparedTransaction? transaction, // null for the derivation message
  }) => /* show the transaction, ask the user, then sign with Ed25519 */;
}

await initZolanaMobile();
final wallet = await ZolanaWallet.open(
  signer: KeystoreSigner(),
  config: WalletConfig(
    rpcUrl: rpcUrl,
    indexerUrl: indexerUrl,
    provingKeyDir: '${(await getApplicationSupportDirectory()).path}/keys',
    mints: const [],                  // SPL mints the wallet holds, see Tokens
  ),
);
await wallet.register();               // once, so others can pay this wallet
await wallet.deposit(BigInt.from(500000000));
await wallet.transfer(
  payments: [Payment(recipient: registeredAccount, amount: BigInt.from(100000000))],
);
```

- **Payments**: a transfer takes one or more payments, each to a registered
  account, and proves them together. An application's own fee is a second
  payment to its collector: the chain sees neither amount nor recipient.

  ```dart
  await wallet.transfer(payments: [
    Payment(recipient: registeredAccount, amount: amount),
    Payment(recipient: feeCollector, amount: fee),
  ]);
  ```

  Two payments and the change prove with 4-output keys, so the notes that
  pay them must number 24 or fewer instead of 40; past that the transfer
  fails with `WalletError.mergeRequired`. The prepared transfer's
  `Operation_Transfer` lists each payment.
- **Public fee**: a deposit or a withdrawal is public, so its fee is a public
  transfer of the same asset from the account to your collector. Pass it as
  `publicFee` and the wallet adds it to the same transaction, after the
  shielded-pool instruction. The deposit or withdrawal and the fee both land,
  or neither does.

  ```dart
  await wallet.deposit(
    amount,
    publicFee: Payment(recipient: publicCollector, amount: fee),
  );
  await wallet.withdraw(
    recipient: destination,
    amount: amount,
    publicFee: Payment(recipient: publicCollector, amount: fee),
  );
  ```

  `prepareDeposit` and `prepareWithdrawal` take it too, and show it in
  `Operation_Deposit.fee` and `Operation_Withdrawal.fee`, so the approval
  screen can name it. The account pays the fee from its public balance and
  signs it with the same signature. A withdrawal to the account itself has
  delivered its funds before the fee runs, so the fee can come out of them.
  Before anything is proved or sent, a zero fee fails with
  `WalletError.amountZero`, a collector that is not a public key with
  `WalletError.invalidPubkey`, and a token fee whose collector has no
  associated token account with `WalletError.recipientTokenAccountMissing`,
  which names the collector. A SOL fee to an account that does not exist yet
  must cover its rent. Your application calculates the fee and holds the
  collector address. The proof does not cover the fee. It adds accounts to the
  message, so a spend of the widest shape may not have room for it: the call
  then fails while building the message, and you can merge first.

- **Proving keys** download from the Zolana key host into `provingKeyDir`
  (through the transport, see [Networking](#networking)), and are used only
  after they match the proving-key lockfile of the pinned Zolana revision.
  The wallet picks the circuit shape, so the first spend of a new shape
  downloads its key, 9 MB for one note and up to 380 MB for the widest spends
  and merges, unless it is already on the device:

  ```dart
  // On Wi-Fi after onboarding: spends of up to four notes, about 80 MB.
  wallet.prefetchProvingKeys(maxInputs: 4).listen(
    (update) => showProgress(update.downloaded, update.total),
  );
  ```

  `prefetchProvingKeys` runs beside the wallet's other calls; a spend that
  needs a key it is downloading waits for it. Cancel its subscription to stop
  it. `provingKeys()` lists each key and how much of it is on the device,
  and `clearProvingKeys()` removes them. Keys download in parts of 8 MB: an
  interrupted download resumes where it stopped, and with the default
  transport a part that receives no data for 30 s fails with
  `WalletError.provingKeyDownloadFailed`. Keep the directory across launches:
  the keys of one lockfile live in a directory of their own there, and the
  wallet removes the directories of other lockfiles when it opens. A key
  that was checked once is not hashed again unless its file changed. After
  its first load, the key's proving key is kept beside it in the device's
  memory layout (`<key>.image`, about twice the key's size): later loads map
  that file instead of decompressing the key, and the system can page the
  key out under memory pressure instead of ending the app.
  `clearProvingKeys()` removes the images with the keys.
- **Tokens**: list each SPL Token or Token-2022 mint the wallet holds in
  `WalletConfig.mints`, with the token program that owns it, as your Solana
  SDK reads it:

  ```dart
  mints: [
    MintConfig(
      mint: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v',
      tokenProgram: 'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA',
    ),
  ],
  ```

  The wallet reads only the asset id the shielded pool registered for each.
  Then pass `mint` (base58) to a call; no `mint` is SOL. Amounts are in base
  units. A mint that is not in the config fails with
  `WalletError.mintNotConfigured`, one the pool has not registered with
  `WalletError.assetNotSupported`, and a token program that is neither SPL
  Token nor Token-2022 fails `open` with `WalletError.invalidTokenProgram`.
  `balances()` lists the private balance of SOL and of each configured mint;
  notes in other mints are left out.
  A token withdrawal goes to the recipient's associated token account and
  fails with `WalletError.recipientTokenAccountMissing`, before proving, when
  the account does not exist. Transaction fees are paid in SOL by this
  account.
- **Public accounts**: read public SOL and token balances, and create
  associated token accounts, with your application's own Solana client.
- **Registration**: `registrationStatus()` is `notRegistered`, `registered` or
  `conflict`. A conflict means the account's user record holds other keys: set
  by another app, by a key rotation, or derived by a signer whose signatures
  change between calls. `register()` then fails with
  `WalletError.registrationConflict` and never replaces them. The wallet can
  still spend and withdraw its own notes.
- **Recipients** are Solana accounts that have registered. A transfer to an
  unregistered account fails with `WalletError.recipientNotRegistered`; use
  `withdraw` for a public payment.
- **Privacy**: deposits and withdrawals are public. Private transfers reveal
  neither amount nor recipient. The indexer learns this wallet's view tags, so
  `allowInsecureHttp` on `open`, which lets the default transport use a
  plaintext URL off loopback, is only for a local test cluster.
- **History**: `activity()` lists the transactions that moved this wallet's
  notes, newest first, as `deposit`, `received`, `sent`, `withdrawal` or
  `selfTransfer` (merges and transfers to itself), one entry per asset. The
  Zolana SDK reads and classifies them. The indexer does not mark withdrawals:
  a spend whose outputs are all this wallet's own is a `withdrawal`, one with
  another wallet's output `sent`.
- **Freshness**: the wallet keeps no chain state. `balances()`,
  `privateBalance()`, `activity()` and every spend read the wallet's notes from
  the indexer when they run, and `transfer`, `deposit` and `withdraw` return once the indexer
  has the transaction, so notes spent by another session or device are never
  picked. Each of those reads is a full read of the wallet's notes: a screen
  that shows balances and history calls `overview()`, which returns both from
  one read, and filters by asset itself. With `WalletConfig.reuseNotesMs`, a
  spend prepared within that many milliseconds of the last read takes its
  notes from that read instead of reading again, which saves a send screen
  that just showed the balance the indexer round trips; a send or a
  confirmation ends the reuse. A note spent elsewhere in that window fails
  the spend when it is sent, with `WalletError.notesAlreadySpent`, as one
  spent right after a read would.
- **Errors** are `WalletError`s that say what happened, with its data; the
  table below lists them. Errors never contain key material.

- **Lock and account switch**: call `wallet.close()`. Operations not yet
  started fail with `WalletError.walletClosed`, and so does the running one,
  whether it is proving or waiting for the network: the application never
  receives a transaction after the lock. Nothing is signed or submitted after
  the call, so lock the UI without awaiting `close()`. The package's default
  transport aborts its requests in flight at once; an application's transport
  should abort its own when the application locks. `close()` waits for the
  running native step (a proof cannot be interrupted) and for an open signer
  prompt, so cancel your prompt on lock. Then it releases the native wallet:
  its keys and the proving key it loaded.
  Open the next account after `close()` completes. A transaction already
  submitted is not recalled.

### Errors

Every failure throws one of these `WalletError` variants. Amounts are in base units, accounts and mints base58.

| Variant | Data | When |
|---|---|---|
| `insufficientPrivateBalance` | `requested`, `available` | the spendable notes of the asset do not cover the amount |
| `mergeRequired` | `amount`, `maxInputs` | the amount needs more notes than one spend takes |
| `tooManyInputTrees` | `trees`, `maxTrees` | the notes that cover the amount are on more trees than one spend takes (2) |
| `amountZero` | | the amount of a spend is zero |
| `nothingToMerge` | `mint` | fewer than two notes of the mint are free to merge on any one tree |
| `mergingDisabled` | | the account's record does not enable merging |
| `indexerBehind` | `indexerSlot`, `rpcSlot` | the indexer did not reach the RPC's confirmed slot within about ten seconds |
| `notesAlreadySpent` | | the chain already spent a note the transaction spends: the indexer was behind when it was prepared |
| `notesReserved` | `amount` | only notes a prepared spend reserves would cover the amount |
| `recipientNotRegistered` | `recipient` | the recipient has no shielded address in the registry |
| `recipientTokenAccountMissing` | `recipient`, `mint` | the recipient, or the collector of a token `publicFee`, has no associated token account for the mint |
| `registrationConflict` | `owner` | the registry holds other keys for this account |
| `assetNotSupported` | `mint` | the shielded pool has not registered the mint |
| `mintNotConfigured` | `mint` | the mint is not in `WalletConfig.mints` |
| `invalidMint` | `mint` | not a base58 key |
| `invalidTokenProgram` | `mint`, `tokenProgram` | the configured token program is neither SPL Token nor Token-2022 |
| `invalidPubkey` | `value` | not a base58 public key |
| `invalidMaxSpendInputs` | `maxInputs`, `limit` | `WalletConfig.maxSpendInputs` is outside 1 to 40 |
| `invalidDerivationSignature` | | the signature is not the account's over the derivation message |
| `invalidWalletKeys` | | saved keys are malformed or do not match their public keys |
| `rpcUrlInsecure`, `indexerUrlInsecure`, `provingKeyUrlInsecure`, `proverUrlInsecure` | `url` | with the default transport, a plaintext URL off loopback without `allowInsecureHttp` |
| `transportFailed` | `message` | the Solana RPC client could not be built |
| `signatureInvalid` | | a signature is malformed or not the signer's over the message |
| `signatureCountMismatch` | `expected`, `got` | `submit` received the wrong number of signatures |
| `transactionNotConfirmed` | `signature` | Solana did not confirm the transaction within the wait |
| `transactionExpired` | `signature` | a send went unanswered and the transaction did not land before its blockhash expired; sending it again is safe |
| `transactionOutcomeUnknown` | `signature`, `message` | a send went unanswered and whether it landed could not be told; wait for it before sending it again |
| `remoteProverMissing` | | the spend asked for remote proving without `WalletConfig.proverUrl` |
| `proofMalformed` | | the remote prover's response is not a proof of the pinned proving key |
| `proofInvalid` | | the proof does not verify against the pinned verifying key |
| `proofFailed` | | the device prover could not prove the request |
| `proverBusy`, `proverClosed`, `proverUnavailable` | | the prepared prover slot is taken, released or unusable |
| `proverInitFailed`, `proverLoadFailed` | | gnark or the proving key could not be loaded |
| `unsupportedCircuit` | | the prepared circuit's shape is not one the wallet proves |
| `provingKeyUnknown`, `provingKeyMismatch` | `name` | the lockfile does not pin the key the client expects |
| `provingKeyDownloadFailed`, `provingKeyCorrupt` | `name` | the key could not be downloaded, or does not hash to the pinned value |
| `provingKeyStoreFailed` | `path` | the key directory could not be read or written |
| `poseidonInputCountInvalid`, `poseidonInputLengthInvalid` | `count`; `index`, `length` | `poseidonHash` input is not 1 to 12 elements of 32 bytes |
| `signerMissing` | | the wallet was opened without a signer |
| `signerMismatch` | `wallet`, `signer` | the signer signs for another account |
| `unexpectedSigners` | `signers` | the transaction needs other signers than this account alone |
| `walletClosed` | | the wallet was closed |
| `client` | `message` | any other Zolana client failure, such as a transport failure (`transport failed: ...`), with `api-key` values masked |

Match on the variant classes, `WalletError_InsufficientPrivateBalance` and so
on, to read the data:

```dart
try {
  await wallet.transfer(payments: [Payment(recipient: account, amount: amount)]);
} on WalletError catch (e) {
  final text = switch (e) {
    WalletError_InsufficientPrivateBalance(:final available) =>
      'Only $available lamports available',
    WalletError_RecipientNotRegistered() => 'Recipient not registered',
    _ => '$e',
  };
}
```

### Opening from saved keys

`exportKeys()` returns the wallet's four derived keys. Save them in the
device's secure storage, and later open the wallet without a derivation
signature:

```dart
final keys = await wallet.exportKeys();
// later
final wallet = await ZolanaWallet.openWithKeys(
  config: config,
  solanaPublicKey: account,
  keys: keys,
  signer: KeystoreSigner(),            // optional
);
```

| Field | Bytes | Encoding |
|---|---|---|
| `viewingPrivateKey` | 32 | P-256 scalar, big-endian |
| `viewingPublicKey` | 33 | compressed SEC1 point |
| `nullifierPrivateKey` | 31 | nullifier secret |
| `nullifierPublicKey` | 32 | Poseidon hash of the nullifier secret |

`openWithKeys` checks each private key against its public key and fails with
`WalletError.invalidWalletKeys` otherwise; the error never contains key bytes.
Keys of another account open under another shielded address, and
`registrationStatus()` reports `conflict` for them. Without a `signer`,
`register`, `deposit`, `transfer` and `withdraw` fail with
`WalletError.signerMissing`; the `prepare` methods, `submit` and `confirm`
work.

The keys cannot move funds: spending needs the Solana account's signature.
They show the wallet's private balances and history and link its spends.
Keep them in device-only secure storage, without cloud sync or backups, per
account and network, and never send them to a backend. Store either the keys
or the 64-byte derivation signature, which derives the same keys, not both.

### Signing and sending in the application

`register`, `deposit`, `transfer` and `withdraw` prepare, sign with the
`SolanaSigner` and submit. An application that approves, signs and sends
transactions itself uses the `prepare` methods and reports back:

```dart
final tx = await wallet.prepareTransfer(
  payments: [Payment(recipient: account, amount: amount)],
);
// Show tx.operation and tx.feePayer, sign tx.message with each of
// tx.signers, send it.
await wallet.confirm(tx, signature); // base58 transaction signature
```

`tx.operation` has one variant per kind of transaction, each with only its own
fields (amounts in base units, `mint` `null` for SOL). Match on it as on
`WalletError`; the compiler checks that the switch covers every kind.
`Operation_Custom` is a transaction the application composes itself, such as a
call to another program, named by its top-level `programs`; the wallet does
not prepare one yet.

```dart
final text = switch (tx.operation) {
  Operation_Registration() => 'Register for private payments',
  Operation_Deposit(:final amount) => 'Shield $amount',
  Operation_Transfer(:final payments) => 'Send privately '
      '${payments.map((p) => '${p.amount} to ${p.recipient}').join(', ')}',
  Operation_Withdrawal(:final recipient, :final amount) =>
    'Unshield $amount to $recipient',
  Operation_Merging(:final enabled) => enabled ? 'Allow merges' : 'Stop merges',
  Operation_Merge(:final amount) => 'Combine notes holding $amount',
  Operation_Custom(:final programs) => 'Call ${programs.join(', ')}',
};
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
- The message carries a recent blockhash: `tx.lastValidBlockHeight` is the
  last block height at which it can land, about 150 blocks (60–90 s) after it
  was prepared. For a slower approval, `wallet.refresh(tx)` returns it with a
  new blockhash and the same proof; sign the new `message`. The proof stays
  valid while the trees still hold the roots it was built on (a state tree
  keeps its last 500); after that the program rejects it as stale, and the
  spend is prepared again.
- A prepared spend reserves the notes it spends, in memory. The next spend
  selects other notes until it is submitted or confirmed, released with
  `wallet.release(tx)` (for example when the user declines it), or past its
  `lastValidBlockHeight`. A spend that needs a reserved note fails with
  `WalletError.notesReserved`. `register`, `deposit`, `transfer` and `withdraw`
  release the notes themselves when signing fails.
- After a restart the prepared transaction is gone, but its signature and
  its `lastValidBlockHeight`, saved before sending, are enough:
  `wallet.waitForTransaction(signature, lastValidBlockHeight: height)` ends
  as `submit` does after a lost send: done once Solana confirms it and the
  indexer has it, the chain's error when the chain refused it,
  `WalletError.transactionExpired` when it can no longer land, or
  `WalletError.transactionOutcomeUnknown`. Without the height, a transaction
  never seen ends unknown, since it cannot be told expired.
- A send can fail after the transaction went out, for example when the RPC
  loses its confirmation. `submit` then looks the signature up until the
  outcome is known, and the result says whether sending again is safe: the
  signature once Solana confirms it; the chain's error when the chain refused
  it, or refused it in simulation so it was never sent;
  `WalletError.transactionExpired` when it did not land before its blockhash
  expired, so `refresh` or prepare it again; `WalletError.transactionOutcomeUnknown`
  when the lookups kept failing or it landed without confirming, so call
  `waitForTransaction(signature)` before sending it again. An application
  that sends itself does the same after any send error that is not the
  chain's verdict.
- Balances, activity and every spend first wait for the indexer to reach
  the RPC's confirmed slot, so a spend made moments ago, in this session or
  another, is read. An indexer still behind after about ten seconds fails
  with `WalletError.indexerBehind`; try again once it catches up.
- A spend can still select a note the chain has already spent, when the
  spend landed after that slot. It fails with
  `WalletError.notesAlreadySpent`, before or after sending; prepare it
  again once the indexer has the spend.

### Recovery

`example/lib/recovery.dart` covers three cases, and
`example/integration_test/recovery_test.dart` runs them on devnet:

- **A new phone**: open the wallet with `ZolanaWallet.open(signer: ...)`,
  exactly as on first use. The same account derives the same keys, and every
  note and the history are read from the indexer; nothing from the old phone
  is needed.
- **Reopening without a signature**: after that first open, save
  `exportKeys()` in the platform keystore and open with
  `ZolanaWallet.openWithKeys` from then on (see
  [Opening from saved keys](#opening-from-saved-keys)). Keys that fail with
  `WalletError.invalidWalletKeys` are derived again from the signature.
- **The app closed while sending**: sign the prepared transaction in the
  application and, before `submit`, save its signature (the fee payer's,
  base58) in storage that survives a restart. Keep it when `submit` fails
  with `WalletError.transactionOutcomeUnknown`, or with
  `WalletError.walletClosed` because the app locked during the send; drop it
  on any other result. Save the transaction's `lastValidBlockHeight` with
  it. After a restart, call `waitForTransaction(signature,
  lastValidBlockHeight: height)` before anything else is sent. With the example's `saveAndSend` and
  `settlePendingSend`:

```dart
try {
  await saveAndSend(wallet, signer, pending, tx);
} on WalletError_TransactionExpired {
  // It can no longer land: the same proof under a new blockhash is safe.
  await saveAndSend(wallet, signer, pending, await wallet.refresh(tx));
} on WalletError_TransactionOutcomeUnknown {
  // It may still land: wait for it before sending again.
  await settlePendingSend(wallet, pending);
}
```

### Backend proving

A transfer or withdrawal can be proved by a remote prover instead of the
device: the application's backend, or a Zolana prover. Set
`WalletConfig.proverUrl` and ask for remote proving per call or in the config:

```dart
final wallet = await ZolanaWallet.open(
  signer: KeystoreSigner(),
  config: WalletConfig(
    rpcUrl: rpcUrl,
    indexerUrl: indexerUrl,
    provingKeyDir: provingKeyDir,
    proverUrl: 'https://backend.example/zolana', // your backend
    proving: Proving.remote,                     // the default; optional
    mints: const [],
  ),
);
await wallet.transfer(
  payments: [Payment(recipient: account, amount: amount)],
  proving: Proving.remote,
);
```

- **The requests**: the wallet asks the prover with the Zolana SDK's own
  prover client, through the wallet's transport (see
  [Networking](#networking)), like every other request. It sends
  `POST <proverUrl>/prove/<key>` with `Content-Type: application/json`,
  `X-Sync: true` and the proof request as the body. `<key>` is the proving
  key's name without `.key`, for example `transfer_confidential_2_2`. The
  query of `proverUrl` (an `api-key`, for example) stays on every request.
  When the prover queues the proof and answers with a `jobId`, the wallet
  polls `GET <proverUrl>/prove/<key>/status?jobId=<jobId>` until the job
  completes or fails.
  The Helius gateway (`https://*.helius-rpc.com/v1/zolana`) routes proofs
  by their body, so for it the paths are `/prove` and `/prove/status`.
- **The application's backend** serves these two routes as a proxy of a
  Zolana prover: it forwards the body and the `X-Sync` / `X-Async` header to
  the same path of the prover, and returns the prover's status and body as
  they are. The response carries the proof's `provingKeySha256`, which the
  wallet checks. A Zolana prover that serves these routes can also be
  `proverUrl` directly.
- **Verification on the device**: the wallet verifies the proof against the
  pinned verifying key and the public input it computed itself, before it
  builds the message. A response that is not a proof of the pinned key fails
  with `WalletError.proofMalformed`, a proof that does not verify with
  `WalletError.proofInvalid`.
  A bad proof never reaches the user's approval. A failed request fails with
  a `WalletError.client` holding the prover client's error, `prover server error: ...` (for example
  `status 401 Unauthorized: ...`), `api-key` values masked.
- **Choice per call**: `WalletConfig.proving` is the default, `Proving.local`
  when it is null. `prepareTransfer`, `prepareWithdrawal`, `transfer` and
  `withdraw` take `proving` to override it. `Proving.remote` without a
  `proverUrl` fails with `WalletError.remoteProverMissing`.
- **Privacy**: the request carries the transaction's full witness: the notes
  it spends and creates, their amounts and owners, and the wallet's nullifier
  secret (the `nullifierPrivateKey` of `exportKeys()`). The backend and its
  prover learn the transaction and can recognize later spends of these notes.
  They cannot move funds: every spend needs the account's signature. Prove on
  the device when this is not acceptable. With the default transport, `open`
  refuses a plaintext `proverUrl` off loopback with
  `WalletError.proverUrlInsecure`.
- **Retries**: the prover client sends the proof request again only when no
  response arrived. When the status arrived and the body was lost, it fails
  instead, so the prover does not prove the same spend twice (see
  `TransportResponseLost` below).

Current limits: a spend takes at most `WalletConfig.maxSpendInputs` notes
(40 when unset, the most a key supports) from at most two trees and fails
with `WalletError.mergeRequired` or `WalletError.tooManyInputTrees` beyond
that (see [Merges](#merges)); one prepared prover is loaded per process, so
close a `LocalProver` before the wallet proves. A spend of zero fails with
`WalletError.amountZero`. Proof time and memory grow with the notes a spend
takes, so set the cap to what the phones you support prove in the time you
accept, and merge before spends need more: `TokenBalance.notes` says how
many notes each balance is in.

### Merges

A merge combines up to 54 notes of one asset on one tree into one note of
their sum. Merge when a spend fails with `WalletError.mergeRequired` or
`WalletError.tooManyInputTrees`, then prepare the spend again; or before
that, when `TokenBalance.notes` grows past what a spend should take, so
that spends stay narrow and fast.

```dart
await wallet.setMerging(true); // once per account; this account signs
final signature = await wallet.merge(proving: Proving.remote);
```

- **Turning merging on**: `setMerging(true)` (or `prepareMerging(true)` to
  sign in the application) writes it to this account's user record; `null`
  when the record already says so, and `mergingEnabled()` reads it. Until
  then a merge fails with `WalletError.mergingDisabled`.
- **What a merge takes**: `prepareMerge` takes the smallest notes of `mint`
  (SOL when null) on the tree that holds the most of them, at most
  `maxInputs` (24 by default, 54 at most), and leaves out notes that
  prepared spends reserve. Fewer than two fails with
  `WalletError.nothingToMerge`. It reserves its notes as a spend does.
- **Who signs**: nobody but the fee payer. `feePayer` (this account when
  null) pays and signs alone, so the application's backend can pay for it.
- **Proving**: `proving` works as for transfers. Through `proverUrl` the
  backend proxies the same `/prove` routes; the merge keys are 56 MB (8
  notes), 173 MB (24) and 380 MB (54), so remote proving suits most phones.
- **What the prover receives** (`Proving.remote`): for each note its amount,
  blinding, asset, nullifier and Merkle paths; the account's owner hash and
  nullifier public key; and the wallet's **nullifier secret**. No signing or
  viewing secret. The prover learns every merged amount and the merged note,
  and keeps the nullifier secret; it cannot move funds.
- **Expiry**: the proof can be sent by anyone who holds it, until ten
  minutes after `prepareMerge`; after that the program refuses it.
  `refresh` gives a new blockhash but does not extend the expiry.
- **Stopping halfway**: each merge is its own transaction, and the wallet
  reads its notes from the indexer every time. An application that stops
  between merges, or after preparing one, just prepares the next. A merge
  that was sent is confirmed with `waitForTransaction(signature)`; one that
  was not is gone with its expiry. A merge and a spend that share a note
  cannot both land: the second fails, and is prepared again.
- **History**: a merge reads as `selfTransfer` of the merged amount.
- **Visible on chain**: the account (it is the owner of the record), the
  tree, and roughly how many notes; not the asset or any amount.

### Networking

The wallet never opens a connection. Every request, Solana RPC and indexer
calls (`POST`, JSON), proving-key downloads (`GET`, the key files above,
in ranged parts, checked against the lockfile) and remote proofs (see
[Backend proving](#backend-proving)), goes through one transport in Dart:

- By default, the package's own transport on `package:http`: one connection
  pool per wallet, closed with `close()`. `open` and `openWithKeys` refuse a
  plaintext (`http`) URL off loopback in the config before anything else, with
  `WalletError.rpcUrlInsecure`, `WalletError.indexerUrlInsecure`,
  `WalletError.provingKeyUrlInsecure` or `WalletError.proverUrlInsecure`,
  unless `allowInsecureHttp: true` is passed. It follows redirects, fails a
  request whose response or any part of its body takes longer than
  `timeoutMs` (30 seconds without one), stops reading a body past
  `maxResponseBytes`, and throws `TransportResponseLost` when a body fails
  after the status arrived.
- Or the application's: pass a `transport` to `ZolanaWallet.open` or
  `ZolanaWallet.openWithKeys` to send through your own stack (a proxy,
  certificate pinning, your backend). Headers, timeouts and which URLs it
  accepts are then yours; `allowInsecureHttp` does not apply.

```dart
final client = http.Client();

Future<TransportResponse> send(TransportRequest request) async {
  final clock = Stopwatch()..start();
  final timeout = Duration(milliseconds: request.timeoutMs ?? 30000);
  final response = await client
      .send(
        http.Request(request.method, Uri.parse(request.url))
          ..headers.addAll(request.headers)
          ..bodyBytes = request.body,
      )
      .timeout(timeout);
  try {
    final body = http.ByteStream(response.stream.timeout(timeout)).toBytes();
    return TransportResponse(
      status: response.statusCode,
      headers: response.headers,
      // `timeoutMs` bounds the whole request; a proving key only each wait.
      body: await (request.timeoutMs == null
          ? body
          : body.timeout(timeout - clock.elapsed)),
    );
  } catch (error) {
    throw TransportResponseLost(response.statusCode, error);
  }
}

final wallet = await ZolanaWallet.open(
  signer: KeystoreSigner(),
  config: config,
  transport: send,
);
```

- The request carries the method, the URL with its `api-key` parameter, the
  content type of a `POST`, `X-Sync` or `X-Async` on a proof request, and
  `Range` on a key download; nothing else. Send it as it is, and follow redirects: the proving-key host
  may redirect. A proof request's body is the transaction's witness, the
  nullifier secret included: do not log it.
- A key download asks for one part of the key with `Range` and sets
  `maxResponseBytes` to that part's size (the whole key for the first part,
  in case the host ignores ranges). Stop reading and throw when the body
  exceeds it; the wallet refuses a longer body either way, but only after the
  whole of it arrived. Return the `206` with its `Content-Range` header: the
  wallet appends a part only to the range it asked for.
- A key download also sets `downloadPath`: write the body to that file as it
  arrives and return the response with an empty body. The wallet appends it
  to the key and checks the key against the lockfile once it is complete. A
  transport that returns the body instead still works, holding a part in
  memory.
- Return the response whatever its status, with its headers: a prover
  inside a TEE marks its encrypted body in them. Throw only when there is no
  response (no network, DNS, TLS, a timeout). The wallet then fails with a
  `WalletError.client` whose message holds `transport failed: ` and the
  exception message, never its stack trace, `api-key` values masked.
- When the status arrived and the body could not be read (a reset or a
  timeout while reading it), throw `TransportResponseLost(status, error)`.
  The server has the request and may be acting on it, so the wallet does not
  send it again: a prover would otherwise prove the same spend twice. Any
  other exception counts as no response, and a proof request is sent again.
- A request with `timeoutMs` set carries the bound for the whole request,
  body included, in milliseconds: just under 600 000 for a proof request,
  which the prover may answer only when the proof is done, and under 30 000
  for a status poll. Use it in place of your own, and when it passes after
  the status arrived, throw `TransportResponseLost`. The SDK gives up a
  second later and counts a request that failed any other way as unanswered,
  which it sends again: a prover would prove the spend twice. Time out every
  other request too: the wallet and `close()` wait for each answer, so abort
  the requests in flight when the application locks. For the same reason the
  transport must not call the wallet.
- `example/lib/app_transport.dart` is the transport above with a log of what
  it sent; the example's devnet test sends through it.

## Prepare once, prove repeatedly

```dart
import 'package:zolana_mobile/zolana_mobile.dart';

await initZolanaMobile();
final prover = await LocalProver.load(
  circuitPath: r1csPath,
  provingKeyPath: provingKeyPath,
  verifyingKeyPath: verifyingKeyPath,
);
try {
  final result = await prover.prove(requestJson: requestJson);
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

Load the native library with `initZolanaMobile()` before any other call.

The structured adapter supports `transfer-confidential`, `transfer-ring`,
`transfer-ring-authority`, `merge`, and `merge-ring`, with a matching supported
key shape. Other circuit types fail closed. The bundled demo stages only 2→2;
provide the matching trusted keys for other shapes.

Only one prepared native prover is loaded at a time per process, and each
`LocalProver` runs one proof at a time; another fails with
`WalletError.proverBusy`, and so do the wallet's own proofs while a
`LocalProver` is loaded. Close it before loading another circuit.
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
`close()` waits for the proof in progress, which then fails with
`WalletError.proverClosed`, and releases the prepared keys; later proofs fail
the same way. It is idempotent. Already-delivered results cannot be revoked:
always check the current wallet session/approval immediately before accepting
a proof or signing/submitting.

Inputs are copied across Dart, Rust, and Go. Do not pass borrowed pointers to
wallet-owned memory. You may clear caller-owned buffers once you no longer need
them, but that does not erase copies already passed to the prover. Dart strings
and Go/FFI temporaries cannot provide a complete secure-erasure guarantee.
Completion and `close()` mean no more SDK work uses the proof; they do not prove
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

Install Go 1.27.1 or newer (`brew install go`) and Flutter; the build hook
builds the Rust and Go sources when no release matches them, and finds Go in its
standard install locations outside Xcode's `PATH`. Then, with the workspace
closed:

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

### Proof timing

One harness times proofs on CI's phones and on yours. It proves the
fixture requests of `fixtures/` through the wallet's key store, as a spend
is proved, and reports each stage: the key's download when it was not on
the device, its load, the witness and the proof. The shapes are transfers
of 1, 2, 4, 8, 16 and 24 notes to two outputs (`1x2` to `24x2`) and a merge
of 24 notes (`merge-24x1`). The numbers come from release builds: `flutter
test` builds only debug, where Dart is interpreted and Rust unoptimized.

`scripts/test-consumer.sh bench-apk` builds the bench app and its
instrumentation for arm64 Android, and `bench-ipa` the iOS app and its
XCTest bundle. On an Android phone over adb:

```sh
ZOLANA_CONSUMER_DIR=/tmp/consumer ZOLANA_BENCH_DIR=/tmp/bench bash scripts/test-consumer.sh bench-apk
adb install -r -t /tmp/bench/app.apk && adb install -r -t /tmp/bench/app-androidTest.apk
adb shell pm clear com.example.zolana_consumer
adb logcat -G 16M && adb logcat -c
for run in fresh restart; do
  adb shell am instrument -w com.example.zolana_consumer.test/androidx.test.runner.AndroidJUnitRunner
done
mkdir -p /tmp/timing && adb logcat -d -s flutter | grep -o 'ZOLANA_TIMING {.*' > /tmp/timing/phone.log
python3 scripts/proof-timing-report.py --logs /tmp/timing
```

The first run finds no keys on the device and gives the first two rows of
the report (first run including the download, then warm); the second is a
new process that finds them and gives the other two (first run after a
restart, then repeated). An iOS test run gets one install, so the
`bench-ipa` app makes the restart rows after the fresh ones in one process,
closing and reopening the wallet between (`ZOLANA_TIMING_RUN=both`). The
keys download from the key host (9 MB for one note, 59 MB for 8, 180 MB for
24, 173 MB for the merge). `ZOLANA_TIMING_SHAPES` narrows the shapes, and
`ZOLANA_TIMING_ROUNDS` (4, and 2 for shapes over 8 notes) is the proofs per
shape and run; the report takes medians.

CI benches every pull request on a Samsung Galaxy A16 and an iPhone 12 in
AWS Device Farm (`.github/workflows/device-bench.yml`, `scripts/devicefarm/`)
and comments each phone's report with the change against the last commit of
`main`. After each push to `main` the reports are commented on the commit
and join the history on the
[`proof-timing`](https://github.com/helius-labs/zolana-mobile-sdk/tree/proof-timing)
branch, a table of the last commits per phone.

`wallet.proveRequest(keyName, requestJson)` is what the harness calls: it
proves any Zolana `/prove` request with a pinned key, and returns the
proof and the stage timings. `tools/prove-fixtures` wrote the fixture
requests, on trees of its own: they prove with the pinned keys and land
nowhere.
