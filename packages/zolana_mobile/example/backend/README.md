# Sponsored cache merge example

This example connects the Flutter wallet, an authenticated TEE proof proxy and a
sponsor relayer. Mobile initiates each round. It sends encrypted witnesses through
the proxy, builds an unsigned `[CreateCache, MergeTransact]`, and gives its message
to the relayer. The relayer signs and submits it. The final send or withdrawal
reads the cache, is proved locally, and is signed by the user alone.

The proxy does not schedule background merges or hold the user's nullifier key.
The relayer's run API and the proof proxy are separate interfaces, even though
this example hosts them in one process.

## Versions

Build the app and server from the same mobile SDK checkout, including its two
Cargo lockfiles, generated bridge and `native/zolana-mobile/zopro-keys.lock`.
Both use Zolana commit **5cfd7eaed15ef29dc95f6b212cd2844ed0e86860** (#501).
These additions are unreleased; the older mobile pin e30b328a does not expose them.
Use the eventual reviewed mobile commit containing this example as the client
pin; the upstream protocol revision alone is not a mobile SDK release.

The v0.4 sponsored contract must be adapted to the v0.6 transaction/account
layout and cache routes. Do not mix its messages, saved runs or proving artifacts
with this example. Finish or reconcile outstanding runs before changing versions.
Wallet snapshots are versioned and authenticated, not a cross-version migration
format. Rebuild the native library with the Dart bindings. Proving-key hashes
are pinned by the lockfile; remote proofs are also checked against the pinned
key hash and verified locally.

## Run the backend

From the repository root, with a funded **devnet** sponsor keypair in Solana's
64-byte JSON format:

```sh
export SF_AUTH_TOKEN='replace-with-a-random-development-token'
export SPONSOR_KEYPAIR='/absolute/path/to/devnet-sponsor.json'
export RPC_URL='https://your-devnet-rpc'
export PROVER_URL='https://dd73hh473h6ha.cloudfront.net'
export RUN_DIR="$PWD/.example-runs"
cargo run -p zolana-example-backend
```

It listens on `127.0.0.1:8788`. `LISTEN_ADDR` overrides that. Use HTTPS when
exposing it outside the local emulator; keep the sponsor key and journal private.
`PROVER_URL` is the direct prover base, with no credentials or query string.
An optional `PROVER_BEARER_TOKEN` is attached upstream as an Authorization header.
There are no provider API-key parameters on the mobile proof URL. RPC credentials
are independent of proof-proxy authentication.

## Run the Flutter example

From `packages/zolana_mobile/example`:

```sh
flutter run -t lib/main_sponsored.dart \
  --dart-define=ZOLANA_BACKEND_URL=http://127.0.0.1:8788/ \
  --dart-define=ZOLANA_BACKEND_AUTH=replace-with-a-random-development-token \
  --dart-define=ZOLANA_RPC_URL=https://your-devnet-rpc \
  --dart-define=ZOLANA_MERGE_ROUNDS=2 \
  --dart-define=ZOLANA_MERGE_INPUTS=2
```

Use `http://10.0.2.2:8788/` for an Android emulator. A physical device needs an
accessible HTTPS backend. `ZOLANA_INDEXER_URL` defaults to the pinned devnet
indexer; `ZOLANA_TREE_ID` defaults to 0. Set `ZOLANA_MINT` for an SPL token.
The public demo account A needs devnet SOL for registration, shielding and the
final payment, plus tokens when testing a token. The sponsor pays merge network
fees and cache creation. Never put real funds in the published demo accounts.

1. Register / enable merging, then shield demo notes.
2. Start the run. Each round uses distinct original inputs and writes a separate
   slot in the same cache. The example creates two notes per round, so use
   `ZOLANA_MERGE_INPUTS=2` for a deterministic multiple-round demonstration.
3. Select a private send or public withdrawal and prepare the final payment.
   Private recipients must already be registered. Preparation downloads any
   missing **local transfer** key, verifies the cache and proves on the device.
4. Review and sign. The example independently checks the message's outputs,
   cache address/slots and priority fee before asking the user to approve.
5. Restart during merging or before payment approval to exercise restoration.
   After payment submission use **Recover submitted payment**, which checks the
   original signature and can resend its identical signed bytes if the outcome
   is unknown. **Refresh unsigned payment** replaces an expired
   blockhash before signing without extending cache or proof validity.

For smoother first use, prefetch transfer keys with
`wallet.prefetchProvingKeys(maxInputs: rounds)`. Remote merges need no merge key
download on the phone. Use remote merge proving on memory-constrained devices;
no phone memory benchmark for these new cached flows is implied by the example.

## HTTP contract

Every endpoint requires `x-sf-auth`. The demo uses one shared bearer token.

| Endpoint | Request / behavior |
| --- | --- |
| `POST /runs` | `{ "owner": "<base58>", "treeId": 0, "slots": 2 }`; returns `id`, `owner`, `slots`, `target`, `submissions` |
| `POST /runs/{id}/merges` | `{ "slot": 0, "message": "<base64 PreparedTransaction.message>" }`; returns `signature`, `accepted`, `outcome` |
| `GET /runs/{id}` | Returns persisted run and chain signature statuses, in slot order |
| `POST /runs/{id}/close` | Send `{}`; after expiry, closes the cache and returns rent to the sponsor |
| `GET /v1/zolana/tee/v1/attestation?nonce=…` | Passes through attestation and nonce |
| `POST /v1/zolana/prove/merge_8_1` | Encrypted synchronous or queued merge proof; also supports `merge_24_1` and `merge_52_1` |
| `GET /v1/zolana/prove/merge_8_1/status?jobId=…` | Encrypted job status/result; same three merge shapes |

`target` contains `sponsor`, `nonce`, `treeId`, `expiresAt` (Unix seconds),
`slot`, `maxPriorityFeeLamports` and `computeUnitLimit`. Use the assigned slot
for each round. The cache PDA derives from sponsor + nonce through the pinned
SDK's `CacheTarget.cacheAddress()`. Sponsor is both cache writer and merge payer.
The example issues 1–4 slots, a ten-minute expiry, 400,000 merge compute units
and a 10,000-lamport **per-transaction priority fee cap**. Mobile chooses 5,000
before preparing either merge or payment. A nonce is a 63-bit JSON integer in
this native Flutter example; other clients must preserve its precision.

The relayer reconstructs the exact permitted message and compares its bytes:
only CreateCache followed by MergeTransact, the run's sponsor/owner/tree/cache,
assigned slot, bounded expiry, 2–51 inputs and the run's fee/compute limits.
Additional instructions, accounts or signers are rejected. On-chain execution
verifies the proof. A slot's signed transaction is journaled with `fsync` before
sending. Repeating the same request resends identical signed bytes and returns
the same signature; different bytes for that slot return 409. `accepted: false`
is **unknown**, not permission to create a replacement.

This is an integration contract, not an implementation of the client's production
prepayment/quote contract. It has no quote signature, prepayment ledger,
aggregate run budget, per-user authorization, production `feePerNullifier`
policy or guarantee of tree-funded rent. The client's backend must apply those
checks before issuing terms and before signing. The SDK enforces the supplied
cache target and priority cap; it cannot establish promises absent from those
terms. Existing production run terms should be adapted to this interface without
silently dropping their additional restrictions.

Cleanup is separate from the final user payment. Before expiry, closing a cache
requires its writer; adding that instruction would add a sponsor signature.
This example permits cleanup only after expiry and returns rent to the sponsor.
It does not automatically call cleanup or delete journal history.

## TEE and authentication

Mobile uses `proverUrl = <backend>/v1/zolana`, `requireProverTee: true` and
`proverSyncOnly: false`. Requiring TEE explicitly matters for a localhost proxy,
where upstream otherwise permits plaintext. The custom transport scopes
`x-sf-auth` to the proxy origin and path, preserves ciphertext and disables
redirects for authenticated requests. RPC/indexer/key requests do not get it.

The proxy forwards the method, relative route, query, encrypted body and TEE
headers unchanged. POST carries ciphertext in its octet-stream **body**, with
`Zolana-Tee`, `Zolana-Tee-Enc` and either `X-Sync: true` or `X-Async: true`;
encrypted GET uses the `Zolana-Tee-Ciphertext` header. Responses preserve the
TEE headers and status. Rewriting the route or query would break request
authentication. Only the three merge shapes and their status routes are exposed;
transfer-proof routes remain absent.

The SDK first requests a synchronous proof. If the prover queues it (including
its async fallback), the SDK decrypts the job ID and polls the matching merge
status route through this proxy until completion, failure or timeout. The Dart
prepare call completes only when the proof has been received and verified.
Authentication applies to every status request. The pinned SDK starts with short
poll intervals and backs off to three seconds, with a 20-minute queued-job wait
limit and a 30-second status-request timeout. A failed or timed-out proof does not
produce a merge transaction; polling does not resubmit the proof job. Normal
initial-submission retry rules still apply to connection failures and busy
responses. A timed-out job may still finish at the prover; the example does not
cancel it or persist a job handle for resuming after an app restart.

The proxy stays opaque: it cannot decrypt the job ID or produce encrypted status
requests on behalf of mobile. It therefore forwards mobile's encrypted requests
rather than fetching the proof in the background itself. This queued mode uses
mobile-to-proxy status polling; it does **not** satisfy a strict no-polling HTTP
contract. For that contract, set `--dart-define=ZOLANA_PROVER_SYNC_ONLY=true`
(`WalletConfig.proverSyncOnly: true`): queued/asynchronous fallbacks then fail
before any status request reaches the transport. Completing queued proofs without
mobile polling would require a different prover delivery protocol. Chain/cache
confirmation remains separate from proof-job polling.

Attestation verification runs in the pinned SDK on the phone, not in this
proxy: vendor evidence, nonce/key binding, deployment measurements, freshness,
and the policy's key/GPU constraints are checked before witnesses are encrypted.
The example bundles devnet-e's reviewed policy in `example/assets/policies/devnet.json`.
For another deployment, pass its reviewed `TeePolicy` or `{ "deployment": … }`
JSON as `WalletConfig.proverTeePolicy` (the example's `ZOLANA_TEE_POLICY`).
`example/assets/policies/mainnet.json` provides the mainnet policy.
Obtain reviewed measurements and the HPKE key
through your trusted deployment process; do not download a policy from the
prover and treat it as trusted. A policy mismatch fails closed. The app is
responsible for securely distributing and updating this policy.

## Recovery and limitations

`exportPreparedTransaction` preserves message bytes, input nullifiers, output
openings and cache descriptors. Restore it with the same wallet and RPC/indexer
configuration **before preparing new spends**. Snapshots are authenticated to
that wallet/configuration and contain private data; authentication is not
encryption. The example encrypts its run file with AES-GCM and writes it before
each handoff, including the user's signature before submitting the payment.
Production should use device-keystore-backed storage and backup policy. Public
demo seeds do not provide storage confidentiality against someone with the seed.

Cached merge inputs and their predicted tree outputs stay reserved for the run;
restored reservations are retained until explicit release or a settled outcome.
The final payment consumes only the supplied distinct slots of one cache and
asset. The SDK checks the cache account owner/layout, sponsor/writer/tree/expiry
and exact commitments. It does not silently add indexed inputs or multiple caches.

An unknown payment is never automatically refreshed, re-proved or replaced.
Recovery reconciles its saved signature, and on an unknown outcome can resend
only the exact saved message and signature. This covers a crash between saving
and submitting. If that replay fails, recovery checks the original signature
again; a replay failure alone never clears the run. The example retains stalled/failed/expired
runs for inspection rather than automatically abandoning them. An expired or
closed cache cannot be used for a new cached payment. Merges also publish their
outputs to the tree: after resolving earlier submissions and releasing the
appropriate reservations, a fresh ordinary `prepareSpend` can select indexed
outputs. This fallback requires indexer refresh and a new proof/approval; the
example does not automate that recovery decision. Blockhash expiry alone is not
evidence that a previous payment failed.

Run the server as a single process per journal. Its in-process lock is not a
multi-replica database transaction. The shared token, 128-run quota, fixed limits,
local keypair and cleanup endpoint are development conveniences. Do not deploy
this example unchanged as a production sponsorship service.

## Validation

```sh
# Repository root
cargo test --workspace --lib --bins
# packages/zolana_mobile
flutter test
flutter analyze
# packages/zolana_mobile/example
flutter test
```

The ignored native integration test
`sponsored_cache_rounds_restore_and_pay_locally` exercises the actual server,
TEE proofs, two slots, restart restoration and a local payment with both private
and public outputs for SOL and optionally SPL tokens. See its environment setup
in `native/zolana-mobile/tests/wallet_flow.rs`; it submits devnet transactions
and requires funded demo keys. It has been run successfully against the pinned
devnet deployment. This is not a claim of compatibility with the client's staging
proxy or production quote contract.

The opt-in Flutter test runs the same flow through the public Dart API and the
example's encrypted storage and signing checks. It requests queued TEE proofs,
reopens the wallet after the first merge and before payment approval, checks
relayer replay, refreshes the unsigned payment and confirms the recipient's
balance and a cold transaction lookup. It covers both SOL and the demo TEST mint.
With the backend running and the public demo accounts funded, run from
`packages/zolana_mobile/example`:

```sh
flutter test integration_test/sponsored_merge_test.dart -d DEVICE \
  --dart-define=ZOLANA_E2E=true \
  --dart-define=ZOLANA_API_KEY=your-devnet-helius-key \
  --dart-define=ZOLANA_BACKEND_URL=http://127.0.0.1:8788/ \
  --dart-define=ZOLANA_BACKEND_AUTH=replace-with-a-random-development-token
```

Use `http://10.0.2.2:8788/` on an Android emulator. These flags can also be passed
using `--dart-define-from-file` with a private, untracked JSON file.
The prover can return proofs inline when no queue is configured; the test reports
the actual status-request count. Add `--dart-define=ZOLANA_E2E_REQUIRE_QUEUED=true`
to require live polling coverage on a deployment with a queue.
