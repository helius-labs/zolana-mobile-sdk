# Mopro native gnark backend

Vendored Mopro BN254 Groth16 with strict input validation and reusable
native proving keys. See [PROVENANCE.md](PROVENANCE.md) for pinned sources,
licenses, and fixtures.

## Rust API

Use this directory (not `crates/`) as the Cargo dependency path.

```rust
let prover = rust_gnark::PreparedProver::load(r1cs, pk, vk)?;
let result = prover.prove_request(request_json)?;
assert!(prover.verify(&result)?);
drop(prover);
```

- `PreparedProver::load(r1cs: &str, pk: &str, vk: &str) -> anyhow::Result<Self>`
- `PreparedProver::load_key(key: &str) -> anyhow::Result<Self>` loads an
  upstream Zolana `.key` container (header, proving key, verifying key,
  constraint system) as published, without splitting it. Its header must
  name the eddsa rail and the shape the constraint system has. The
  container is read before its sections can be checked against each other,
  so verify the file against the pinned proving-key lockfile first.
- `prove_request(&self, request_json: &str) -> anyhow::Result<Groth16ProofResult>`
- `verify(&self, result: &Groth16ProofResult) -> anyhow::Result<bool>`
- `Drop` releases the Go handle. The type is `Send`, not `Sync` or `Clone`.

All three `load` paths are mandatory. Prepared proof calls verify using the
loaded VK before returning; callers need not repeat verification.
Load validates PK/VK curve points with `ReadFrom`, preflights the FFT domain
before its parallel precomputation, and checks key/circuit dimensions.
Warm calls never reopen the assets. One prepared handle is permitted per
process; drop it before loading another. IDs are not reused. A Go mutex
serializes all backend operations, including release.

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

Verification consumes only `proof` and `public_inputs`; constructing the
remaining fields with `..Default::default()` is supported. `proof_json` is a
generated output, not a second verification input.

## Inputs

Input is the ordinary Zolana `/prove` request, using the pinned protocol's
hexadecimal strings.
Supported types are `transfer-confidential`, `transfer-ring`,
`transfer-ring-authority`, `merge`, and `merge-ring`.
P256, custom-ring, address-append, and unknown types fail closed.

Structured object fields must have exact spelling and cannot be duplicated.
Every declared field is required, as the Zolana Rust and TypeScript clients
always send them, and the assignment's variable names must equal the key's
names exactly. Fields cannot be null or empty strings, and all field-element
values must be nonnegative and less than the scalar modulus. Numeric arities
must be unsigned integer JSON numbers. Requests are capped at 8 MiB.

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
Go 1.27.1 is selected by default unless GOTOOLCHAIN is explicitly configured.
Pin the same compiler, SDK/NDK, target and environment for reproducible builds.
Original Mopro Apple/Android cross-compiler selection is retained.

```sh
(cd go && go test -mod=readonly -count=1 ./...)
(cd go && go test -mod=readonly -race -short -count=1 ./...)
cargo test --offline
cargo package --allow-dirty --offline
```

To exercise deployed transfer keys, set `ZOLANA_TEST_KEYS` to a directory
containing `transfer_confidential_2_3.{r1cs,pk,vk}`. The staged-key test
checks the witness built from the pinned request against the canonical
flattened vector in `go/testdata/witness-2x3.json`, proves twice using the
retained handle, and verifies native/web JSON interoperability.

On the supplied macOS host, set SDKROOT to Xcode's MacOSX.sdk and CC/AR to
Xcode's explicit clang/ar paths; use `GOFLAGS=-buildvcs=false`.
