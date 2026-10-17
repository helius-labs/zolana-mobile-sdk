# Proof timing on main

Written by CI after every push to `main` (`scripts/proof-timing-history.py` on `main`), from release builds on the phones of `scripts/devicefarm/devices.json` in AWS Device Farm. Two runs of one commit on a phone differ by up to about 4 %. The data is in `history/`, a line per commit.

## Samsung Galaxy A16, Android 14, low tier

**After:** [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) of 2026-10-17. No report of `main` to compare with yet.

| by scenario | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|
| first run, including download | 6.62 s (download: 3.78 s, 8.8 MiB) | 10.39 s (download: 5.76 s, 15.6 MiB) | 21.44 s (download: 12.87 s, 28.9 MiB) | 51.62 s (download: 35.61 s, 55.9 MiB) | 81.66 s (download: 51.25 s, 109.5 MiB) | 128.05 s (download: 76.49 s, 171.6 MiB) | 120.90 s (download: 74.87 s, 165.0 MiB) |
| warm | 1.23 s | 2.08 s | 3.72 s | 7.11 s | 15.49 s | 23.03 s | 22.53 s |
| first run after restart | 1.27 s | 2.12 s | 3.93 s | 7.28 s | 15.57 s | 25.73 s | 22.95 s |
| repeated after restart | 1.25 s | 2.06 s | 3.75 s | 7.30 s | 15.13 s | 25.30 s | 21.58 s |

| by stage | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|
| key load | 0.08 s | 0.09 s | 0.15 s | 0.22 s | 0.34 s | 0.54 s | 0.53 s |
| witness | 0.02 s | 0.03 s | 0.06 s | 0.12 s | 0.23 s | 0.37 s | 0.18 s |
| prove | 1.19 s | 2.01 s | 3.69 s | 6.99 s | 14.92 s | 23.81 s | 21.72 s |

### Proof, last 1 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 1.19 s | 2.01 s | 3.69 s | 6.99 s | 14.92 s | 23.81 s | 21.72 s |

### First run after a restart, last 1 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 1.27 s | 2.12 s | 3.93 s | 7.28 s | 15.57 s | 25.73 s | 22.95 s |

## Samsung Galaxy A56, Android 15, mid tier

**After:** [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) of 2026-10-17. No report of `main` to compare with yet.

| by scenario | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|
| first run, including download | 3.24 s (download: 2.36 s, 8.8 MiB) | 3.73 s (download: 2.32 s, 15.6 MiB) | 7.15 s (download: 4.53 s, 28.9 MiB) | 13.46 s (download: 8.49 s, 55.9 MiB) | 26.18 s (download: 16.42 s, 109.5 MiB) | 41.27 s (download: 25.98 s, 171.6 MiB) | 39.72 s (download: 24.94 s, 165.0 MiB) |
| warm | 0.35 s | 0.58 s | 1.06 s | 1.99 s | 3.88 s | 6.07 s | 5.81 s |
| first run after restart | 0.55 s | 0.64 s | 1.15 s | 2.15 s | 4.13 s | 6.32 s | 5.93 s |
| repeated after restart | 0.35 s | 0.59 s | 1.07 s | 2.02 s | 3.83 s | 6.08 s | 5.92 s |

| by stage | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|
| key load | 0.10 s | 0.03 s | 0.04 s | 0.09 s | 0.15 s | 0.28 s | 0.20 s |
| witness | 0.00 s | 0.01 s | 0.01 s | 0.02 s | 0.04 s | 0.06 s | 0.04 s |
| prove | 0.35 s | 0.57 s | 1.06 s | 1.98 s | 3.87 s | 6.00 s | 5.78 s |

### Proof, last 1 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.35 s | 0.57 s | 1.06 s | 1.98 s | 3.87 s | 6.00 s | 5.78 s |

### First run after a restart, last 1 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.55 s | 0.64 s | 1.15 s | 2.15 s | 4.13 s | 6.32 s | 5.93 s |

## Samsung Galaxy S25, Android 15, high tier

**After:** [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) of 2026-10-17. No report of `main` to compare with yet.

| by scenario | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|
| first run, including download | 2.55 s (download: 2.04 s, 8.8 MiB) | 2.38 s (download: 1.55 s, 15.6 MiB) | 4.45 s (download: 3.17 s, 28.9 MiB) | 9.04 s (download: 6.11 s, 55.9 MiB) | 15.47 s (download: 10.53 s, 109.5 MiB) | 28.30 s (download: 20.39 s, 171.6 MiB) | 25.20 s (download: 17.46 s, 165.0 MiB) |
| warm | 0.19 s | 0.32 s | 0.59 s | 1.11 s | 2.17 s | 3.50 s | 3.40 s |
| first run after restart | 0.29 s | 0.37 s | 0.67 s | 1.38 s | 2.73 s | 4.61 s | 6.40 s |
| repeated after restart | 0.20 s | 0.35 s | 0.64 s | 1.36 s | 2.66 s | 6.37 s | 6.98 s |

| by stage | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|
| key load | 0.04 s | 0.01 s | 0.02 s | 0.05 s | 0.10 s | 0.16 s | 0.29 s |
| witness | 0.00 s | 0.01 s | 0.01 s | 0.03 s | 0.05 s | 0.08 s | 0.07 s |
| prove | 0.20 s | 0.33 s | 0.59 s | 1.31 s | 2.36 s | 3.89 s | 4.68 s |

### Proof, last 1 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.20 s | 0.33 s | 0.59 s | 1.31 s | 2.36 s | 3.89 s | 4.68 s |

### First run after a restart, last 1 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.29 s | 0.37 s | 0.67 s | 1.38 s | 2.73 s | 4.61 s | 6.40 s |

## iPhone 12, iOS 18.5, low tier

**After:** [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) of 2026-10-17. No report of `main` to compare with yet.

| by scenario | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|
| first run, including download | 1.45 s (download: 0.50 s, 8.8 MiB) | 2.05 s (download: 0.69 s, 15.6 MiB) | 3.77 s (download: 1.25 s, 28.9 MiB) | 7.24 s (download: 2.41 s, 55.9 MiB) | 14.43 s (download: 4.71 s, 109.5 MiB) | 23.30 s (download: 7.36 s, 171.6 MiB) | 23.03 s (download: 7.04 s, 165.0 MiB) |
| warm | 0.32 s | 0.53 s | 0.96 s | 1.87 s | 3.60 s | 6.01 s | 6.32 s |
| first run after restart | 0.41 s | 0.71 s | 1.28 s | 2.33 s | 4.41 s | 7.47 s | 6.96 s |
| repeated after restart | 0.41 s | 0.66 s | 1.21 s | 2.26 s | 5.28 s | 6.91 s | 6.69 s |

| by stage | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|
| key load | 0.01 s | 0.03 s | 0.04 s | 0.08 s | 0.15 s | 0.22 s | 0.20 s |
| witness | 0.00 s | 0.00 s | 0.00 s | 0.01 s | 0.01 s | 0.01 s | 0.01 s |
| prove | 0.37 s | 0.60 s | 1.08 s | 2.04 s | 3.96 s | 6.49 s | 6.49 s |

### Proof, last 1 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.37 s | 0.60 s | 1.08 s | 2.04 s | 3.96 s | 6.49 s | 6.49 s |

### First run after a restart, last 1 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.41 s | 0.71 s | 1.28 s | 2.33 s | 4.41 s | 7.47 s | 6.96 s |

## iPhone 15, iOS 18.5, mid tier

**After:** [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) of 2026-10-17. No report of `main` to compare with yet.

| by scenario | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|
| first run, including download | 1.29 s (download: 0.48 s, 8.8 MiB) | 1.85 s (download: 0.69 s, 15.6 MiB) | 3.32 s (download: 1.22 s, 28.9 MiB) | 6.29 s (download: 2.33 s, 55.9 MiB) | 12.22 s (download: 4.79 s, 109.5 MiB) | 19.38 s (download: 7.17 s, 171.6 MiB) | 18.95 s (download: 6.93 s, 165.0 MiB) |
| warm | 0.28 s | 0.43 s | 0.76 s | 1.44 s | 2.72 s | 4.56 s | 4.53 s |
| first run after restart | 0.31 s | 0.51 s | 0.92 s | 1.67 s | 3.08 s | 5.07 s | 4.96 s |
| repeated after restart | 0.29 s | 0.49 s | 0.86 s | 1.58 s | 2.91 s | 4.82 s | 5.01 s |

| by stage | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|
| key load | 0.01 s | 0.02 s | 0.03 s | 0.06 s | 0.11 s | 0.17 s | 0.16 s |
| witness | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.01 s | 0.01 s | 0.01 s |
| prove | 0.29 s | 0.46 s | 0.81 s | 1.50 s | 2.80 s | 4.68 s | 4.65 s |

### Proof, last 1 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.29 s | 0.46 s | 0.81 s | 1.50 s | 2.80 s | 4.68 s | 4.65 s |

### First run after a restart, last 1 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.31 s | 0.51 s | 0.92 s | 1.67 s | 3.08 s | 5.07 s | 4.96 s |

## iPhone 17 Pro, iOS 26.3.1, high tier

**After:** [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) of 2026-10-17. No report of `main` to compare with yet.

| by scenario | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|
| first run, including download | 0.97 s (download: 0.45 s, 8.8 MiB) | 1.58 s (download: 0.68 s, 15.6 MiB) | 2.84 s (download: 1.21 s, 28.9 MiB) | 5.56 s (download: 2.58 s, 55.9 MiB) | 10.49 s (download: 4.63 s, 109.5 MiB) | 16.57 s (download: 7.08 s, 171.6 MiB) | 16.00 s (download: 6.72 s, 165.0 MiB) |
| warm | 0.20 s | 0.33 s | 0.61 s | 1.16 s | 2.16 s | 3.58 s | 3.45 s |
| first run after restart | 0.22 s | 0.38 s | 0.67 s | 1.23 s | 2.26 s | 3.68 s | 3.60 s |
| repeated after restart | 0.22 s | 0.37 s | 0.65 s | 1.21 s | 2.20 s | 3.64 s | 3.52 s |

| by stage | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|
| key load | 0.01 s | 0.01 s | 0.02 s | 0.03 s | 0.05 s | 0.09 s | 0.09 s |
| witness | 0.00 s | 0.00 s | 0.00 s | 0.01 s | 0.01 s | 0.01 s | 0.01 s |
| prove | 0.20 s | 0.35 s | 0.63 s | 1.17 s | 2.17 s | 3.57 s | 3.46 s |

### Proof, last 1 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.20 s | 0.35 s | 0.63 s | 1.17 s | 2.17 s | 3.57 s | 3.46 s |

### First run after a restart, last 1 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.22 s | 0.38 s | 0.67 s | 1.23 s | 2.26 s | 3.68 s | 3.60 s |
