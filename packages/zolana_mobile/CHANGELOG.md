## Unreleased

* Add SPL Token and Token-2022 assets: `mint` on deposit, transfer and
  withdrawal, `balances()`, `privateBalance()`, `publicBalance()`,
  `prepareTokenAccount()`, and a mint on each activity entry. Amounts are
  `amount` in base units.
* Add `WalletConfig.rpcHeaders`: extra HTTP headers on every Solana RPC
  request.
* Add `prepareRegistration`, `prepareDeposit`, `prepareTransfer`,
  `prepareWithdrawal`, `submit` and `confirm` for applications that sign and
  send transactions themselves.
* Add `ZolanaWallet.close()` for lock and account switch: it rejects queued
  operations, never signs or submits after the call, drains the running step
  and releases the native wallet and the proving key it loaded.
* Add `registrationStatus()`: `notRegistered`, `registered` or `conflict`.
  `register()` fails with `registration_conflict` when the user record holds
  other keys, instead of replacing them.
* Add `ZolanaWallet`: registration, deposit, sync, private transfer and
  withdrawal, proving on the device with the Solana key held by a
  `SolanaSigner`.
* Add `initZolanaMobile()`, which loads the native library on every platform.
  `RustLib.init()` alone fails on iOS: the pod links the Rust library into the
  plugin framework, not the `mopro_flutter_bindings.framework` it looks for.
* Add `activity` for history.
* Sync before selecting notes to spend and after each confirmed transaction.
* Download proving keys on first use, pinned by the Zolana proving-key lockfile.
* Remove the synthetic `prepareTransfer` draft and seed-based `shieldedAddress`.
* Sync the vendored prover, keys and request schema with Zolana `main`.

## 0.1.0

* Bundle the Rust implementation and patched Mopro native backend in the package.
* Pin Flutter Rust Bridge runtime and generated bindings to the same version.
* Add prepared provers, structured Zolana requests, and canonical proof responses.
* Add explicit job discard, completion, and draining close semantics.
* Reject ambiguous witness values and sanitize backend errors.
* Add isolated-consumer build and native integration tests.

* Add Android and iOS bindings for local Mopro/gnark Groth16 proving.
* Add Poseidon hashing, shielded-address derivation, and demo transfer-draft primitives.
