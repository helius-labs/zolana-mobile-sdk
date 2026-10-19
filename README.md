# Proof timing on main

Written by CI after every push to `main` (`scripts/proof-timing-history.py` on `main`), from release builds on the phones of `scripts/devicefarm/devices.json` in AWS Device Farm. Two runs of one commit differ by up to 5 % on the Galaxy A16 and A56 and up to 15 % on the S25 and the iPhones. The data is in `history/`, a line per commit.

## Samsung Galaxy A16, Android 14, low tier

**Before:** [`00b2dbe`](https://github.com/helius-labs/zolana-mobile-sdk/commit/00b2dbe2265c90ce984c6634e5227b65027730c2) of 2026-10-19. **After:** [`9bde901`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bde901088fa31b64651906cdbefcf755372c6f8) of 2026-10-19.

| by scenario | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| first run, including download | before | 5.55 s (download: 2.73 s, 8.8 MiB) | 11.47 s (download: 6.99 s, 15.6 MiB) | 32.26 s (download: 23.88 s, 28.9 MiB) | 39.83 s (download: 23.49 s, 55.9 MiB) | 79.80 s (download: 49.34 s, 109.5 MiB) | 126.32 s (download: 75.51 s, 171.6 MiB) | 114.31 s (download: 66.86 s, 165.0 MiB) |
| | after | 5.26 s (download: 2.45 s, 8.8 MiB) (-5 %) | 11.52 s (download: 7.01 s, 15.6 MiB) | 21.63 s (download: 13.28 s, 28.9 MiB) (-33 % **!**) | 40.06 s (download: 24.02 s, 55.9 MiB) (+1 %) | 77.80 s (download: 47.13 s, 109.5 MiB) (-2 %) | 126.77 s (download: 75.48 s, 171.6 MiB) | 119.31 s (download: 73.09 s, 165.0 MiB) (+4 %) |
| warm | before | 1.25 s | 2.11 s | 3.73 s | 6.99 s | 14.90 s | 25.63 s | 22.86 s |
| | after | 1.23 s | 2.14 s | 3.73 s | 7.16 s (+2 %) | 15.01 s (+1 %) | 25.57 s | 22.27 s (-3 %) |
| first run after restart | before | 1.19 s | 2.23 s | 3.87 s | 7.47 s | 15.73 s | 26.60 s | 23.12 s |
| | after | 1.21 s | 2.14 s (-4 %) | 3.99 s (+3 %) | 7.31 s (-2 %) | 15.69 s | 26.41 s (-1 %) | 22.79 s (-1 %) |
| repeated after restart | before | 1.22 s | 2.09 s | 3.68 s | 7.18 s | 15.28 s | 23.80 s | 22.39 s |
| | after | 1.25 s | 2.06 s | 3.73 s (+1 %) | 7.17 s | 15.11 s (-1 %) | 25.72 s (+8 %) | 22.46 s |

| by stage | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| key load | before | 0.06 s | 0.11 s | 0.14 s | 0.20 s | 0.34 s | 0.46 s | 0.78 s |
| | after | 0.07 s | 0.10 s | 0.14 s | 0.24 s | 0.36 s | 0.71 s (+56 % **!**) | 0.45 s (-43 % **!**) |
| witness | before | 0.02 s | 0.03 s | 0.06 s | 0.11 s | 0.18 s | 0.21 s | 0.18 s |
| | after | 0.02 s | 0.04 s | 0.07 s | 0.11 s | 0.21 s | 0.37 s (+70 % **!**) | 0.14 s |
| prove | before | 1.19 s | 2.04 s | 3.65 s | 7.03 s | 14.89 s | 24.47 s | 22.16 s |
| | after | 1.20 s | 2.00 s | 3.65 s | 7.01 s | 14.83 s | 25.25 s (+3 %) | 22.09 s |

### Proof, last 4 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`9bde901`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bde901088fa31b64651906cdbefcf755372c6f8) | 2026-10-19 | 1.20 s | 2.00 s | 3.65 s | 7.01 s | 14.83 s | 25.25 s | 22.09 s |
| [`00b2dbe`](https://github.com/helius-labs/zolana-mobile-sdk/commit/00b2dbe2265c90ce984c6634e5227b65027730c2) | 2026-10-19 | 1.19 s | 2.04 s | 3.65 s | 7.03 s | 14.89 s | 24.47 s | 22.16 s |
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 1.18 s | 2.02 s | 3.66 s | 7.07 s | 14.96 s | 24.88 s | 22.09 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 1.19 s | 2.01 s | 3.69 s | 6.99 s | 14.92 s | 23.81 s | 21.72 s |

### First run after a restart, last 4 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`9bde901`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bde901088fa31b64651906cdbefcf755372c6f8) | 2026-10-19 | 1.21 s | 2.14 s | 3.99 s | 7.31 s | 15.69 s | 26.41 s | 22.79 s |
| [`00b2dbe`](https://github.com/helius-labs/zolana-mobile-sdk/commit/00b2dbe2265c90ce984c6634e5227b65027730c2) | 2026-10-19 | 1.19 s | 2.23 s | 3.87 s | 7.47 s | 15.73 s | 26.60 s | 23.12 s |
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 1.15 s | 2.18 s | 3.80 s | 7.49 s | 15.63 s | 25.46 s | 23.25 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 1.27 s | 2.12 s | 3.93 s | 7.28 s | 15.57 s | 25.73 s | 22.95 s |

## Samsung Galaxy A56, Android 15, mid tier

**Before:** [`00b2dbe`](https://github.com/helius-labs/zolana-mobile-sdk/commit/00b2dbe2265c90ce984c6634e5227b65027730c2) of 2026-10-19. **After:** [`9bde901`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bde901088fa31b64651906cdbefcf755372c6f8) of 2026-10-19.

| by scenario | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| first run, including download | before | 2.28 s (download: 1.40 s, 8.8 MiB) | 3.60 s (download: 2.17 s, 15.6 MiB) | 7.03 s (download: 4.40 s, 28.9 MiB) | 12.80 s (download: 7.88 s, 55.9 MiB) | 25.03 s (download: 15.59 s, 109.5 MiB) | 41.91 s (download: 26.73 s, 171.6 MiB) | 38.96 s (download: 24.26 s, 165.0 MiB) |
| | after | 5.99 s (download: 5.12 s, 8.8 MiB) (+163 % **!**) | 10.51 s (download: 9.08 s, 15.6 MiB) (+192 % **!**) | 19.95 s (download: 17.32 s, 28.9 MiB) (+184 % **!**) | 28.34 s (download: 23.41 s, 55.9 MiB) (+121 % **!**) | 41.11 s (download: 31.61 s, 109.5 MiB) (+64 % **!**) | 69.91 s (download: 54.62 s, 171.6 MiB) (+67 % **!**) | 152.41 s (download: 137.91 s, 165.0 MiB) (+291 % **!**) |
| warm | before | 0.34 s | 0.57 s | 1.06 s | 2.02 s | 3.83 s | 6.12 s | 5.70 s |
| | after | 0.34 s | 0.58 s | 1.08 s | 2.02 s | 3.96 s (+3 %) | 5.98 s (-2 %) | 5.78 s (+1 %) |
| first run after restart | before | 0.56 s | 0.65 s | 1.15 s | 2.16 s | 4.08 s | 6.37 s | 5.93 s |
| | after | 0.57 s | 0.64 s | 1.17 s | 2.13 s | 4.15 s (+2 %) | 6.36 s | 6.12 s (+3 %) |
| repeated after restart | before | 0.35 s | 0.60 s | 1.08 s | 2.01 s | 3.72 s | 6.14 s | 5.86 s |
| | after | 0.35 s | 0.59 s | 1.06 s | 2.01 s | 3.97 s (+7 %) | 6.11 s | 5.85 s |

| by stage | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| key load | before | 0.09 s | 0.03 s | 0.06 s | 0.09 s | 0.15 s | 0.23 s | 0.20 s |
| | after | 0.07 s | 0.03 s | 0.05 s | 0.09 s | 0.17 s | 0.20 s | 0.19 s |
| witness | before | 0.00 s | 0.01 s | 0.01 s | 0.02 s | 0.04 s | 0.06 s | 0.05 s |
| | after | 0.00 s | 0.01 s | 0.01 s | 0.02 s | 0.04 s | 0.06 s | 0.05 s |
| prove | before | 0.34 s | 0.58 s | 1.05 s | 2.00 s | 3.73 s | 6.05 s | 5.70 s |
| | after | 0.34 s | 0.58 s | 1.07 s | 2.00 s | 3.92 s (+5 %) | 6.03 s | 5.76 s (+1 %) |

### Proof, last 4 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`9bde901`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bde901088fa31b64651906cdbefcf755372c6f8) | 2026-10-19 | 0.34 s | 0.58 s | 1.07 s | 2.00 s | 3.92 s | 6.03 s | 5.76 s |
| [`00b2dbe`](https://github.com/helius-labs/zolana-mobile-sdk/commit/00b2dbe2265c90ce984c6634e5227b65027730c2) | 2026-10-19 | 0.34 s | 0.58 s | 1.05 s | 2.00 s | 3.73 s | 6.05 s | 5.70 s |
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 0.34 s | 0.58 s | 1.06 s | 2.00 s | 3.90 s | 6.08 s | 5.80 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.35 s | 0.57 s | 1.06 s | 1.98 s | 3.87 s | 6.00 s | 5.78 s |

### First run after a restart, last 4 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`9bde901`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bde901088fa31b64651906cdbefcf755372c6f8) | 2026-10-19 | 0.57 s | 0.64 s | 1.17 s | 2.13 s | 4.15 s | 6.36 s | 6.12 s |
| [`00b2dbe`](https://github.com/helius-labs/zolana-mobile-sdk/commit/00b2dbe2265c90ce984c6634e5227b65027730c2) | 2026-10-19 | 0.56 s | 0.65 s | 1.15 s | 2.16 s | 4.08 s | 6.37 s | 5.93 s |
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 0.56 s | 0.65 s | 1.17 s | 2.16 s | 4.24 s | 6.41 s | 6.11 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.55 s | 0.64 s | 1.15 s | 2.15 s | 4.13 s | 6.32 s | 5.93 s |

## Samsung Galaxy S25, Android 15, high tier

**Before:** [`00b2dbe`](https://github.com/helius-labs/zolana-mobile-sdk/commit/00b2dbe2265c90ce984c6634e5227b65027730c2) of 2026-10-19. **After:** [`9bde901`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bde901088fa31b64651906cdbefcf755372c6f8) of 2026-10-19.

| by scenario | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| first run, including download | before | 1.79 s (download: 1.27 s, 8.8 MiB) | 2.36 s (download: 1.55 s, 15.6 MiB) | 4.27 s (download: 2.96 s, 28.9 MiB) | 7.66 s (download: 5.16 s, 55.9 MiB) | 15.52 s (download: 10.74 s, 109.5 MiB) | 24.14 s (download: 16.27 s, 171.6 MiB) | 22.98 s (download: 15.48 s, 165.0 MiB) |
| | after | 1.86 s (download: 1.33 s, 8.8 MiB) (+4 %) | 2.53 s (download: 1.69 s, 15.6 MiB) (+7 %) | 4.72 s (download: 3.44 s, 28.9 MiB) (+10 %) | 8.97 s (download: 6.54 s, 55.9 MiB) (+17 %) | 19.44 s (download: 14.55 s, 109.5 MiB) (+25 % **!**) | 30.65 s (download: 22.88 s, 171.6 MiB) (+27 % **!**) | 28.11 s (download: 20.51 s, 165.0 MiB) (+22 % **!**) |
| warm | before | 0.19 s | 0.34 s | 0.54 s | 1.12 s | 2.13 s | 3.39 s | 3.25 s |
| | after | 0.19 s | 0.30 s | 0.58 s | 1.08 s | 2.14 s | 3.52 s (+4 %) | 3.27 s |
| first run after restart | before | 0.28 s | 0.34 s | 0.63 s | 1.20 s | 2.32 s | 3.77 s | 3.58 s |
| | after | 0.27 s | 0.34 s | 0.61 s | 1.17 s | 2.57 s (+11 %) | 6.88 s (+83 % **!**) | 4.66 s (+30 % **!**) |
| repeated after restart | before | 0.20 s | 0.34 s | 0.60 s | 1.18 s | 2.29 s | 3.59 s | 3.54 s |
| | after | 0.20 s | 0.33 s | 0.60 s | 1.15 s | 4.27 s (+87 % **!**) | 6.50 s (+81 % **!**) | 3.79 s (+7 %) |

| by stage | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| key load | before | 0.03 s | 0.01 s | 0.03 s | 0.04 s | 0.08 s | 0.11 s | 0.14 s |
| | after | 0.03 s | 0.01 s | 0.03 s | 0.04 s | 0.08 s | 0.28 s (+152 % **!**) | 0.30 s (+106 % **!**) |
| witness | before | 0.00 s | 0.01 s | 0.02 s | 0.03 s | 0.05 s | 0.07 s | 0.06 s |
| | after | 0.00 s | 0.01 s | 0.01 s | 0.03 s | 0.05 s | 0.10 s | 0.08 s |
| prove | before | 0.19 s | 0.33 s | 0.56 s | 1.10 s | 2.13 s | 3.42 s | 3.28 s |
| | after | 0.19 s | 0.32 s | 0.57 s | 1.07 s | 2.27 s (+6 %) | 4.91 s (+43 % **!**) | 3.45 s (+5 %) |

### Proof, last 4 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`9bde901`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bde901088fa31b64651906cdbefcf755372c6f8) | 2026-10-19 | 0.19 s | 0.32 s | 0.57 s | 1.07 s | 2.27 s | 4.91 s | 3.45 s |
| [`00b2dbe`](https://github.com/helius-labs/zolana-mobile-sdk/commit/00b2dbe2265c90ce984c6634e5227b65027730c2) | 2026-10-19 | 0.19 s | 0.33 s | 0.56 s | 1.10 s | 2.13 s | 3.42 s | 3.28 s |
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 0.20 s | 0.32 s | 0.58 s | 1.12 s | 2.22 s | 3.47 s | 4.65 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.20 s | 0.33 s | 0.59 s | 1.31 s | 2.36 s | 3.89 s | 4.68 s |

### First run after a restart, last 4 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`9bde901`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bde901088fa31b64651906cdbefcf755372c6f8) | 2026-10-19 | 0.27 s | 0.34 s | 0.61 s | 1.17 s | 2.57 s | 6.88 s | 4.66 s |
| [`00b2dbe`](https://github.com/helius-labs/zolana-mobile-sdk/commit/00b2dbe2265c90ce984c6634e5227b65027730c2) | 2026-10-19 | 0.28 s | 0.34 s | 0.63 s | 1.20 s | 2.32 s | 3.77 s | 3.58 s |
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 0.29 s | 0.35 s | 0.63 s | 1.26 s | 2.50 s | 3.80 s | 6.47 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.29 s | 0.37 s | 0.67 s | 1.38 s | 2.73 s | 4.61 s | 6.40 s |

## iPhone 12, iOS 18.5, low tier

**Before:** [`00b2dbe`](https://github.com/helius-labs/zolana-mobile-sdk/commit/00b2dbe2265c90ce984c6634e5227b65027730c2) of 2026-10-19. **After:** [`9bde901`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bde901088fa31b64651906cdbefcf755372c6f8) of 2026-10-19.

| by scenario | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| first run, including download | before | 1.59 s (download: 0.55 s, 8.8 MiB) | 2.24 s (download: 0.72 s, 15.6 MiB) | 3.99 s (download: 1.27 s, 28.9 MiB) | 7.33 s (download: 2.46 s, 55.9 MiB) | 14.45 s (download: 4.74 s, 109.5 MiB) | 23.59 s (download: 7.49 s, 171.6 MiB) | 23.94 s (download: 7.07 s, 165.0 MiB) |
| | after | 1.50 s (download: 0.53 s, 8.8 MiB) (-6 %) | 2.15 s (download: 0.71 s, 15.6 MiB) (-4 %) | 3.88 s (download: 1.27 s, 28.9 MiB) (-3 %) | 7.51 s (download: 2.50 s, 55.9 MiB) (+2 %) | 14.65 s (download: 4.88 s, 109.5 MiB) (+1 %) | 23.80 s (download: 7.48 s, 171.6 MiB) (+1 %) | 22.90 s (download: 7.08 s, 165.0 MiB) (-4 %) |
| warm | before | 0.33 s | 0.57 s | 0.95 s | 1.87 s | 3.55 s | 6.17 s | 6.40 s |
| | after | 0.31 s | 0.54 s | 0.97 s | 1.85 s | 3.56 s | 5.99 s (-3 %) | 5.99 s (-6 %) |
| first run after restart | before | 0.41 s | 0.72 s | 1.27 s | 2.35 s | 4.33 s | 6.91 s | 6.70 s |
| | after | 0.43 s | 0.69 s | 1.27 s | 2.28 s (-3 %) | 4.27 s (-1 %) | 6.95 s | 6.77 s (+1 %) |
| repeated after restart | before | 0.40 s | 0.68 s | 1.19 s | 2.19 s | 4.10 s | 6.54 s | 6.36 s |
| | after | 0.38 s | 0.65 s | 1.16 s | 2.15 s | 4.03 s (-2 %) | 6.61 s (+1 %) | 6.48 s (+2 %) |

| by stage | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| key load | before | 0.02 s | 0.03 s | 0.06 s | 0.11 s | 0.19 s | 0.23 s | 0.24 s |
| | after | 0.01 s | 0.03 s | 0.05 s | 0.10 s | 0.18 s | 0.27 s | 0.26 s |
| witness | before | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.01 s | 0.01 s | 0.01 s |
| | after | 0.00 s | 0.00 s | 0.00 s | 0.01 s | 0.01 s | 0.01 s | 0.01 s |
| prove | before | 0.37 s | 0.62 s | 1.08 s | 2.03 s | 3.81 s | 6.34 s | 6.40 s |
| | after | 0.36 s | 0.59 s | 1.07 s | 1.99 s | 3.78 s | 6.28 s (-1 %) | 6.22 s (-3 %) |

### Proof, last 4 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`9bde901`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bde901088fa31b64651906cdbefcf755372c6f8) | 2026-10-19 | 0.36 s | 0.59 s | 1.07 s | 1.99 s | 3.78 s | 6.28 s | 6.22 s |
| [`00b2dbe`](https://github.com/helius-labs/zolana-mobile-sdk/commit/00b2dbe2265c90ce984c6634e5227b65027730c2) | 2026-10-19 | 0.37 s | 0.62 s | 1.08 s | 2.03 s | 3.81 s | 6.34 s | 6.40 s |
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 0.37 s | 0.59 s | 1.03 s | 1.93 s | 3.69 s | 6.12 s | 6.17 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.37 s | 0.60 s | 1.08 s | 2.04 s | 3.96 s | 6.49 s | 6.49 s |

### First run after a restart, last 4 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`9bde901`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bde901088fa31b64651906cdbefcf755372c6f8) | 2026-10-19 | 0.43 s | 0.69 s | 1.27 s | 2.28 s | 4.27 s | 6.95 s | 6.77 s |
| [`00b2dbe`](https://github.com/helius-labs/zolana-mobile-sdk/commit/00b2dbe2265c90ce984c6634e5227b65027730c2) | 2026-10-19 | 0.41 s | 0.72 s | 1.27 s | 2.35 s | 4.33 s | 6.91 s | 6.70 s |
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 0.40 s | 0.72 s | 1.25 s | 2.27 s | 4.21 s | 6.90 s | 6.67 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.41 s | 0.71 s | 1.28 s | 2.33 s | 4.41 s | 7.47 s | 6.96 s |

## iPhone 15, iOS 18.5, mid tier

**Before:** [`00b2dbe`](https://github.com/helius-labs/zolana-mobile-sdk/commit/00b2dbe2265c90ce984c6634e5227b65027730c2) of 2026-10-19. **After:** [`9bde901`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bde901088fa31b64651906cdbefcf755372c6f8) of 2026-10-19.

| by scenario | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| first run, including download | before | 1.27 s (download: 0.48 s, 8.8 MiB) | 1.92 s (download: 0.68 s, 15.6 MiB) | 3.47 s (download: 1.25 s, 28.9 MiB) | 8.08 s (download: 4.00 s, 55.9 MiB) | 12.73 s (download: 4.73 s, 109.5 MiB) | 20.27 s (download: 7.38 s, 171.6 MiB) | 19.45 s (download: 6.92 s, 165.0 MiB) |
| | after | 1.28 s (download: 0.48 s, 8.8 MiB) | 1.87 s (download: 0.66 s, 15.6 MiB) (-3 %) | 3.38 s (download: 1.22 s, 28.9 MiB) (-3 %) | 6.25 s (download: 2.33 s, 55.9 MiB) (-23 % **!**) | 12.24 s (download: 4.58 s, 109.5 MiB) (-4 %) | 19.64 s (download: 7.13 s, 171.6 MiB) (-3 %) | 19.12 s (download: 6.86 s, 165.0 MiB) (-2 %) |
| warm | before | 0.28 s | 0.48 s | 0.88 s | 1.55 s | 2.99 s | 4.83 s | 4.72 s |
| | after | 0.28 s | 0.47 s | 0.79 s (-10 %) | 1.51 s | 2.84 s (-5 %) | 4.64 s (-4 %) | 4.61 s (-2 %) |
| first run after restart | before | 0.32 s | 0.53 s | 0.95 s | 1.74 s | 3.22 s | 5.25 s | 5.11 s |
| | after | 0.31 s | 0.52 s | 0.92 s | 1.68 s (-4 %) | 3.15 s (-2 %) | 5.07 s (-3 %) | 4.97 s (-3 %) |
| repeated after restart | before | 0.30 s | 0.51 s | 0.90 s | 1.65 s | 3.06 s | 4.97 s | 4.93 s |
| | after | 0.29 s | 0.49 s | 0.86 s | 1.61 s | 2.96 s (-3 %) | 4.83 s (-3 %) | 4.98 s |

| by stage | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| key load | before | 0.01 s | 0.02 s | 0.03 s | 0.06 s | 0.12 s | 0.17 s | 0.17 s |
| | after | 0.01 s | 0.02 s | 0.03 s | 0.05 s | 0.10 s | 0.15 s | 0.15 s |
| witness | before | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.01 s | 0.01 s | 0.01 s |
| | after | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.01 s | 0.01 s | 0.01 s |
| prove | before | 0.30 s | 0.49 s | 0.88 s | 1.60 s | 3.02 s | 4.89 s | 4.81 s |
| | after | 0.29 s | 0.48 s | 0.86 s | 1.59 s | 2.89 s (-4 %) | 4.72 s (-3 %) | 4.70 s (-2 %) |

### Proof, last 4 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`9bde901`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bde901088fa31b64651906cdbefcf755372c6f8) | 2026-10-19 | 0.29 s | 0.48 s | 0.86 s | 1.59 s | 2.89 s | 4.72 s | 4.70 s |
| [`00b2dbe`](https://github.com/helius-labs/zolana-mobile-sdk/commit/00b2dbe2265c90ce984c6634e5227b65027730c2) | 2026-10-19 | 0.30 s | 0.49 s | 0.88 s | 1.60 s | 3.02 s | 4.89 s | 4.81 s |
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 0.36 s | 0.59 s | 1.04 s | 1.96 s | 3.70 s | 5.70 s | 5.59 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.29 s | 0.46 s | 0.81 s | 1.50 s | 2.80 s | 4.68 s | 4.65 s |

### First run after a restart, last 4 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`9bde901`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bde901088fa31b64651906cdbefcf755372c6f8) | 2026-10-19 | 0.31 s | 0.52 s | 0.92 s | 1.68 s | 3.15 s | 5.07 s | 4.97 s |
| [`00b2dbe`](https://github.com/helius-labs/zolana-mobile-sdk/commit/00b2dbe2265c90ce984c6634e5227b65027730c2) | 2026-10-19 | 0.32 s | 0.53 s | 0.95 s | 1.74 s | 3.22 s | 5.25 s | 5.11 s |
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 0.39 s | 0.63 s | 1.09 s | 1.98 s | 4.20 s | 6.37 s | 5.80 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.31 s | 0.51 s | 0.92 s | 1.67 s | 3.08 s | 5.07 s | 4.96 s |

## iPhone 17 Pro, iOS 26.3.1, high tier

**Before:** [`00b2dbe`](https://github.com/helius-labs/zolana-mobile-sdk/commit/00b2dbe2265c90ce984c6634e5227b65027730c2) of 2026-10-19. **After:** [`9bde901`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bde901088fa31b64651906cdbefcf755372c6f8) of 2026-10-19.

| by scenario | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| first run, including download | before | 1.83 s (download: 1.31 s, 8.8 MiB) | 2.47 s (download: 1.61 s, 15.6 MiB) | 6.42 s (download: 4.86 s, 28.9 MiB) | 9.37 s (download: 6.43 s, 55.9 MiB) | 16.23 s (download: 10.57 s, 109.5 MiB) | 93.77 s (download: 84.70 s, 171.6 MiB) | 126.09 s (download: 117.31 s, 165.0 MiB) |
| | after | 0.80 s (download: 0.29 s, 8.8 MiB) (-56 % **!**) | 1.25 s (download: 0.39 s, 15.6 MiB) (-49 % **!**) | 2.12 s (download: 0.54 s, 28.9 MiB) (-67 % **!**) | 4.11 s (download: 1.02 s, 55.9 MiB) (-56 % **!**) | 7.99 s (download: 1.97 s, 109.5 MiB) (-51 % **!**) | 13.16 s (download: 3.44 s, 171.6 MiB) (-86 % **!**) | 12.58 s (download: 3.10 s, 165.0 MiB) (-90 % **!**) |
| warm | before | 0.20 s | 0.33 s | 0.61 s | 1.13 s | 2.10 s | 3.38 s | 3.29 s |
| | after | 0.20 s | 0.34 s | 0.61 s | 1.17 s | 2.18 s (+4 %) | 3.52 s (+4 %) | 3.49 s (+6 %) |
| first run after restart | before | 0.21 s | 0.36 s | 0.64 s | 1.19 s | 2.24 s | 3.66 s | 3.64 s |
| | after | 0.23 s | 0.38 s | 0.68 s | 1.24 s | 2.30 s (+3 %) | 3.72 s (+2 %) | 3.62 s |
| repeated after restart | before | 0.22 s | 0.36 s | 0.63 s | 1.18 s | 2.18 s | 3.61 s | 3.53 s |
| | after | 0.22 s | 0.37 s | 0.65 s | 1.20 s | 2.25 s (+3 %) | 3.64 s | 3.61 s (+2 %) |

| by stage | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| key load | before | 0.00 s | 0.01 s | 0.02 s | 0.03 s | 0.06 s | 0.09 s | 0.09 s |
| | after | 0.01 s | 0.01 s | 0.02 s | 0.03 s | 0.06 s | 0.09 s | 0.09 s |
| witness | before | 0.00 s | 0.00 s | 0.00 s | 0.01 s | 0.01 s | 0.01 s | 0.01 s |
| | after | 0.00 s | 0.00 s | 0.00 s | 0.01 s | 0.01 s | 0.01 s | 0.01 s |
| prove | before | 0.20 s | 0.34 s | 0.62 s | 1.14 s | 2.13 s | 3.46 s | 3.40 s |
| | after | 0.21 s | 0.35 s | 0.63 s | 1.18 s | 2.20 s (+3 %) | 3.56 s (+3 %) | 3.50 s (+3 %) |

### Proof, last 4 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`9bde901`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bde901088fa31b64651906cdbefcf755372c6f8) | 2026-10-19 | 0.21 s | 0.35 s | 0.63 s | 1.18 s | 2.20 s | 3.56 s | 3.50 s |
| [`00b2dbe`](https://github.com/helius-labs/zolana-mobile-sdk/commit/00b2dbe2265c90ce984c6634e5227b65027730c2) | 2026-10-19 | 0.20 s | 0.34 s | 0.62 s | 1.14 s | 2.13 s | 3.46 s | 3.40 s |
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 0.21 s | 0.34 s | 0.62 s | 1.16 s | 2.12 s | 3.54 s | 3.47 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.20 s | 0.35 s | 0.63 s | 1.17 s | 2.17 s | 3.57 s | 3.46 s |

### First run after a restart, last 4 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`9bde901`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bde901088fa31b64651906cdbefcf755372c6f8) | 2026-10-19 | 0.23 s | 0.38 s | 0.68 s | 1.24 s | 2.30 s | 3.72 s | 3.62 s |
| [`00b2dbe`](https://github.com/helius-labs/zolana-mobile-sdk/commit/00b2dbe2265c90ce984c6634e5227b65027730c2) | 2026-10-19 | 0.21 s | 0.36 s | 0.64 s | 1.19 s | 2.24 s | 3.66 s | 3.64 s |
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 0.23 s | 0.38 s | 0.68 s | 1.23 s | 2.27 s | 3.70 s | 3.63 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.22 s | 0.38 s | 0.67 s | 1.23 s | 2.26 s | 3.68 s | 3.60 s |
