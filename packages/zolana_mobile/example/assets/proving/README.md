Place the staged proving assets in this directory.

From the repository root, run:

```sh
./scripts/stage-demo-assets.sh
```

The script downloads and checks `transfer_confidential_2_2.zopro`, the zopro
key of the 2 -> 2 transfer, and copies the committed proof requests. All
staged assets are ignored by Git.
