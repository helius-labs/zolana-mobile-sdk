# Proof timing on main

Written by CI after every push to `main` (`scripts/proof-timing-history.py` on `main`), from release builds on the phones of `scripts/devicefarm/devices.json` in AWS Device Farm. Two runs of one commit differ by up to 5 % on the Galaxy A16 and A56 and up to 15 % on the S25 and the iPhones. The data is in `history/`, a line per commit.

## Samsung Galaxy A16, Android 14, low tier

**Before:** [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) of 2026-10-17. **After:** [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) of 2026-10-19.

| by scenario | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| first run, including download | before | 6.62 s (download: 3.78 s, 8.8 MiB) | 10.39 s (download: 5.76 s, 15.6 MiB) | 21.44 s (download: 12.87 s, 28.9 MiB) | 51.62 s (download: 35.61 s, 55.9 MiB) | 81.66 s (download: 51.25 s, 109.5 MiB) | 128.05 s (download: 76.49 s, 171.6 MiB) | 120.90 s (download: 74.87 s, 165.0 MiB) |
| | after | 5.70 s (download: 2.93 s, 8.8 MiB) (-14 %) | 11.62 s (download: 7.03 s, 15.6 MiB) (+12 %) | 20.85 s (download: 12.52 s, 28.9 MiB) (-3 %) | 35.07 s (download: 19.04 s, 55.9 MiB) (-32 % **!**) | 79.30 s (download: 47.51 s, 109.5 MiB) (-3 %) | 124.29 s (download: 73.63 s, 171.6 MiB) (-3 %) | 116.09 s (download: 68.53 s, 165.0 MiB) (-4 %) |
| warm | before | 1.23 s | 2.08 s | 3.72 s | 7.11 s | 15.49 s | 23.03 s | 22.53 s |
| | after | 1.23 s | 2.12 s | 3.72 s | 7.19 s (+1 %) | 15.23 s (-2 %) | 26.02 s (+13 %) | 22.91 s (+2 %) |
| first run after restart | before | 1.27 s | 2.12 s | 3.93 s | 7.28 s | 15.57 s | 25.73 s | 22.95 s |
| | after | 1.15 s (-10 %) | 2.18 s (+3 %) | 3.80 s (-3 %) | 7.49 s (+3 %) | 15.63 s | 25.46 s (-1 %) | 23.25 s (+1 %) |
| repeated after restart | before | 1.25 s | 2.06 s | 3.75 s | 7.30 s | 15.13 s | 25.30 s | 21.58 s |
| | after | 1.23 s | 2.07 s | 3.76 s | 7.17 s (-2 %) | 15.24 s (+1 %) | 25.41 s | 21.93 s (+2 %) |

| by stage | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| key load | before | 0.08 s | 0.09 s | 0.15 s | 0.22 s | 0.34 s | 0.54 s | 0.53 s |
| | after | 0.05 s | 0.11 s | 0.14 s | 0.20 s | 0.35 s | 0.58 s | 0.65 s (+22 % **!**) |
| witness | before | 0.02 s | 0.03 s | 0.06 s | 0.12 s | 0.23 s | 0.37 s | 0.18 s |
| | after | 0.03 s | 0.03 s | 0.05 s | 0.13 s | 0.24 s | 0.23 s (-36 % **!**) | 0.15 s |
| prove | before | 1.19 s | 2.01 s | 3.69 s | 6.99 s | 14.92 s | 23.81 s | 21.72 s |
| | after | 1.18 s | 2.02 s | 3.66 s | 7.07 s (+1 %) | 14.96 s | 24.88 s (+5 %) | 22.09 s (+2 %) |

### Proof, last 2 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 1.18 s | 2.02 s | 3.66 s | 7.07 s | 14.96 s | 24.88 s | 22.09 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 1.19 s | 2.01 s | 3.69 s | 6.99 s | 14.92 s | 23.81 s | 21.72 s |

### First run after a restart, last 2 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 1.15 s | 2.18 s | 3.80 s | 7.49 s | 15.63 s | 25.46 s | 23.25 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 1.27 s | 2.12 s | 3.93 s | 7.28 s | 15.57 s | 25.73 s | 22.95 s |

## Samsung Galaxy A56, Android 15, mid tier

**Before:** [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) of 2026-10-17. **After:** [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) of 2026-10-19.

| by scenario | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| first run, including download | before | 3.24 s (download: 2.36 s, 8.8 MiB) | 3.73 s (download: 2.32 s, 15.6 MiB) | 7.15 s (download: 4.53 s, 28.9 MiB) | 13.46 s (download: 8.49 s, 55.9 MiB) | 26.18 s (download: 16.42 s, 109.5 MiB) | 41.27 s (download: 25.98 s, 171.6 MiB) | 39.72 s (download: 24.94 s, 165.0 MiB) |
| | after | 7.69 s (download: 6.80 s, 8.8 MiB) (+137 % **!**) | 5.23 s (download: 3.78 s, 15.6 MiB) (+40 % **!**) | 9.34 s (download: 6.71 s, 28.9 MiB) (+31 % **!**) | 17.51 s (download: 12.56 s, 55.9 MiB) (+30 % **!**) | 27.23 s (download: 17.46 s, 109.5 MiB) (+4 %) | 41.59 s (download: 25.96 s, 171.6 MiB) (+1 %) | 40.64 s (download: 25.75 s, 165.0 MiB) (+2 %) |
| warm | before | 0.35 s | 0.58 s | 1.06 s | 1.99 s | 3.88 s | 6.07 s | 5.81 s |
| | after | 0.34 s | 0.59 s | 1.05 s | 2.01 s | 3.90 s | 6.14 s (+1 %) | 5.79 s |
| first run after restart | before | 0.55 s | 0.64 s | 1.15 s | 2.15 s | 4.13 s | 6.32 s | 5.93 s |
| | after | 0.56 s | 0.65 s | 1.17 s | 2.16 s | 4.24 s (+3 %) | 6.41 s (+1 %) | 6.11 s (+3 %) |
| repeated after restart | before | 0.35 s | 0.59 s | 1.07 s | 2.02 s | 3.83 s | 6.08 s | 5.92 s |
| | after | 0.35 s | 0.60 s | 1.07 s | 2.04 s | 3.93 s (+2 %) | 6.14 s (+1 %) | 5.92 s |

| by stage | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| key load | before | 0.10 s | 0.03 s | 0.04 s | 0.09 s | 0.15 s | 0.28 s | 0.20 s |
| | after | 0.08 s | 0.03 s | 0.06 s | 0.09 s | 0.17 s | 0.21 s (-26 % **!**) | 0.18 s |
| witness | before | 0.00 s | 0.01 s | 0.01 s | 0.02 s | 0.04 s | 0.06 s | 0.04 s |
| | after | 0.00 s | 0.01 s | 0.01 s | 0.02 s | 0.05 s | 0.06 s | 0.05 s |
| prove | before | 0.35 s | 0.57 s | 1.06 s | 1.98 s | 3.87 s | 6.00 s | 5.78 s |
| | after | 0.34 s | 0.58 s | 1.06 s | 2.00 s | 3.90 s | 6.08 s (+1 %) | 5.80 s |

### Proof, last 2 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 0.34 s | 0.58 s | 1.06 s | 2.00 s | 3.90 s | 6.08 s | 5.80 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.35 s | 0.57 s | 1.06 s | 1.98 s | 3.87 s | 6.00 s | 5.78 s |

### First run after a restart, last 2 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 0.56 s | 0.65 s | 1.17 s | 2.16 s | 4.24 s | 6.41 s | 6.11 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.55 s | 0.64 s | 1.15 s | 2.15 s | 4.13 s | 6.32 s | 5.93 s |

## Samsung Galaxy S25, Android 15, high tier

**Before:** [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) of 2026-10-17. **After:** [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) of 2026-10-19.

| by scenario | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| first run, including download | before | 2.55 s (download: 2.04 s, 8.8 MiB) | 2.38 s (download: 1.55 s, 15.6 MiB) | 4.45 s (download: 3.17 s, 28.9 MiB) | 9.04 s (download: 6.11 s, 55.9 MiB) | 15.47 s (download: 10.53 s, 109.5 MiB) | 28.30 s (download: 20.39 s, 171.6 MiB) | 25.20 s (download: 17.46 s, 165.0 MiB) |
| | after | 1.70 s (download: 1.19 s, 8.8 MiB) (-33 % **!**) | 2.41 s (download: 1.63 s, 15.6 MiB) | 4.35 s (download: 2.99 s, 28.9 MiB) (-2 %) | 8.33 s (download: 5.80 s, 55.9 MiB) (-8 %) | 16.14 s (download: 11.26 s, 109.5 MiB) (+4 %) | 26.40 s (download: 18.61 s, 171.6 MiB) (-7 %) | 25.67 s (download: 18.09 s, 165.0 MiB) (+2 %) |
| warm | before | 0.19 s | 0.32 s | 0.59 s | 1.11 s | 2.17 s | 3.50 s | 3.40 s |
| | after | 0.20 s | 0.32 s | 0.57 s | 1.09 s | 2.14 s | 3.43 s (-2 %) | 3.31 s (-3 %) |
| first run after restart | before | 0.29 s | 0.37 s | 0.67 s | 1.38 s | 2.73 s | 4.61 s | 6.40 s |
| | after | 0.29 s | 0.35 s | 0.63 s | 1.26 s (-8 %) | 2.50 s (-9 %) | 3.80 s (-18 %) | 6.47 s (+1 %) |
| repeated after restart | before | 0.20 s | 0.35 s | 0.64 s | 1.36 s | 2.66 s | 6.37 s | 6.98 s |
| | after | 0.20 s | 0.35 s | 0.63 s | 1.22 s (-11 %) | 2.41 s (-10 %) | 4.83 s (-24 % **!**) | 6.16 s (-12 %) |

| by stage | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| key load | before | 0.04 s | 0.01 s | 0.02 s | 0.05 s | 0.10 s | 0.16 s | 0.29 s |
| | after | 0.04 s | 0.01 s | 0.02 s | 0.05 s | 0.09 s | 0.13 s | 0.28 s |
| witness | before | 0.00 s | 0.01 s | 0.01 s | 0.03 s | 0.05 s | 0.08 s | 0.07 s |
| | after | 0.00 s | 0.01 s | 0.01 s | 0.03 s | 0.05 s | 0.08 s | 0.08 s |
| prove | before | 0.20 s | 0.33 s | 0.59 s | 1.31 s | 2.36 s | 3.89 s | 4.68 s |
| | after | 0.20 s | 0.32 s | 0.58 s | 1.12 s (-14 %) | 2.22 s (-6 %) | 3.47 s (-11 %) | 4.65 s |

### Proof, last 2 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 0.20 s | 0.32 s | 0.58 s | 1.12 s | 2.22 s | 3.47 s | 4.65 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.20 s | 0.33 s | 0.59 s | 1.31 s | 2.36 s | 3.89 s | 4.68 s |

### First run after a restart, last 2 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 0.29 s | 0.35 s | 0.63 s | 1.26 s | 2.50 s | 3.80 s | 6.47 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.29 s | 0.37 s | 0.67 s | 1.38 s | 2.73 s | 4.61 s | 6.40 s |

## iPhone 12, iOS 18.5, low tier

**Before:** [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) of 2026-10-17. **After:** [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) of 2026-10-19.

| by scenario | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| first run, including download | before | 1.45 s (download: 0.50 s, 8.8 MiB) | 2.05 s (download: 0.69 s, 15.6 MiB) | 3.77 s (download: 1.25 s, 28.9 MiB) | 7.24 s (download: 2.41 s, 55.9 MiB) | 14.43 s (download: 4.71 s, 109.5 MiB) | 23.30 s (download: 7.36 s, 171.6 MiB) | 23.03 s (download: 7.04 s, 165.0 MiB) |
| | after | 1.59 s (download: 0.56 s, 8.8 MiB) (+10 %) | 2.26 s (download: 0.72 s, 15.6 MiB) (+10 %) | 5.37 s (download: 2.81 s, 28.9 MiB) (+43 % **!**) | 9.40 s (download: 4.54 s, 55.9 MiB) (+30 % **!**) | 20.31 s (download: 10.84 s, 109.5 MiB) (+41 % **!**) | 24.74 s (download: 9.00 s, 171.6 MiB) (+6 %) | 22.70 s (download: 7.18 s, 165.0 MiB) (-1 %) |
| warm | before | 0.32 s | 0.53 s | 0.96 s | 1.87 s | 3.60 s | 6.01 s | 6.32 s |
| | after | 0.33 s | 0.56 s | 0.94 s | 1.78 s (-5 %) | 3.37 s (-6 %) | 5.71 s (-5 %) | 6.03 s (-5 %) |
| first run after restart | before | 0.41 s | 0.71 s | 1.28 s | 2.33 s | 4.41 s | 7.47 s | 6.96 s |
| | after | 0.40 s | 0.72 s | 1.25 s | 2.27 s (-3 %) | 4.21 s (-5 %) | 6.90 s (-8 %) | 6.67 s (-4 %) |
| repeated after restart | before | 0.41 s | 0.66 s | 1.21 s | 2.26 s | 5.28 s | 6.91 s | 6.69 s |
| | after | 0.38 s | 0.65 s | 1.13 s (-6 %) | 2.12 s (-6 %) | 3.97 s (-25 % **!**) | 6.47 s (-6 %) | 6.33 s (-5 %) |

| by stage | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| key load | before | 0.01 s | 0.03 s | 0.04 s | 0.08 s | 0.15 s | 0.22 s | 0.20 s |
| | after | 0.02 s | 0.03 s | 0.05 s | 0.10 s | 0.18 s | 0.22 s | 0.25 s |
| witness | before | 0.00 s | 0.00 s | 0.00 s | 0.01 s | 0.01 s | 0.01 s | 0.01 s |
| | after | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.01 s | 0.01 s | 0.01 s |
| prove | before | 0.37 s | 0.60 s | 1.08 s | 2.04 s | 3.96 s | 6.49 s | 6.49 s |
| | after | 0.37 s | 0.59 s | 1.03 s | 1.93 s (-5 %) | 3.69 s (-7 %) | 6.12 s (-6 %) | 6.17 s (-5 %) |

### Proof, last 2 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 0.37 s | 0.59 s | 1.03 s | 1.93 s | 3.69 s | 6.12 s | 6.17 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.37 s | 0.60 s | 1.08 s | 2.04 s | 3.96 s | 6.49 s | 6.49 s |

### First run after a restart, last 2 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 0.40 s | 0.72 s | 1.25 s | 2.27 s | 4.21 s | 6.90 s | 6.67 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.41 s | 0.71 s | 1.28 s | 2.33 s | 4.41 s | 7.47 s | 6.96 s |

## iPhone 15, iOS 18.5, mid tier

**Before:** [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) of 2026-10-17. **After:** [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) of 2026-10-19.

| by scenario | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| first run, including download | before | 1.29 s (download: 0.48 s, 8.8 MiB) | 1.85 s (download: 0.69 s, 15.6 MiB) | 3.32 s (download: 1.22 s, 28.9 MiB) | 6.29 s (download: 2.33 s, 55.9 MiB) | 12.22 s (download: 4.79 s, 109.5 MiB) | 19.38 s (download: 7.17 s, 171.6 MiB) | 18.95 s (download: 6.93 s, 165.0 MiB) |
| | after | 1.44 s (download: 0.51 s, 8.8 MiB) (+12 %) | 2.20 s (download: 0.68 s, 15.6 MiB) (+19 %) | 3.93 s (download: 1.24 s, 28.9 MiB) (+18 %) | 10.39 s (download: 5.45 s, 55.9 MiB) (+65 % **!**) | 14.24 s (download: 4.65 s, 109.5 MiB) (+16 %) | 21.41 s (download: 7.19 s, 171.6 MiB) (+10 %) | 21.11 s (download: 6.95 s, 165.0 MiB) (+11 %) |
| warm | before | 0.28 s | 0.43 s | 0.76 s | 1.44 s | 2.72 s | 4.56 s | 4.53 s |
| | after | 0.36 s (+32 % **!**) | 0.55 s (+26 % **!**) | 1.03 s (+35 % **!**) | 1.96 s (+36 % **!**) | 3.36 s (+24 % **!**) | 5.51 s (+21 % **!**) | 5.78 s (+28 % **!**) |
| first run after restart | before | 0.31 s | 0.51 s | 0.92 s | 1.67 s | 3.08 s | 5.07 s | 4.96 s |
| | after | 0.39 s (+24 % **!**) | 0.63 s (+26 % **!**) | 1.09 s (+19 %) | 1.98 s (+19 %) | 4.20 s (+36 % **!**) | 6.37 s (+26 % **!**) | 5.80 s (+17 %) |
| repeated after restart | before | 0.29 s | 0.49 s | 0.86 s | 1.58 s | 2.91 s | 4.82 s | 5.01 s |
| | after | 0.35 s (+24 % **!**) | 0.59 s (+22 % **!**) | 1.06 s (+23 % **!**) | 2.17 s (+37 % **!**) | 4.11 s (+41 % **!**) | 5.93 s (+23 % **!**) | 5.64 s (+13 %) |

| by stage | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| key load | before | 0.01 s | 0.02 s | 0.03 s | 0.06 s | 0.11 s | 0.17 s | 0.16 s |
| | after | 0.01 s | 0.02 s | 0.04 s | 0.07 s | 0.17 s (+52 % **!**) | 0.24 s (+41 % **!**) | 0.22 s (+35 % **!**) |
| witness | before | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.01 s | 0.01 s | 0.01 s |
| | after | 0.00 s | 0.00 s | 0.00 s | 0.01 s | 0.01 s | 0.01 s | 0.01 s |
| prove | before | 0.29 s | 0.46 s | 0.81 s | 1.50 s | 2.80 s | 4.68 s | 4.65 s |
| | after | 0.36 s (+26 % **!**) | 0.59 s (+29 % **!**) | 1.04 s (+28 % **!**) | 1.96 s (+31 % **!**) | 3.70 s (+32 % **!**) | 5.70 s (+22 % **!**) | 5.59 s (+20 % **!**) |

### Proof, last 2 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 0.36 s | 0.59 s | 1.04 s | 1.96 s | 3.70 s | 5.70 s | 5.59 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.29 s | 0.46 s | 0.81 s | 1.50 s | 2.80 s | 4.68 s | 4.65 s |

### First run after a restart, last 2 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 0.39 s | 0.63 s | 1.09 s | 1.98 s | 4.20 s | 6.37 s | 5.80 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.31 s | 0.51 s | 0.92 s | 1.67 s | 3.08 s | 5.07 s | 4.96 s |

## iPhone 17 Pro, iOS 26.3.1, high tier

**Before:** [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) of 2026-10-17. **After:** [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) of 2026-10-19.

| by scenario | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| first run, including download | before | 0.97 s (download: 0.45 s, 8.8 MiB) | 1.58 s (download: 0.68 s, 15.6 MiB) | 2.84 s (download: 1.21 s, 28.9 MiB) | 5.56 s (download: 2.58 s, 55.9 MiB) | 10.49 s (download: 4.63 s, 109.5 MiB) | 16.57 s (download: 7.08 s, 171.6 MiB) | 16.00 s (download: 6.72 s, 165.0 MiB) |
| | after | 0.85 s (download: 0.31 s, 8.8 MiB) (-13 %) | 4.51 s (download: 3.62 s, 15.6 MiB) (+186 % **!**) | 7.27 s (download: 5.62 s, 28.9 MiB) (+156 % **!**) | 9.10 s (download: 6.16 s, 55.9 MiB) (+64 % **!**) | 24.38 s (download: 18.77 s, 109.5 MiB) (+132 % **!**) | 12.56 s (download: 3.11 s, 171.6 MiB) (-24 % **!**) | 14.32 s (download: 5.07 s, 165.0 MiB) (-11 %) |
| warm | before | 0.20 s | 0.33 s | 0.61 s | 1.16 s | 2.16 s | 3.58 s | 3.45 s |
| | after | 0.20 s | 0.33 s | 0.58 s | 1.12 s | 2.05 s (-5 %) | 3.50 s (-2 %) | 3.44 s |
| first run after restart | before | 0.22 s | 0.38 s | 0.67 s | 1.23 s | 2.26 s | 3.68 s | 3.60 s |
| | after | 0.23 s | 0.38 s | 0.68 s | 1.23 s | 2.27 s | 3.70 s | 3.63 s |
| repeated after restart | before | 0.22 s | 0.37 s | 0.65 s | 1.21 s | 2.20 s | 3.64 s | 3.52 s |
| | after | 0.22 s | 0.37 s | 0.65 s | 1.20 s | 2.23 s | 3.62 s | 3.53 s |

| by stage | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| key load | before | 0.01 s | 0.01 s | 0.02 s | 0.03 s | 0.05 s | 0.09 s | 0.09 s |
| | after | 0.01 s | 0.01 s | 0.02 s | 0.03 s | 0.06 s | 0.09 s | 0.09 s |
| witness | before | 0.00 s | 0.00 s | 0.00 s | 0.01 s | 0.01 s | 0.01 s | 0.01 s |
| | after | 0.00 s | 0.00 s | 0.00 s | 0.01 s | 0.01 s | 0.01 s | 0.01 s |
| prove | before | 0.20 s | 0.35 s | 0.63 s | 1.17 s | 2.17 s | 3.57 s | 3.46 s |
| | after | 0.21 s | 0.34 s | 0.62 s | 1.16 s | 2.12 s (-3 %) | 3.54 s | 3.47 s |

### Proof, last 2 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 0.21 s | 0.34 s | 0.62 s | 1.16 s | 2.12 s | 3.54 s | 3.47 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.20 s | 0.35 s | 0.63 s | 1.17 s | 2.17 s | 3.57 s | 3.46 s |

### First run after a restart, last 2 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 0.23 s | 0.38 s | 0.68 s | 1.23 s | 2.27 s | 3.70 s | 3.63 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.22 s | 0.38 s | 0.67 s | 1.23 s | 2.26 s | 3.68 s | 3.60 s |
