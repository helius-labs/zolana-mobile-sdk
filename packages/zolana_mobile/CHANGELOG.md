## 0.1.0

Unreleased.

* Add sponsored cache merges: `prepareMerge(cacheTarget: ...)` builds unsigned
  CreateCache + MergeTransact; `prepareSpend(cachedInputs: ...)` consumes verified
  cache slots with a local proof. The sponsor signs merges and the user signs
  the final payment. Both paths enforce the supplied priority fee cap.
* Add wallet-authenticated prepared-transaction export/restore, retaining cache
  descriptors, output openings and pending reservations across restarts.
* Add explicit TEE enforcement/custom trusted policy and synchronous-only remote
  proving. The example also supports encrypted queued-proof status polling.
  Include an `x-sf-auth` proxy, validating sponsor relayer, persistent
  run journal and Flutter sponsored-flow example for the pinned v0.5 protocol.
* Recognize cached spends in cold transaction lookup and the example's independent
  output checks. Validate SOL and SPL cache rounds and locally proved final
  send/withdrawal against devnet.


* Fix historical transaction recovery in `waitForTransaction`, `confirm` and
  lost-send handling: search RPC history and accept matching indexed events.
  Missing history remains `transactionOutcomeUnknown` after blockhash expiry;
  the SDK no longer reports it as `transactionExpired`.

* `ZolanaWallet.transaction(signature)` fetches and decrypts one signature
  without history pagination. Its result distinguishes ready (possibly empty),
  pending indexing, not found and incomplete details. Outgoing reads can also
  fetch confirmed public settlements and recipient/asset metadata.
* Activity rows now include a stable `index`: the pool's asset id, consistent
  across signature lookup, `activity()` and `overview()`.

* A prepared spend exposes its output openings, including change and padding,
  so an application's signer can recompute commitments and check the message.
  The example performs these checks before signing. Seeds stay inside the SDK.

* Built on Zolana v0.5.0-alpha, which devnet-d runs: shielded pool
  `spp3CfN2Y7se74A27bBpnAkmjXyEb2YmmZafhFrC9xt`, user registry
  `regy3WAeRXMYuMYxr5iK3Bf9Arr3PcxGxyzAddbiqDQ`. The viewing key is an X25519
  key.
* Remote proof requests go only to a prover that attests to the enclave the
  Zolana SDK pins, encrypted to it, the Helius gateway included; only a
  loopback prover gets plaintext.
* On Android the package requests `android.permission.INTERNET`, which
  Flutter's template grants only to debug and profile builds: a release
  app without it could not download a proving key.
* `ZolanaWallet`: registration, deposit and spends of SOL and SPL Token /
  Token-2022 assets, proving on the device. The Solana key stays with the
  application's `SolanaSigner`.
* The wallet keeps no chain state: `balances()`, `privateBalance()`,
  `activity()` and every spend read the indexer when they run.
  The wallet holds every mint the shielded pool registered, reading a mint's
  asset id and token program when a call names it or a read meets its
  notes; `WalletConfig.mints` limits it to the listed mints, each with its
  token program, and then reads only their asset ids. A mint the pool has
  not registered yet reads as no balance and no history, and its first
  deposit registers it in the same transaction:
  `Operation.deposit.createsSplInterface` shows the extra rent. A spend takes the
  largest notes first, at most 40 from at most two trees, as the Zolana
  SDK selects them, and fails with `WalletError.mergeRequired` or
  `WalletError.tooManyInputTrees` beyond that and with
  `WalletError.amountZero` for a zero amount.
* `WalletConfig.maxSpendInputs` caps the notes a spend takes on the device
  (40 when unset); a spend that needs more fails with
  `WalletError.mergeRequired` before proving.
* `TokenBalance` reports `amount`, `reserved` included; `notes`, how many
  the amount is in; `reserved`, what this wallet's prepared spends hold; and
  `limits`, the most one spend of 2, 4, 8 or 16 outputs can take now without
  a merge, from the unreserved notes within the width's note cap and two
  trees. A spend with `k` private outputs uses `k + 1` outputs, public
  outputs take no slot.
* `WalletConfig.reuseNotesMs` lets a spend take its notes from a read made
  within that window instead of reading the indexer again; a send or a
  confirmation ends the reuse. A spend's other lookups, the recipient's
  registration or token account, run beside the read, and the wait for
  the indexer to reach the RPC's slot is asked again after a tenth of a
  second at first.
* `activity()`: the wallet's history, newest first, as the Zolana SDK
  classifies it: `deposit`, `received`, `sent`, `withdrawal` or
  `selfTransfer`.
* `overview()`: the balances and the history from one read of the indexer,
  for a screen that shows both.
  `unresolvedAssets` lists asset ids and mints that remain unresolved after
  refreshing the pool's registrations. Their notes are excluded from balances
  and history, so non-empty lists mean the portfolio is incomplete.
* `spend` / `prepareSpend` take `outputs` of one asset, proved together in
  one transaction: `Output.private`, a note for a registered wallet with its
  amount hidden and the note tagged with the recipient's Solana address on
  chain, and `Output.public`, funds to a Solana account with recipient and
  amount public, tokens to its associated token account, which the spend
  creates when it does not exist, the fee payer paying its rent. The
  application's fee is one more private output to its registered collector.
  `Operation_Spend` lists the outputs in order and the recipients whose
  token account it creates. An empty list fails with `WalletError.noOutputs`.
* `prepareRegistration`, `prepareDeposit`, `prepareSpend`, `submit` and
  `confirm` for applications that sign and send transactions themselves.
  `feePayer` lets another account pay the network fee of a spend.
* Priority fees: `WalletConfig.priorityFeeLamports`, and
  `priorityFeeLamports` on every `prepare` method and on `register`,
  `deposit`, `spend`, `setMerging` and `merge`, set a total fee per transaction
  in lamports. A call overrides the configured fee, including with zero.
  `PreparedTransaction.priorityFeeLamports` reads the fee from the message
  header before signing; `refresh` preserves it.
* Merges: `setMerging` / `prepareMerging` and `mergingEnabled()` for the
  account's opt-in, and `merge` / `prepareMerge` to combine up to 52 notes of
  one asset on one tree into one, proved on the device or by `proverUrl`.
  A merge needs only the fee payer's signature and expires ten minutes after
  it is prepared.
* `submit` looks the signature up after a send error that is not the chain's
  verdict, using RPC history and indexed events. It returns
  `WalletError.transactionOutcomeUnknown` when neither source establishes
  an outcome, including after blockhash expiry.
  A spend of a note the chain already spent fails with
  `WalletError.notesAlreadySpent`.
* Balances, activity and every spend wait for the indexer to reach the RPC's
  confirmed slot, and fail with `WalletError.indexerBehind` when it does not
  within about ten seconds.
* Proving keys download in ranged parts of 8 MB, each written to
  `TransportRequest.downloadPath` as it arrives, so a key never sits in
  memory and an interrupted download resumes where it stopped. One key
  downloads once however many calls need it. A checked key is recorded beside
  it and not hashed again until its file changes. The keys of one lockfile
  live in a directory of their own under `provingKeyDir`; opening a wallet
  removes those of other lockfiles.
* `prefetchProvingKeys(maxInputs, outputs, maxMergeInputs)` downloads the
  keys of those spends and merges ahead of time with a progress stream, and
  stops when its subscription is cancelled; `provingKeys()` lists the keys
  and how much of each is on the device; `clearProvingKeys()` removes them.
* Proving keys load once per device: the first load writes the key's
  image beside it (`<key>.zopro.zopro-raw-3.solver2`: its points in the
  device's memory layout and its solver), and later loads map that file
  instead of decompressing the key, so a restart costs no key preparation
  and the system may drop the points' pages under memory pressure rather
  than the app holding them.
* `proveRequest(keyName, requestJson)` proves a Zolana `/prove` request
  with a pinned key through the wallet's key store and reports where the
  time went: the key's download, its load, the witness, the proof and
  its verification. The example's `proof_timing_test.dart` times the
  fixture requests of 1 to 24 notes and a merge of 24 with it, on CI and
  on phones, and `scripts/proof-timing-report.py` writes the report.
* `close()` aborts the default transport's requests in flight instead of
  waiting for them; the step they held fails with `WalletError.walletClosed`.
* `registrationStatus()`: `notRegistered`, `registered` or `conflict`.
  `register()` never replaces a record that holds other keys.
* Prepared spends reserve their notes, so the next spend selects others;
  `release(tx)` frees them, and so does a send with a verdict: landed,
  refused by the chain, or a note already spent. A send whose
  outcome is unknown keeps them. `waitForTransaction(signature,
  lastValidBlockHeight:)` waits for a transaction sent before a restart and
  ends confirmed, refused or unknown, as `submit` does.
* `refresh(tx)`: a prepared transaction with a new blockhash and the same
  proof, for slow approvals. Every prepared transaction carries its
  `operation`, `feePayer`, `signers`, `message` and `lastValidBlockHeight`.
  `operation` has one variant per kind, each with its own fields
  (`Operation_Spend` its `mint`, `outputs` and `createdTokenAccounts`,
  `Operation_Merging` whether it
  turns merges on,
  `Operation_Custom` the top-level programs of a transaction the application
  composes, which the wallet does not prepare yet); the application builds
  its approval text from it, and the `SolanaSigner` receives the transaction
  it signs.
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
  sets the default; `proving` on `prepareSpend` and `spend` chooses per
  call.
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
  lockfile. With the default transport, a part that receives no data for
  30 s fails with `WalletError.provingKeyDownloadFailed`.
* Errors are `WalletError`s with the data of what happened: amounts,
  accounts, mints. They carry no key material, and `api-key` values in them
  are masked.
* `LocalProver`: load a circuit once and prove structured Zolana requests
  repeatedly, one at a time; `close()` fails a proof in progress with
  `WalletError.proverClosed`. `poseidonHash`.
* `initZolanaMobile()` loads the native library on every platform.
* Android native libraries are aligned for 16 KB memory pages.
* Signed precompiled native libraries for Android (API 24+) and iOS (15+);
  building from source with Rust and Go is the fallback.
* iOS: a build hook bundles the native library as
  `mopro_flutter_bindings.framework`, so apps build with Swift Package
  Manager or CocoaPods.
