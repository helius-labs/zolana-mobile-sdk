# Vendored native prover provenance

## Mopro backend

Base: Mopro commit `c1071f96fa5a28dca8e567dd3944331047450a2b`,
`native/rust-gnark`, copied from the pinned Mopro source checkout.
The Mopro README identifies its original base as FluxePay/rust-gnark
`v0.0.2`, commit `02029a5`. The supplied Mopro Cargo package version is
`0.0.3`. Its MIT LICENSE and UPSTREAM_README.md are preserved.

This remains Mopro's native gnark 0.15.0 / gnark-crypto 0.20.1 BN254
Groth16 backend with rangecheck hints. There is no alternate prover, PLONK
entry point, arithmetic accelerator, or downloaded native binary.

Local changes replace the unsafe JSON/FFI boundary, add prepared key
lifecycle and request assignment, retain safe compatibility functions, and
make source builds mandatory. Cargo.toml moved from crates/ to this root
so published Cargo packages contain their Go source and local module.
crates/build.rs, crates/src/lib.rs, go/wrapper.go, go/go.mod, go/go.sum,
Cargo.toml, and README.md differ from Mopro. New files are listed in FILES.md.
The package declares MIT AND Apache-2.0 because it includes both sources.

## Pinned Zolana protocol assignment

Source repository: https://github.com/helius-labs/zolana-web-sdk

Commit: `ef2fa67fe8e9fe4b123169e95a742ca192f454b5`.

`go/protocol/` vendors the selected dependency closure from `wasm/`,
module `zolana/prover`. Its Go implementation files are copied without
semantic changes. go.mod is reduced to the dependencies used by this
closure; the gnark-lean-extractor replacement remains pinned to
`github.com/Lightprotocol/gnark-lean-extractor/v3`
`v3.0.0-20250920122823-aa0219463107`.
The original repository's Apache-2.0 LICENSE is go/protocol/LICENSE.
Source-path and SHA-256 inventory: go/protocol/UPSTREAM_FILES.sha256.

Only the native bridge dispatch, strict validation, and shape-name matching
are new. TransferParameters.UpdateWithJSON/CreateWitness,
MergeParameters.UpdateWithJSON/CreateWitness, their ValidateShape methods,
and common.Proof.MarshalJSON are the pinned protocol implementation.
Unused upstream common serialization helpers remain in their source file;
the native bridge never invokes their UnsafeReadFrom methods or downloads
keys. Native asset loading uses validated ReadFrom exclusively.

## Existing key name compatibility

Staged transfer keys predate the Zone-to-Ring source rename. The provenance
of the exact spelling-only mapping is Zolana commit
`5ff5aebf67bd73ec64aa14d438b8b5be7a7c30a1`,
`chore: rename zone to ring (#181)`, reviewed in
`https://github.com/helius-labs/zolana`.
In go/request.go, matchingWitnessNames recognizes only whole path segments
ZoneDataHash -> RingDataHash, ZoneProgramID -> RingProgramID, and
OutputZoneDataHash -> OutputRingDataHash. Variable counts, visibility,
array positions, and ordering still must match. No witness value is
modified by this compatibility check. Flattened requests must use the
literal names actually embedded in the key.

## Regression fixtures

- go/testdata/transfer-2x3.json is the pinned web repository's
  examples/browser/public/fixtures/transfer-2x3.json, also byte-identical to
  the supplied mobile prove-request-2x3.json before its trailing newline.
  Original SHA-256:
  `074f0e453ac8a50063118521e754f418f036a31f7887266fe3f7f3d8ea6d9da5`.
- go/testdata/witness-2x3.json is the supplied staged mobile flattened
  fixture. Its two NullifierNextValue entries are changed from the legacy
  signed representation "-1" to canonical scalar modulus minus one:
  `21888242871839275222246405745257275088548364400416034343698204186575808495616`.
  This agrees with the structured request's hexadecimal field values.
  The decoder itself rejects negative values; it never normalizes them.
- go/merge_fixture_test.go is pinned
  wasm/circuits/spp_merge/fixture_test.go with only its package declaration
  changed from merge_test to main. The fixture construction and its
  prover-test dependency closure remain the pinned implementation.

Tests create ephemeral proving systems locally and optionally validate the
staged deployed transfer key. No proving keys, downloaded binaries, or real
user witnesses are included in this vendor subtree.
