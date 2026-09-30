## Unreleased

* Add `initZolanaMobile()`, which loads the native library on Android and iOS.
  `RustLib.init()` alone fails on iOS: the pod links the Rust library into the
  plugin framework, not the `mopro_flutter_bindings.framework` it looks for.
* Download signed precompiled native libraries for Android and iOS, so apps
  build without Rust or Go. Building from source stays the fallback.
* Include the path dependencies, including the Go prover sources, in the
  cargokit crate hash.
* Declare the Flutter platform minimums: Android API 24 (was 21) and iOS 15
  (was 11).
* CI runs the clean consumer on an Android API 26 emulator and checks that the
  iOS native library runs on iOS 16.

## 0.1.0

* Bundle the Rust implementation and patched Mopro native backend in the package.
* Pin Flutter Rust Bridge runtime and generated bindings to the same version.
* Add prepared provers, structured Zolana requests, and canonical proof responses.
* Add explicit job discard, completion, and draining close semantics.
* Reject ambiguous witness values and sanitize backend errors.
* Add isolated-consumer build and native integration tests.

* Add Android and iOS bindings for local Mopro/gnark Groth16 proving.
* Add Poseidon hashing, shielded-address derivation, and demo transfer-draft primitives.
