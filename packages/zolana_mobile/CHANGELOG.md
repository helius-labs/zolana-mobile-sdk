## Unreleased

* Add `ZolanaWallet`: registration, deposit, sync, private transfer and
  withdrawal, proving on the device with the Solana key held by a
  `SolanaSigner`.
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
