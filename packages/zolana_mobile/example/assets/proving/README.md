Place the staged proving key and solved assignment in this directory.

From the repository root, run:

```sh
./scripts/stage-demo-assets.sh
```

The script downloads and verifies `transfer_confidential_2_3.key` when needed,
copies the committed `assignment-2x3.bin` fixture, and stages both here. The
staged binary assets are ignored by Git.
