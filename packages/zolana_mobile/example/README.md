# Zolana mobile demo

This app generates and verifies a Groth16 proof locally from staged 2→3
fixtures. The underlying Dart package still exposes the Zolana transaction
bindings for SDK consumers.

From the repository root:

```sh
./scripts/stage-demo-assets.sh
cd packages/zolana_mobile/example
flutter run
```

The proving key is large and ignored by Git. See `../../../README.md` for the
SDK architecture and current limitations.
