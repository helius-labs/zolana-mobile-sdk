# Zolana mobile demo

The wallet proves transfers and withdrawals locally with at most eight input
notes (`WalletConfig.maxSpendInputs: 8`). If an amount plus its app fee needs
more notes, the payment sheet asks you to merge first.

Choose **Merge with TEE**, turn on **Allow merges**, then merge the selected
asset. The SDK attests devnet-e's prover and encrypts the request to its enclave.
A merge combines up to 24 notes on one tree without changing the private balance;
public SOL pays network fees. Close the sheet to refresh balances and history,
then retry the payment. Repeat merges if needed. Turn **Allow merges** off to
opt out on chain. Merges appear in history as self transfers.

Build the wallet with `--dart-define=ZOLANA_API_KEY=...` for the devnet RPC,
or supply a Solana devnet RPC with `--dart-define=ZOLANA_RPC_URL=...`.
The Network menu includes the TEE prover URL, which defaults to
`https://dd73hh473h6ha.cloudfront.net`. Transfers continue using local proving;
only the merge action requests `Proving.remote`.

The **Proof benchmark** screen also generates and verifies a Groth16 proof on the device with zopro, from
a public structured Zolana 2→2 request fixture. It loads the key once per
session and shows key loading, witness preparation, proving, and verification
timings separately.

From the repository root:

```sh
./scripts/stage-demo-assets.sh
cd packages/zolana_mobile/example
flutter run --release -d YOUR_DEVICE_ID
```

The proving key is large and ignored by Git. The staging script downloads one
checksum-locked zopro key.

Use **Lock / discard** to reject a pending result. Backgrounding the app also
locks the demo. The native proof finishes before its keys are released; unlock
waits for that drain before creating a new session. This demonstrates lifecycle
handling, not wallet authentication. No private witness is written to disk.
