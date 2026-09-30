## Unreleased

* Add `initZolanaMobile()`, which loads the native library on Android and iOS.
  `RustLib.init()` alone fails on iOS: the pod links the Rust library into the
  plugin framework, not the `mopro_flutter_bindings.framework` it looks for.

## 0.1.0

* Bundle the Rust implementation and patched Mopro native backend in the package.
* Pin Flutter Rust Bridge runtime and generated bindings to the same version.
* Add prepared provers, structured Zolana requests, and canonical proof responses.
* Add explicit job discard, completion, and draining close semantics.
* Reject ambiguous witness values and sanitize backend errors.
* Add isolated-consumer build and native integration tests.

* Add Android and iOS bindings for local Mopro/gnark Groth16 proving.
* Add Poseidon hashing, shielded-address derivation, and demo transfer-draft primitives.
