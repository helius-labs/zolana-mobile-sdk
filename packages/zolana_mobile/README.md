# Zolana Flutter SDK

A private Zolana wallet for Flutter that proves on the device with zopro, a
Groth16 prover in Rust for gnark's proving keys, on the GPU where there is
one. Android and iOS are supported. The package contains its Rust sources; it
does not require a sibling Zolana checkout.

This SDK pins Zolana source `5cfd7eaed15ef29dc95f6b212cd2844ed0e86860`
([#501](https://github.com/helius-labs/zolana/pull/501)) and shielded-pool program
`ring1GaLCuJnkUMVkX2JeZLTTX17wKsV4fZmPVzXANW`. Applications supply deployment
configuration independently of the SDK:

```dart
final config = WalletConfig(
  rpcUrl: solanaRpcUrl,
  indexerUrl: indexerUrl,
  proverUrl: proverUrl,
  proverTeePolicy: reviewedPolicyJson,
  provingKeyDir: '/persistent/private/proving-keys',
);
```

Remote URLs require a supplied policy; `requireProverTee: true` requires one for
loopback proxies too. Missing policy raises `WalletError.proverPolicyMissing`
before HTTP. The URL never selects a policy, and there is no devnet fallback.
`Deployment.devnet()` and `Deployment.mainnet()` have been removed; migrate
presets into application configuration. Complete reviewed policy files, including
HPKE public-key pins, live under the example's `assets/policies/`. Bundle and
checksum-pin the chosen file independently of the SDK release. Never learn trust
pins from the prover being verified. Local proving needs no TEE policy.

Earlier programs' notes, caches and pending transactions cannot be migrated by
changing URLs. Reconcile them using their matching SDK and deployment before
upgrading. Prepared snapshots now use version 2 and a new authentication domain;
old snapshots are rejected. Transfer and merge proving-key hashes are unchanged,
so the verified local key cache can be reused. The public-seed Flutter example
continues to use devnet-e; use your own signer for mainnet.

**Performance.** The native prover at four threads on an Apple M5 Pro proves a warm spend (key loaded) in 0.17 s for one note to two outputs, 0.27 s for two, 0.55 s for four, 0.92 s for eight, 1.7 s for sixteen and 2.7 s for twenty-four; a restart adds the key's load from its mapped image (tens of milliseconds there), and the first run its download (9 MB to 180 MB). On phones, in release builds, one note proves in 0.18 s on a Samsung Galaxy S25, 0.21 s on an iPhone 17 Pro, 0.30 s to 0.35 s on an iPhone 15, an iPhone 12 and a Galaxy A56, and 1.2 s on a Galaxy A16; eight notes in 1.2 s to 2.0 s, 7.0 s on the A16; twenty-four in 3.6 s to 5.9 s, 25 s on the A16. A key loads from its image in under 0.3 s, up to 0.74 s on the A16. CI benches release builds on these six phones for every pull request and keeps the history of `main` on the [`proof-timing`](https://github.com/helius-labs/zolana-mobile-sdk/tree/proof-timing) branch; "Proof timing" below describes the harness.

## Requirements

- Flutter with Dart 3.13.0 or newer. CI checks Flutter 3.47.2 (Dart 3.13.2) and
  3.47.4.
- Android: the NDK named by the app's `android.ndkVersion`.
- iOS: Xcode with its license accepted. The app may use Swift Package Manager
  or CocoaPods.
- Android API 24 or newer and iOS 15 or newer, the Flutter minimums. CI runs
  the clean consumer on Android API 26 and 35 emulators, and checks that every
  object in the iOS native library runs on iOS 16.
- A zopro key (`<key>.zopro`) from a trusted, checksum-verified release for
  each shape proved with `LocalProver`; the wallet downloads its own, pinned
  by its lockfile. Do not load keys supplied by an untrusted proof request.

Rust is needed only for a build from source, see
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

The release name contains a hash of the Rust sources of the plugin. If no
release matches the sources, for example after a local change, the plugin builds
the library from source. A build from source needs:

- Rust stable with the target of the platform.
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

`ZolanaWallet` runs the whole flow: registration, deposit, and spends to
private and public recipients. It builds every transaction with the Zolana
Rust client and proves it on the device, or through the application's backend
(see [Backend proving](#backend-proving)). The Solana secret key never enters
the package; your `SolanaSigner` (a platform keystore, a wallet adapter, a
custodian) signs two things:

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
  ),
);
await wallet.register();               // once, so others can pay this wallet
await wallet.deposit(BigInt.from(500000000));
await wallet.spend(
  outputs: [Output.private(recipient: registeredAccount, amount: BigInt.from(100000000))],
);
```

- **Outputs**: a spend takes one or more outputs of one asset and proves them
  together, in one transaction. `Output.private` is a note for a registered
  account; `Output.public` moves funds to a Solana account, tokens to its
  associated token account. Your application's own fee is one more private
  output, to its registered collector, on a private send and on a withdrawal
  alike:

  ```dart
  await wallet.spend(outputs: [
    Output.private(recipient: registeredAccount, amount: amount),
    Output.private(recipient: feeCollector, amount: fee),
  ]);
  await wallet.spend(outputs: [
    Output.public(recipient: destination, amount: amount),
    Output.private(recipient: feeCollector, amount: fee),
  ]);
  ```

  A spend with `k` private outputs proves with `k + 1` outputs, its change
  included, and public outputs take no proof slot: the keys take at most 40
  notes for `k <= 1`, 24 for `k <= 3`, 16 for `k <= 7` and 8 for `k <= 15`,
  within `WalletConfig.maxSpendInputs`. Past that the spend fails with
  `WalletError.mergeRequired`; `TokenBalance.limits` says what each width can
  take now (see [Balances](#balances)). The prepared spend's
  `Operation_Spend` lists each output in order. An empty list fails with
  `WalletError.noOutputs`, a zero amount with `WalletError.amountZero`,
  before anything is asked of the network.
- **Checking before signing**: `PreparedTransaction.outputs` returns the final
  output openings of a spend in proof order, including change and padding.
  Every opening contains its final blinding; neither the random transaction
  seed nor the derived output seed is exposed. Other transaction types return
  an empty list, and `refresh` preserves the openings.

  Payment policy belongs to the app. The example's `SpendChecks` parses the
  actual Solana v1 message and its default-ring `transact` instruction, reads
  recipient registrations and the output tree from the app's trusted RPC,
  recomputes each published output commitment and checks discovery tags. It
  compares private payments, the app fee, change ownership and public
  withdrawals against the UI request captured before preparation. It permits
  only the token-account creation needed by those withdrawals and caps the
  priority fee at 10,000 lamports. Unknown formats or circuits are refused.
  The signer signs the exact byte snapshot that passed these checks and shows
  the checked payment facts instead of trusting `operation` as evidence.

  This example policy checks spend outputs; it does not validate recipient
  ciphertext decryptability, audit protocol tree fees, or independently
  verify Groth16 proofs. The shielded-pool program verifies proofs during
  execution. Registration, deposit and merge approvals retain their existing
  behavior. Openings are confidential: keep them out of logs and analytics.

- **One transaction**: use `await wallet.transaction(signature)` on a detail
  screen. It requests `getShieldedTransactionsBySignature` and the confirmed
  Solana transaction, decrypts only that signature's events, and returns every
  asset row. It may read asset and recipient registrations, but never calls
  the transaction/UTXO history pagers or warms the spendable-note cache.

  ```dart
  final result = await wallet.transaction(signature);
  switch (result.status) {
    case TransactionLookupStatus.ready:
      showRows(result.entries); // Empty means no rows for configured assets.
    case TransactionLookupStatus.pendingIndexing:
      showIndexing(); // The indexer has not reached result.slot yet.
    case TransactionLookupStatus.notFound:
      showUnavailable(); // Neither indexer nor confirmed RPC found it.
    case TransactionLookupStatus.incomplete:
      showDetailsUnavailable(); // Never a partial or guessed amount.
  }
  ```

  Network errors throw `WalletError`. `notFound` does not prove a transaction
  never existed: an RPC may prune old transactions. A fresh wallet can read
  deposits, received notes, merges, default-ring sends with authenticatable
  recipient outputs, and public withdrawals without first scanning its input
  notes. Missing settlement metadata, unsupported spend formats, and
  recipient-tagged dummy padding return `incomplete`. The last case can occur
  on an exact-balance private send with no change: a cold reader cannot tell
  that padding from an unreadable recipient note. The method does not hide
  that limitation behind a full-history scan.

  Every `ActivityEntry` from this method, `activity()` and `overview()` has
  `{signature, slot, index}`. `index` is the pool's immutable asset id (1 for
  SOL), so it does not move when rows are filtered or another asset is added.
  Mobile rows aggregate one asset per signature; this index is not the TS
  SDK's per-output leaf index and should not be compared numerically with it.

- **What is public**: a deposit shows this account, the asset and the amount.
  A spend shows this account, the asset, each public output's recipient and
  amount, and the Solana address each private output's note is tagged with
  on chain, so anyone can see which accounts a spend paid privately; the
  private amounts and the change are hidden. The indexer learns this wallet's
  view tags, so `allowInsecureHttp` on `open`, which lets the default
  transport use a plaintext URL off loopback, is only for a local test
  cluster.

- **Proving keys** are zopro keys, each built offline from the gnark proving
  key of the pinned Zolana revision with the same name. They download from
  the Zolana key host into `provingKeyDir` (through the transport, see
  [Networking](#networking)), and are used only after they match
  `zopro-keys.lock`, which pins each file's size and sha256 beside the gnark
  key's sha256 that proofs report. The wallet picks the circuit shape, so the
  first spend of a new shape downloads its key, 7 MB for one note and up to
  315 MB for the widest spends and merges, unless it is already on the
  device:

  ```dart
  // On Wi-Fi after onboarding: spends of up to four notes, about 60 MB.
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
  that was checked once is not hashed again unless its file changed. A
  key's first load writes its image beside it (`<key>.zopro.zopro-raw-3.solver2`,
  about three times the key's size): its points as the prover reads them and
  its solver. Later loads map the image instead of decompressing the key and
  building its solver, and the system may drop and read again its pages under
  memory pressure rather than counting them against the app. A device that
  cannot write the image proves from the key file.
  `clearProvingKeys()` removes the images with the keys.
- **Tokens**: the wallet holds every SPL Token or Token-2022 mint the
  shielded pool registered. It reads a mint's asset id and token program the
  first time a call names the mint, and when a read meets notes in a mint it
  does not know yet, it reads the pool's registrations once and decrypts the
  transactions it already fetched again. Notes in an asset the pool has not
  registered are left out. To hold only some mints, list each in
  `WalletConfig.mints` with the token program that owns it, as your Solana SDK
  reads it:

  ```dart
  mints: [
    MintConfig(
      mint: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v',
      tokenProgram: 'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA',
    ),
  ],
  ```

  The wallet then reads only the asset id the shielded pool registered for
  each. Pass `mint` (base58) to a call; no `mint` is SOL. Amounts are in base
  units. With a list, a mint that is not in it fails with
  `WalletError.mintNotConfigured`, a token program that is neither SPL Token
  nor Token-2022 fails `open` with `WalletError.invalidTokenProgram`, and
  notes in other mints are left out.
  A mint the pool has not registered yet has no balance and no history, and
  the other assets read as usual. Its first deposit registers it: the
  transaction creates the mint's registry entry and vault before the
  deposit, this account pays their rent, and `Operation.deposit` has
  `createsSplInterface` set so the approval screen can say so. A pool that
  lets only its authority register mints refuses that deposit with
  `WalletError.assetNotSupported`, as it refuses a spend or merge of a mint
  it has not registered. Two first deposits of one mint prepared at the same
  time both register it, so the second to land fails.
  `balances()` lists the private balance of SOL and of each mint the wallet
  knows.
  A public token output goes to the recipient's associated token account.
  When the recipient has none for the mint, the transaction creates it ahead
  of the spend and the fee payer pays its rent (about 0.002 SOL);
  `Operation.spend`'s `createdTokenAccounts` names those recipients, for the
  approval text. Transaction fees are paid in SOL by this account.
- **Public accounts**: read public SOL and token balances with your
  application's own Solana client.
- **Registration**: `registrationStatus()` is `notRegistered`, `registered` or
  `conflict`. A conflict means the account's user record holds other keys: set
  by another app, by a key rotation, or derived by a signer whose signatures
  change between calls. `register()` then fails with
  `WalletError.registrationConflict` and never replaces them. The wallet can
  still spend its own notes.
- **Recipients** of a private output are Solana accounts that have
  registered. An unregistered one fails with
  `WalletError.recipientNotRegistered`; a public output pays it publicly.
- **History**: `activity()` lists the transactions that moved this wallet's
  notes, newest first, as `deposit`, `received`, `sent`, `withdrawal` or
  `selfTransfer` (merges and transfers to itself), one entry per asset. The
  Zolana SDK reads and classifies them. The indexer does not mark
  withdrawals: a spend with a note for another wallet is `sent`, for its
  whole total, fee and public outputs included; one with only public
  outputs, whose notes are all this wallet's own, is a `withdrawal`.
- **Freshness**: the wallet keeps no chain state. `balances()`,
  `privateBalance()`, `activity()` and every spend read the wallet's notes from
  the indexer when they run, and `spend` and `deposit` return once the indexer
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
| `noOutputs` | | a spend has no outputs |
| `nothingToMerge` | `mint` | fewer than two notes of the mint are free to merge on any one tree |
| `mergingDisabled` | | the account's record does not enable merging |
| `indexerBehind` | `indexerSlot`, `rpcSlot` | the indexer did not reach the RPC's confirmed slot within about ten seconds |
| `notesAlreadySpent` | | the chain already spent a note the transaction spends: the indexer was behind when it was prepared |
| `notesReserved` | `amount` | only notes a prepared spend reserves would cover the amount |
| `recipientNotRegistered` | `recipient` | the recipient has no shielded address in the registry |
| `registrationConflict` | `owner` | the registry holds other keys for this account |
| `assetNotSupported` | `mint` | the shielded pool has not registered the mint, and this call cannot: a spend, a merge, or a deposit the pool lets only its authority register; or the mint is not an SPL Token or Token-2022 mint |
| `mintNotConfigured` | `mint` | `WalletConfig.mints` lists mints and this one is not among them |
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
| `transactionExpired` | `signature` | retained for compatibility; missing history after expiry now returns `transactionOutcomeUnknown` |
| `transactionOutcomeUnknown` | `signature`, `message` | a send went unanswered and whether it landed could not be told; wait for it before sending it again |
| `remoteProverMissing` | | the spend asked for remote proving without `WalletConfig.proverUrl` |
| `proofMalformed` | | the remote prover's response is not a proof of the pinned proving key |
| `proofInvalid` | | the proof does not verify against the pinned verifying key |
| `proofFailed` | | the device prover could not prove the request |
| `proverBusy`, `proverClosed`, `proverUnavailable` | | the prepared prover slot is taken, released or unusable |
| `proverLoadFailed` | | the proving key could not be loaded |
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
  await wallet.spend(outputs: [Output.private(recipient: account, amount: amount)]);
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
| `viewingPrivateKey` | 32 | clamped X25519 secret |
| `viewingPublicKey` | 33 | `0x04` and the X25519 u-coordinate |
| `nullifierPrivateKey` | 31 | nullifier secret |
| `nullifierPublicKey` | 32 | Poseidon hash of the nullifier secret |

`openWithKeys` checks each private key against its public key and fails with
`WalletError.invalidWalletKeys` otherwise; the error never contains key bytes.
Keys of another account open under another shielded address, and
`registrationStatus()` reports `conflict` for them. Without a `signer`,
`register`, `deposit` and `spend` fail with `WalletError.signerMissing`; the
`prepare` methods, `submit` and `confirm` work.

The keys cannot move funds: spending needs the Solana account's signature.
They show the wallet's private balances and history and link its spends.
Keep them in device-only secure storage, without cloud sync or backups, per
account and network, and never send them to a backend. Store either the keys
or the 64-byte derivation signature, which derives the same keys, not both.

### Signing and sending in the application

`register`, `deposit` and `spend` prepare, sign with the `SolanaSigner` and
submit. An application that approves, signs and sends transactions itself
uses the `prepare` methods and reports back:

```dart
final tx = await wallet.prepareSpend(
  outputs: [Output.private(recipient: account, amount: amount)],
);
// Show tx.operation and tx.feePayer, sign tx.message with each of
// tx.signers, send it.
await wallet.confirm(tx, signature); // base58 transaction signature
```

For a new wallet, `prepareRegistration(enableMerging: true)` adds
`set_merge_encryption(PoseidonCtrV1)` after register in the same message.
The option defaults to `false`. An already registered wallet still returns
`null`; use `prepareMerging(true)` for it. Enabling merges lets anyone holding
the wallet's nullifier secret and notes combine them without moving funds.

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
  Operation_Spend(:final outputs) => 'Send ${outputs.map((o) => switch (o) {
    Output_Private(:final recipient, :final amount) => '$amount privately to $recipient',
    Output_Public(:final recipient, :final amount) => '$amount publicly to $recipient',
  }).join(', ')}',
  Operation_Merging(:final enabled) => enabled ? 'Allow merges' : 'Stop merges',
  Operation_Merge(:final amount) => 'Combine notes holding $amount',
  Operation_Custom(:final programs) => 'Call ${programs.join(', ')}',
};
```

`confirm` checks that `signature` is the fee payer's signature over
`tx.message`, waits until Solana confirms it and, for shielded-pool
transactions, until the indexer has it. `submit(tx, signatures)`
sends through the wallet's RPC instead.

- `prepareSpend` takes an optional `feePayer`, for example the application's
  backend. It pays the network fee instead of this account and signs first:
  `tx.signers` is `[feePayer, owner]`, and the fee payer's signature is the
  transaction signature. The proof binds the fee payer, so choose it at
  prepare. `spend` always uses this account.
- Every `prepare` method, and `register`, `deposit`, `spend`, `setMerging`
  and `merge`, accepts `priorityFeeLamports`: a total priority fee per
  transaction, in lamports. Omitting it uses `WalletConfig.priorityFeeLamports`;
  omitting both adds no priority fee. Pass zero to override a configured fee.
  The fee is written into the v1 message header before signing and exposed as
  `tx.priorityFeeLamports`. It is added to the base transaction fee and is
  independent of the compute limit, including when a spend creates token
  accounts. `refresh` preserves it; prepare again to change it.
- The message carries a recent blockhash: `tx.lastValidBlockHeight` is the
  last block height at which it can land, about 150 blocks (60–90 s) after it
  was prepared. For a slower approval, `wallet.refresh(tx)` returns it with a
  new blockhash and the same proof; sign the new `message`. The proof stays
  valid while the trees still hold the roots it was built on (a state tree
  keeps its last 500); after that the program rejects it as stale, and the
  spend is prepared again.
- A prepared spend reserves the notes it spends, in memory, in this open
  wallet only. The next spend selects other notes until the spend's outcome
  is known through `submit` or `confirm`, it is released with
  `wallet.release(tx)` (for example when the user declines it), or it is
  past its `lastValidBlockHeight`. A spend that needs a reserved note fails
  with `WalletError.notesReserved`; `TokenBalance.reserved` says how much the
  reserved notes hold. `register`, `deposit` and `spend` release the notes
  themselves when signing fails. A send with a verdict releases them at
  once: the chain refused it or it spent a note the chain already had.
  An unknown outcome leaves reservations unchanged; their block height
  still limits their lifetime. Reservations do not replace the application's
  pending-payment tracking.
- After a restart, call
  `wallet.waitForTransaction(signature, lastValidBlockHeight: height)` with
  the signature saved before sending. It searches RPC history and matching
  indexed events, returns a confirmed chain error when available, and waits
  for indexing on success. Missing history remains
  `WalletError.transactionOutcomeUnknown`, including after blockhash expiry.
  The optional height stops waiting early; it does not establish non-inclusion.
- A send can fail after the transaction went out, for example when the RPC
  loses its confirmation. `submit` uses the same recovery checks. When its
  outcome remains unknown, keep the signature and call `waitForTransaction`
  again before preparing a replacement payment. `confirm` applies those
  checks when the application sends the transaction itself.
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
  with `WalletError.transactionOutcomeUnknown`, `WalletError.client`
  (including an indexing timeout after the payment succeeded), or with
  `WalletError.walletClosed` because the app locked during the send; drop it
  on any other result. Save the transaction's `lastValidBlockHeight` with
  it. After a restart, call `waitForTransaction(signature,
  lastValidBlockHeight: height)` before anything else is sent. The example's
  `saveAndSend` and `settlePendingSend` retain the saved send on those errors:

```dart
try {
  await saveAndSend(wallet, signer, pending, tx);
} on WalletError_TransactionOutcomeUnknown {
  // Keep the signature: expiry alone does not make a replacement safe.
  await settlePendingSend(wallet, pending);
} on WalletError_Client {
  // The payment may have succeeded: finish indexing before sending again.
  await settlePendingSend(wallet, pending);
}
```

`waitForTransaction` searches RPC history, including transactions no longer
in the recent status cache, and can recover from matching indexed events
when RPC history is unavailable. If neither source establishes an outcome,
it returns `transactionOutcomeUnknown`, even after `lastValidBlockHeight`.
That height stops waiting early; it does not prove the payment never landed.
The application keeps the pending signature and decides when to check again.

### Sponsored cache merges (v0.6)

For a backend that accepts `[CreateCache, MergeTransact]`, pass a `cacheTarget`
to `prepareMerge`, then use each merge's `cachedOutput` in the final
`prepareSpend(cachedInputs: ...)`. Without a target, `prepareMerge` retains its
ordinary tree flow; changing only `feePayer` does not create a cache transaction.

```dart
// Terms and slot assigned by your authenticated backend run.
final merge = await wallet.prepareMerge(
  cacheTarget: CacheTarget(
    sponsor: sponsor, nonce: nonce, treeId: treeId, expiresAt: expiresAt,
    slot: slot, maxPriorityFeeLamports: priorityCap, computeUnitLimit: mergeCu,
  ),
  maxInputs: 8,
  proving: Proving.remote,
  priorityFeeLamports: BigInt.from(5000),
);
final snapshot = await wallet.exportPreparedTransaction(merge);
// Persist snapshot securely BEFORE handing merge.message to the relayer.
// The sponsor signs; the wallet user does not sign this merge.
// After the relayer submits it, check its predicted output in the cache:
await wallet.verifyCachedOutputs([merge.cachedOutput!]);
final payment = await wallet.prepareSpend(
  outputs: [Output.public(recipient: destination, amount: amount)],
  cachedInputs: [merge.cachedOutput!], // Or distinct slots of this same cache.
  proving: Proving.local,
  priorityFeeLamports: BigInt.from(5000),
);
// Persist payment too, independently check outputs, obtain user approval,
// sign payment.message and submit. No sponsor signature is needed here.
```

Cached payment preparation always proves locally and rejects an explicit remote
proving request. It uses only the supplied cached inputs, from one cache and one
asset. Cached merges select original notes from the target tree (24 by default,
51 at most with CreateCache); each round writes one slot. Merge outputs are also
published to the tree, but the cache removes the final payment's wait for their
indexed inclusion paths. Existing input/witness discovery still uses the indexer.

`verifyCachedOutputs` checks the confirmed account's owner, layout, run metadata
and exact output commitments. A not-yet-created or wrong cache fails, so wait or
reconcile the relayer submission before proceeding. The fee is chosen before
preparation and must fit the run's priority cap. Full quote/prepayment validation
is the backend's responsibility. Cache cleanup is separate from the user payment
in the example, after expiry, so it does not add a sponsor signature.

Save snapshots and submission signatures across rounds. On restart, reopen the
same wallet and call `restorePreparedTransaction(snapshot)` before allowing new
spends; restored transactions include cache descriptors, openings and input
reservations. Snapshots contain private data and are authenticated, **not
encrypted**. Store them securely. Cached and restored reservations require an
explicit release or a settled outcome; a lost submission response or an expired
blockhash does not make a replacement payment safe. Refresh only an unsigned
payment; it rechecks cache validity but cannot renew its expiry or proof roots.

The [runnable proxy/relayer example](example/backend/README.md) includes
`x-sf-auth`, synchronous and queued TEE merge routes, persistent run recovery and a Flutter entry
point (`example/lib/main_sponsored.dart`). Both use upstream
`5cfd7eaed15ef29dc95f6b212cd2844ed0e86860` and the artifacts in
this checkout. These mobile APIs are unreleased; e30b328a does not expose them.
See the example for migration boundaries, trusted TEE policy configuration and
the production responsibilities it intentionally leaves to the client backend.

### Backend proving

A spend can be proved by a remote prover instead of the device: the
application's backend, or a Zolana prover. Set
`WalletConfig.proverUrl` and ask for remote proving per call or in the config:

```dart
final wallet = await ZolanaWallet.open(
  signer: KeystoreSigner(),
  config: WalletConfig(
    rpcUrl: rpcUrl,
    indexerUrl: indexerUrl,
    provingKeyDir: provingKeyDir,
    proverUrl: 'https://backend.example/zolana', // your backend
    proverTeePolicy: reviewedPolicyJson,
    proving: Proving.remote,                     // the default; optional
  ),
);
await wallet.spend(
  outputs: [Output.private(recipient: account, amount: amount)],
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
  completes or fails. Set `WalletConfig.proverSyncOnly: true` when the backend
  supports synchronous responses only; queued or asynchronous fallbacks then
  fail without sending a status-polling request.
  The Helius gateway (`https://*.helius-rpc.com/v1/zolana`) routes proofs
  by their body, so for it the paths are `/prove` and `/prove/status`.
- **Attestation**: from every prover but a loopback one, the Helius gateway
  included, the wallet first fetches
  `GET <proverUrl>/tee/v1/attestation?nonce=<hex>`, checks that the prover
  matches the application's supplied policy, and encrypts every
  request body to it, with `Zolana-Tee` and `Zolana-Tee-Enc` headers.
  POST ciphertext is in the octet-stream body; encrypted GET uses the
  `Zolana-Tee-Ciphertext` header. Each request is bound to its method and
  its route below `proverUrl`. A remote prover that does not attest fails
  every remote spend: no setting sends it plaintext. Set
  `requireProverTee: true` to enforce this for a localhost proxy too.
  `proverTeePolicy` is required for remote URLs and forced loopback attestation.
  It takes trusted `TeePolicy` JSON or a deployment pin file; no policy is selected
  automatically. Distribute that policy
  through the application's trusted configuration, never from the prover being
  verified. Verification checks vendor evidence, freshness, nonce/key binding
  and the policy's deployment measurements and key/GPU constraints.
- **The application's backend** serves these routes, the attestation
  included, as a proxy of an attested Zolana prover: it forwards the body
  and the `X-Sync` / `X-Async` and `Zolana-Tee*` headers to the same route
  of the prover, and returns the prover's status, headers and body as they
  are. The response carries the proof's `provingKeySha256`, which the
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
  when it is null. `prepareSpend` and `spend` take `proving` to override it.
  `Proving.remote` without a `proverUrl` fails with
  `WalletError.remoteProverMissing`.
- **Privacy**: the request carries the transaction's full witness: the notes
  it spends and creates, their amounts and owners, and the wallet's nullifier
  secret (the `nullifierPrivateKey` of `exportKeys()`). Encrypted to the
  attested prover, only the enclave reads it, not a backend or gateway that
  relays it. A loopback prover gets it in plaintext.
  None of them can move funds: every spend needs the account's signature.
  Prove on the device when this is not acceptable. With the default transport,
  `open` refuses a plaintext `proverUrl` off loopback with
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
many notes each balance is in, and `TokenBalance.limits` what one spend can
take now.

### Balances

`balances()` returns one `TokenBalance` per asset held, and `overview()` the
same from one read with the history and `unresolvedAssets`:

- `amount` is what the indexer has confirmed for this wallet, `reserved`
  included. A payment or change on its way appears once its transaction is
  confirmed and indexed.
- `notes` is how many spendable notes the amount is in.
- `reserved` is the part of `amount` in notes this wallet's prepared spends
  hold. Reservations live in memory, in this open wallet only.
- `limits` has one `SpendLimit` per proof width the pinned keys offer (2, 4,
  8 and 16 outputs), ascending: the most one spend of up to that many outputs
  can take now without a merge, from the unreserved notes, largest first,
  within the width's note cap and `WalletConfig.maxSpendInputs`, from at
  most two trees. A spend with `k` private outputs uses `k + 1` outputs, and
  public outputs take no slot: a withdrawal, a withdrawal with one private
  fee, or a send to one wallet reads the entry with `outputs: 2`, a send to
  one wallet with a fee the entry with `outputs: 4`. The limits include any
  application fee the spend pays. Every amount up to a limit selects; one
  more fails with `WalletError.mergeRequired`,
  `WalletError.tooManyInputTrees` or `WalletError.insufficientPrivateBalance`.

`overview().unresolvedAssets` lists asset ids and mints found in wallet notes
that remain unresolved after refreshing the pool's registrations. Those notes
are excluded from balances and activity. Senders can supply unregistered asset
ids, so these notes do not make the whole read fail. Show the known balances
and mark the portfolio as incomplete while either list is non-empty. Later
reads include the notes if their assets become registered. Mints excluded by
`WalletConfig.mints` also appear in these lists.

### Merges

A merge combines up to 52 notes of one asset on one tree into one note of
their sum. Merge when a spend fails with `WalletError.mergeRequired` or
`WalletError.tooManyInputTrees`, then prepare the spend again; or before
that, when `TokenBalance.notes` grows past what a spend should take, or
`TokenBalance.limits` falls short of what the user wants to send, so that
spends stay narrow and fast.

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
  `maxInputs` (24 by default, bounded by the pinned merge shape), and leaves out notes that
  prepared spends reserve. Fewer than two fails with
  `WalletError.nothingToMerge`. It reserves its notes as a spend does.
- **Who signs**: nobody but the fee payer. `feePayer` (this account when
  null) pays and signs alone, so the application's backend can pay for it.
- **Proving**: `proving` works as for spends. Through `proverUrl` the
  backend proxies the same `/prove` routes; remote merges need no merge-key
  download on the device. The pinned merge shapes are 8, 24 and 52 inputs.
  Local merges load the corresponding key; use `provingKeys()` for its size.
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
final prover = await LocalProver.load(keyPath: zoproKeyPath);
try {
  final result = await prover.prove(requestJson: requestJson);
  useProofIfWalletSessionIsStillAuthorized(result.proofJson);
} finally {
  await prover.close();
}
```

`requestJson` is a structured Zolana `/prove` request built from real wallet
state, not a seed or an invented input balance. The prover builds its witness
locally and returns the canonical `{ar, bs, krs}` proof JSON consumed by Zolana.
It verifies the proof locally before returning. The SDK does not acquire wallet
state, authorize a transaction, sign, or submit it; `ZolanaWallet` does all of
that except signing.

Load the native library with `initZolanaMobile()` before any other call.

The prover supports `transfer-confidential`, `transfer-ring`,
`transfer-ring-authority`, `merge`, and `merge-ring`, each with the key of its
shape. Other circuit types fail closed. The bundled demo stages only 2→2;
provide the matching trusted keys for other shapes.

Only one prepared native prover is loaded at a time per process, and each
`LocalProver` runs one proof at a time; another fails with
`WalletError.proverBusy`, and so do the wallet's own proofs while a
`LocalProver` is loaded. Close it before loading another key.
The key is read once by `load`, not on every warm proof.
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

`ZOLANA_API_KEY` is a Helius key: the example reaches Solana devnet RPC through
Helius with it, and Zolana's devnet-e indexer and prover without it. It is
compiled into that build only; never commit it.
Alternatively, set `ZOLANA_RPC_URL` to your Solana devnet RPC.

### From Xcode

Install Flutter; the build hook builds the Rust sources when no release
matches them. Then, with the workspace closed:

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
mkdir -p /tmp/timing/my-phone
adb logcat -d -s flutter | grep -o 'ZOLANA_TIMING {.*' > /tmp/timing/my-phone/timing.log
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
shape and run; the report takes medians, a directory per phone.

CI benches every pull request and every push to `main` on a low, a mid and
a high tier Android phone and iPhone in AWS Device Farm (Samsung Galaxy A16,
A56, S25; iPhone 12, 15, 17 Pro; `scripts/devicefarm/devices.json`,
`.github/workflows/device-bench.yml`). A pull request gets one comment of
the proof time by phone, with the change against the last commit of
`main`; after a push to `main` the report is commented on the commit and
joins the history on the
[`proof-timing`](https://github.com/helius-labs/zolana-mobile-sdk/tree/proof-timing)
branch, a table of the last commits per phone.

`wallet.proveRequest(keyName, requestJson)` is what the harness calls: it
proves any Zolana `/prove` request with a pinned key, and returns the
proof and the stage timings. `tools/prove-fixtures` wrote the fixture
requests, on trees of its own: they prove with the pinned keys and land
nowhere.

## Download catalogs

`WalletConfig.provingKeyCatalog` accepts the JSON contents of the
`/proving-keys` catalog (`keys[].artifacts[]`). Supply it from app configuration
or fetch it separately. The SDK makes no automatic catalog or policy request.
This option and `provingKeyUrl` are mutually exclusive.

The gnark source digest and zopro checksum and size must match the bundled pins.
Downloads use the complete zopro URL, including its prefix and query. HTTPS and
loopback HTTP are accepted. Missing conversions fail without falling back to
another source. The catalog describes downloads, not server routes or supported
circuits. Proofs still report the gnark digest as `provingKeySha256`. Omit the
catalog to use the bundled URLs.

Both SDKs sync the same reviewed upstream zopro manifest, pinned by commit and
SHA-256 in `key-catalog-source.json`. `python3 scripts/sync-key-catalog.py --check`
checks it; omitting `--check` regenerates the snapshot. Converted artifacts keep
their own prefix, which also identifies their local cache, independently of the
gnark manifest's prefix. Compatibility is established by per-key digests.
