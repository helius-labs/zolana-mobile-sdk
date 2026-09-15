# Zolana mobile demo

This app generates and verifies a Groth16 proof locally through Mopro's native
gnark adapter using staged Zolana 2→3 fixtures. The Dart package also exposes
the Zolana transaction bindings for SDK consumers.

From the repository root:

```sh
./scripts/stage-demo-assets.sh
cd packages/zolana_mobile/example
flutter run
```

The circuit and proving key are large and ignored by Git. The staging script
downloads one checksum-locked packed key and prepares the Mopro files; users do
not run `mopro init` or `mopro build`.
