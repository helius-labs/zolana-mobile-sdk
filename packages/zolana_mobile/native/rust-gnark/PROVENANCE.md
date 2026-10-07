# Vendored native prover provenance

## Mopro backend

Base: Mopro commit `c1071f96fa5a28dca8e567dd3944331047450a2b`,
`native/rust-gnark`, copied from the pinned Mopro source checkout.
The Mopro README identifies its original base as FluxePay/rust-gnark
`v0.0.2`, commit `02029a5`. The supplied Mopro Cargo package version is
`0.0.3`. Its MIT LICENSE and UPSTREAM_README.md are preserved.

Mopro's native BN254 Groth16 backend with rangecheck hints, built against
gnark 0.16.3 / gnark-crypto 0.21.0 so it reads proving keys produced by the
pinned Zolana prover (Mopro's base was gnark 0.15.0 / gnark-crypto 0.20.1).
There is no alternate prover, PLONK entry point, arithmetic accelerator, or
downloaded native binary.

Local changes replace the unsafe JSON/FFI boundary, add prepared key
lifecycle and request assignment, retain safe compatibility functions, and
make source builds mandatory. Cargo.toml moved from crates/ to this root
so published Cargo packages contain their Go source and local module.
crates/build.rs, crates/src/lib.rs, go/wrapper.go, go/go.mod, go/go.sum,
Cargo.toml, and README.md differ from Mopro. New files are listed in FILES.md.
The package declares MIT AND Apache-2.0 because it includes both sources.

## Pinned Zolana protocol assignment

Source repository: https://github.com/helius-labs/zolana

Commit: the `[workspace.metadata.upstream] revision` in the root Cargo.toml,
which the Rust `zolana-*` git dependencies pin as well. It must be a commit on
`main`.

`go/protocol/` vendors the selected dependency closure of
`prover/server/prover/transfer_eddsa_only` and `prover/server/prover/merge`
from that commit, module `zolana/prover`, plus the `prover-test` packages the
merge fixture test needs. Every Go file is copied byte for byte from
`prover/server/<same path>`; unused files of a package (key download, lazy
key management, setup, and the GPU backend behind the `aeglos` build tag)
are omitted. go.mod is reduced to the dependencies
used by this closure at the same gnark and gnark-crypto versions as upstream;
the gnark-lean-extractor replacement remains pinned to
`github.com/Lightprotocol/gnark-lean-extractor/v3`
`v3.0.0-20250920122823-aa0219463107`.
The original repository's Apache-2.0 LICENSE is go/protocol/LICENSE.
Source-path and SHA-256 inventory: go/protocol/UPSTREAM_FILES.sha256.
`scripts/check-upstream.sh` verifies every inventory entry against the pinned
commit, that no vendored file is missing from the inventory, that the gnark
versions and Rust pins agree, and that the staged proving key matches the
upstream proving-key lockfile. CI runs it on every push.

Only the native bridge dispatch and strict validation are new: every field of
the structured request is required, exactly as the Rust and TypeScript
Zolana clients always emit it, and the assignment's variable names must equal
the key's names exactly. TransferParameters.UpdateWithJSON/CreateWitness,
MergeParameters.UpdateWithJSON/CreateWitness, their ValidateShape methods,
and common.Proof.MarshalJSON are the pinned protocol implementation.
Unused upstream common serialization helpers remain in their source file;
the native bridge never invokes their UnsafeReadFrom methods or downloads
keys. Native asset loading uses validated ReadFrom exclusively.

## Regression fixtures

- go/testdata/transfer-2x2.json is a deterministic, satisfying
  `transfer-confidential` 2→2 `/prove` request built with the pinned revision's
  own Go test helpers (`prover-test/spp`), the request zolana-web-sdk generates
  as `wasm/prover-wasm/testdata/transfer-2x2.json`. Keys and blindings are test
  values. fixtures/prove-request-2x2.json is the same bytes plus a trailing
  newline, and fixtures/prove-response-2x2.json is the devnet Helius prover's
  proof of it with the pinned `transfer_confidential_2_2.key`.
- go/testdata/witness-2x2.json is that request
  assigned onto the staged `transfer_confidential_2_2` constraint system and
  flattened to canonical decimal strings keyed by the key's variable names.
- go/merge_fixture_test.go is the pinned
  prover/server/circuits/spp_merge/fixture_test.go with only its package
  declaration changed from merge_test to main. The fixture construction and its
  prover-test dependency closure remain the pinned implementation.

Tests create ephemeral proving systems locally and optionally validate the
staged deployed transfer key. No proving keys, downloaded binaries, or real
user witnesses are included in this vendor subtree.
