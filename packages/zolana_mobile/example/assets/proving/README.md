Place the staged Mopro gnark assets in this directory.

From the repository root, run:

```sh
./scripts/stage-demo-assets.sh
```

The script downloads and verifies `transfer_confidential_2_3.key`, splits its
`.pk`, `.vk`, and `.r1cs` sections at locked byte offsets, and copies the
committed witness JSON. All staged assets are ignored by Git.
