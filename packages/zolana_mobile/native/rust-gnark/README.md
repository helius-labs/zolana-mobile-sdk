# Mopro native gnark 0.15 backend

Vendored Mopro BN254 Groth16 with strict input validation and reusable
native proving keys. See [PROVENANCE.md](PROVENANCE.md) for pinned sources,
licenses, the limited historical key-name aliases, and fixture changes.

## Rust API

Use this directory (not `crates/`) as the Cargo dependency path.

```rust
let prover = rust_gnark::PreparedProver::load(r1cs, pk, vk)?;
let result = prover.prove_request(request_json)?;
assert!(prover.verify(&result)?);
drop(prover);
```

- `PreparedProver::load(r1cs: &str, pk: &str, vk: &str) -> anyhow::Result<Self>`
- `prove(&self, witness_json: &str) -> anyhow::Result<Groth16ProofResult>`
- `prove_request(&self, request_json: &str) -> anyhow::Result<Groth16ProofResult>`
- `verify(&self, result: &Groth16ProofResult) -> anyhow::Result<bool>`
- `Drop` releases the Go handle. The type is `Send`, not `Sync` or `Clone`.

All three load paths are mandatory. Prepared proof calls verify using the
loaded VK before returning; callers need not repeat verification.
Load validates PK/VK curve points with `ReadFrom`, preflights the FFT domain
before its parallel precomputation, and checks key/circuit dimensions.
Warm calls never reopen the assets. One prepared handle is permitted per
process; drop it before loading another. IDs are not reused. A Go mutex
serializes all backend operations, including release and compatibility calls.

`Groth16ProofResult` derives `Debug, Clone, Default` and contains:

| Field | Type | Meaning |
| --- | --- | --- |
| proof | String | Hex gnark compressed binary proof |
| public_inputs | String | Hex binary public witness |
| proof_json | String | Pinned common.Proof JSON: ar, bs, krs; optional BSB22 fields |
| inputs, outputs | u32 | Circuit slot counts, valid only if shape_known |
| shape_known | bool | Whether names identify a Zolana transfer/merge shape |
| witness_ms | u64 | Request parsing and witness construction |
| prove_ms | u64 | Groth16 proving only |
| verify_ms | u64 | Prepared proof's mandatory native verification |

Durations use monotonic elapsed time truncated to milliseconds, so sub-ms
operations report zero. They exclude serialization, key loading, C/Rust
copies, and time queued behind the backend mutex. Generic gnark circuits
have `shape_known=false`; wallet core must reject that rather than treating
zero placeholder counts as a Zolana shape.

`groth16_prove(r1cs, pk, witness_json)` and
`groth16_verify(r1cs, vk, &result)` remain available. The legacy prove call
has no VK and therefore cannot self-verify. Verification consumes only
`proof` and `public_inputs`; constructing the remaining fields with
`..Default::default()` is supported. `proof_json` is a generated output,
not a second verification input.

## Inputs

Flattened input is one object mapping every embedded public/secret variable
name (except the constant wire) to a decimal JSON STRING. Accepted integers
match `0|[1-9][0-9]*` and are smaller than the BN254 scalar modulus.
Numbers including `3` and `3.9`, fractional strings, negatives, signs,
whitespace in values, prefixes, leading zeroes, out-of-range integers,
duplicate/unknown/missing names, trailing JSON, nulls and containers fail
with the fixed error `invalid witness`.

Structured input is the ordinary Zolana `/prove` request, using pinned
protocol hexadecimal strings rather than flattened decimal strings.
Supported types are `transfer-confidential`, `transfer-ring`,
`transfer-ring-authority`, `merge`, and `merge-ring`.
P256, custom-ring, address-append, and unknown types fail closed.

Structured object fields must have exact spelling and cannot be duplicated.
All declared fields are required except these non-assigned/rail-unused fields:

- Transfer input/output `isDummy` metadata (the circuit uses UTXO domain).
- Default confidential transfer's top-level `ringProgramId` and each output's
  `ownerPkHash` (the assignment uses published owner hashes instead).
- Ring-authority's `publishedOutputOwnerPkHashes`, output `ownerPkHash`,
  and output `nullifierPk`.
- Default merge's `ringProgramId` and `outputRingDataHash`.

Omitted optional fields retain the pinned assignment defaults. Present
fields cannot be null or empty strings, and all field-element values must
remain nonnegative and less than the scalar modulus. Numeric arities must
be unsigned integer JSON numbers. Requests are capped at 8 MiB.

## Privacy boundary

gnark logging is disabled during Go package initialization, before any
entry point. Public errors are fixed codes/messages; they never contain
caller keys, values, paths, solver constraints, or panic payloads.
Recoverable panics are caught before crossing C. Rust interior-NUL errors
are sanitized too. C result allocations have matching Rust RAII owners.

The caller must still integrity-check trusted circuit/key assets. This is
not a sandbox for arbitrary attacker-created constraint systems, and fatal
runtime failures such as out-of-memory cannot be recovered. Go GC releases
unreferenced keys/witnesses; this API does not promise memory zeroization.

## Build and test

Source is always built; prebuilt directories and download URLs are not
used. Cargo includes `go/**`, including the local protocol module. The
build script tracks `crates/../go` recursively, uses `-trimpath`,
`-buildvcs=false`, and `-mod=readonly`, and keeps Go bounds checks enabled.
Go 1.25.7 is selected by default unless GOTOOLCHAIN is explicitly configured.
Pin the same compiler, SDK/NDK, target and environment for reproducible builds.
Original Mopro Apple/Android cross-compiler selection is retained.

```sh
(cd go && go test -mod=readonly -count=1 ./...)
(cd go && go test -mod=readonly -race -short -count=1 ./...)
cargo test --offline
cargo package --allow-dirty --offline
```

To exercise deployed transfer keys, set `ZOLANA_TEST_KEYS` to a directory
containing `transfer_confidential_2_3.{r1cs,pk,vk}` and `witness-2x3.json`.
The staged-key test checks the pinned structured fixture against the
canonical flattened vector, proves twice using the retained handle, checks
the compatibility API, and verifies native/web JSON interoperability.

On the supplied macOS host, set SDKROOT to Xcode's MacOSX.sdk and CC/AR to
Xcode's explicit clang/ar paths; use `GOFLAGS=-buildvcs=false`.
