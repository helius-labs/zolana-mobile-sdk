# Zolana mobile demo

This app generates and verifies a Groth16 proof on the device with zopro, from
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
