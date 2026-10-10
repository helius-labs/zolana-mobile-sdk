## 0.1.0

Unreleased.

* `ZolanaWallet`: registration, deposit, private transfer and withdrawal of
  SOL and SPL Token / Token-2022 assets, proving on the device. The Solana
  key stays with the application's `SolanaSigner`.
* The wallet keeps no chain state: `balances()`, `privateBalance()`,
  `activity()` and every spend read the indexer when they run.
  `WalletConfig.mints` lists the SPL mints the wallet holds, each with its
  token program; the wallet reads only their asset ids. A spend takes the
  largest notes first, at most 40 from at most two trees, as the Zolana
  SDK selects them, and fails with `WalletError.mergeRequired` or
  `WalletError.tooManyInputTrees` beyond that and with
  `WalletError.amountZero` for a zero amount.
* `activity()`: the wallet's history, newest first, as the Zolana SDK
  classifies it: `deposit`, `received`, `sent`, `withdrawal` or
  `selfTransfer`.
* `prepareRegistration`, `prepareDeposit`, `prepareTransfer`,
  `prepareWithdrawal`, `submit` and `confirm` for applications that sign and
  send transactions themselves. `feePayer` lets another account pay the
  network fee of a transfer or withdrawal.
* Merges: `setMerging` / `prepareMerging` and `mergingEnabled()` for the
  account's opt-in, and `merge` / `prepareMerge` to combine up to 54 notes of
  one asset on one tree into one, proved on the device or by `proverUrl`.
  A merge needs only the fee payer's signature and expires ten minutes after
  it is prepared.
* `submit` checks the signature's status after a send error that is not the
  chain's verdict, so a transaction that landed is not reported as failed.
  A spend of a note the chain already spent fails with
  `WalletError.notesAlreadySpent`.
* Balances, activity and every spend wait for the indexer to reach the RPC's
  confirmed slot, and fail with `WalletError.indexerBehind` when it does not
  within about ten seconds.
* Proving keys download straight to disk: a key download carries
  `TransportRequest.downloadPath`, the default and example transports write
  the body there as it arrives, and the wallet checks the file from disk.
* `close()` aborts the default transport's requests in flight instead of
  waiting for them; the step they held fails with `WalletError.walletClosed`.
* `registrationStatus()`: `notRegistered`, `registered` or `conflict`.
  `register()` never replaces a record that holds other keys.
* Prepared spends reserve their notes, so the next spend selects others;
  `release(tx)` frees them. `waitForTransaction(signature)` waits for a
  transaction sent before a restart.
* `refresh(tx)`: a prepared transaction with a new blockhash and the same
  proof, for slow approvals. Every prepared transaction carries its `kind`,
  `amount`, `mint`, `recipient`, `feePayer`, `signers`, `message` and
  `lastValidBlockHeight`; the application builds its approval text from
  them, and the `SolanaSigner` receives the transaction it signs.
* `exportKeys()` and `ZolanaWallet.openWithKeys()`: open the wallet from its
  saved derived keys, without a derivation signature; the signer is optional.
* `close()` for lock and account switch: it rejects queued operations and a
  `prepare` call that is proving at the time, never signs or submits after the
  call, and releases the native wallet once the running step ends.
* Backend proving: `Proving.remote` proves a spend with the Zolana SDK's
  prover client at `WalletConfig.proverUrl` (the application's backend, which
  proxies the prover's `/prove/<key>` routes, or a Zolana prover), through the
  wallet's transport. The device verifies the returned proof against the
  pinned verifying key before it builds the message. `WalletConfig.proving`
  sets the default; `proving` on `prepareTransfer`, `prepareWithdrawal`,
  `transfer` and `withdraw` chooses per call.
* The wallet never opens a connection: every request (Solana RPC, indexer,
  proving-key downloads, remote proofs) goes through a transport in Dart, the
  package's own on `package:http` by default, or the application's networking
  passed as `transport` to `ZolanaWallet.open` and
  `ZolanaWallet.openWithKeys`. With the default transport, `open` refuses
  plaintext URLs off loopback unless `allowInsecureHttp` is passed. A
  transport returns each response with its headers, and throws
  `TransportResponseLost` when a body fails after its status arrived, so a
  proof request is not sent twice.
* Proving keys download on first use, pinned by the Zolana proving-key
  lockfile. With the default transport, a download that receives no data for
  30 s fails with `WalletError.provingKeyDownloadFailed`, and the next call downloads
  the key again.
* Errors are `ZolanaWalletException`s whose `error` is a `WalletError` with
  the data of what happened: amounts, accounts, mints. They carry no key
  material, and `api-key` values in them are masked.
* `LocalProver`: prepare a circuit once and prove structured Zolana requests
  repeatedly. `poseidonHash`.
* `initZolanaMobile()` loads the native library on every platform.
* Android native libraries are aligned for 16 KB memory pages.
* Signed precompiled native libraries for Android (API 24+) and iOS (15+);
  building from source with Rust and Go is the fallback.
