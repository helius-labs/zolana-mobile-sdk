# Proof timing on main

Written by CI after every push to `main` (`scripts/proof-timing-history.py` on `main`), from the proof timing report of each runner. Runner numbers compare commits, not phones: about 20 % between two commits can be the runner. The data is in `history/`, a line per commit.

## android-api35

**Before:** [`a702d13`](https://github.com/helius-labs/zolana-mobile-sdk/commit/a702d1357d8232522ddbb0be877975fa57253658) of 2026-10-17. **After:** [`9bcc5ac`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bcc5ac53d37d862a2ca0b9768cb32a00844cf4b) of 2026-10-17.

| by scenario | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| first run, including download | before | 5.41 s (download: 3.31 s, 8.8 MiB) | 7.81 s (download: 4.35 s, 15.6 MiB) | 13.81 s (download: 7.38 s, 28.9 MiB) | 28.59 s (download: 14.10 s, 55.9 MiB) | 50.91 s (download: 27.64 s, 109.5 MiB) | 80.02 s (download: 42.58 s, 171.6 MiB) | 76.91 s (download: 40.62 s, 165.0 MiB) |
| | after | 4.24 s (download: 2.49 s, 8.8 MiB) (-22 %) | 6.20 s (download: 3.13 s, 15.6 MiB) (-21 %) | 11.15 s (download: 5.58 s, 28.9 MiB) (-19 %) | 22.37 s (download: 10.56 s, 55.9 MiB) (-22 %) | 40.87 s (download: 20.57 s, 109.5 MiB) (-20 %) | 64.61 s (download: 31.59 s, 171.6 MiB) (-19 %) | 61.86 s (download: 29.94 s, 165.0 MiB) (-20 %) |
| warm | before | 0.94 s | 1.54 s | 2.71 s | 5.08 s | 9.29 s | 15.41 s | 14.83 s |
| | after | 0.84 s (-10 %) | 1.34 s (-13 %) | 2.41 s (-11 %) | 4.45 s (-13 %) | 8.32 s (-10 %) | 13.88 s (-10 %) | 13.45 s (-9 %) |
| first run after restart | before | 1.17 s | 1.58 s | 2.84 s | 5.36 s | 9.74 s | 16.09 s | 15.49 s |
| | after | 0.90 s (-23 %) | 1.36 s (-14 %) | 2.53 s (-11 %) | 4.61 s (-14 %) | 8.49 s (-13 %) | 14.44 s (-10 %) | 13.74 s (-11 %) |
| repeated after restart | before | 0.89 s | 1.54 s | 2.71 s | 5.04 s | 9.21 s | 15.38 s | 14.93 s |
| | after | 0.78 s (-13 %) | 1.33 s (-14 %) | 2.31 s (-15 %) | 4.42 s (-12 %) | 8.08 s (-12 %) | 14.06 s (-9 %) | 13.46 s (-10 %) |

| by stage | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| key load | before | 0.06 s | 0.03 s | 0.04 s | 0.09 s | 0.15 s | 0.24 s | 0.27 s |
| | after | 0.07 s (+23 %) | 0.03 s (-24 %) | 0.04 s (+5 %) | 0.07 s (-24 %) | 0.12 s (-19 %) | 0.20 s (-18 %) | 0.21 s (-23 %) |
| witness | before | 0.00 s | 0.00 s | 0.01 s | 0.01 s | 0.03 s | 0.04 s | 0.04 s |
| | after | 0.00 s (-50 % **!**) | 0.00 s (-38 % **!**) | 0.01 s (-15 %) | 0.01 s (+0 %) | 0.02 s (-39 % **!**) | 0.03 s (-34 % **!**) | 0.03 s (-40 % **!**) |
| prove | before | 0.94 s | 1.53 s | 2.71 s | 5.05 s | 9.39 s | 15.44 s | 14.90 s |
| | after | 0.80 s (-15 %) | 1.33 s (-13 %) | 2.36 s (-13 %) | 4.44 s (-12 %) | 8.33 s (-11 %) | 13.99 s (-9 %) | 13.42 s (-10 %) |
| verify | before | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s |
| | after | 0.00 s (-33 % **!**) | 0.00 s (+0 %) | 0.00 s (+0 %) | 0.00 s (-50 % **!**) | 0.00 s (+0 %) | 0.00 s (+0 %) | 0.00 s (+0 %) |

### Warm proof, last 3 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`9bcc5ac`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bcc5ac53d37d862a2ca0b9768cb32a00844cf4b) | 2026-10-17 | 0.84 s | 1.34 s | 2.41 s | 4.45 s | 8.32 s | 13.88 s | 13.45 s |
| [`a702d13`](https://github.com/helius-labs/zolana-mobile-sdk/commit/a702d1357d8232522ddbb0be877975fa57253658) | 2026-10-17 | 0.94 s | 1.54 s | 2.71 s | 5.08 s | 9.29 s | 15.41 s | 14.83 s |
| [`2db799e`](https://github.com/helius-labs/zolana-mobile-sdk/commit/2db799e2721bc382679855b46880fd887dd18113) | 2026-10-17 | 0.61 s | 1.07 s | 1.84 s | 3.49 s | 6.69 s | 10.57 s | 9.54 s |

### First proof after a restart, last 3 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`9bcc5ac`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bcc5ac53d37d862a2ca0b9768cb32a00844cf4b) | 2026-10-17 | 0.90 s | 1.36 s | 2.53 s | 4.61 s | 8.49 s | 14.44 s | 13.74 s |
| [`a702d13`](https://github.com/helius-labs/zolana-mobile-sdk/commit/a702d1357d8232522ddbb0be877975fa57253658) | 2026-10-17 | 1.17 s | 1.58 s | 2.84 s | 5.36 s | 9.74 s | 16.09 s | 15.49 s |
| [`2db799e`](https://github.com/helius-labs/zolana-mobile-sdk/commit/2db799e2721bc382679855b46880fd887dd18113) | 2026-10-17 | 0.86 s | 1.14 s | 2.07 s | 3.94 s | 7.68 s | 12.06 s | 10.28 s |

## ios-simulator

**Before:** [`a702d13`](https://github.com/helius-labs/zolana-mobile-sdk/commit/a702d1357d8232522ddbb0be877975fa57253658) of 2026-10-17. **After:** [`9bcc5ac`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bcc5ac53d37d862a2ca0b9768cb32a00844cf4b) of 2026-10-17.

| by scenario | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| first run, including download | before | 2.61 s (download: 1.42 s, 8.8 MiB) | 3.26 s (download: 1.07 s, 15.6 MiB) | 5.93 s (download: 1.87 s, 28.9 MiB) | 11.57 s (download: 3.51 s, 55.9 MiB) | 19.63 s (download: 6.78 s, 109.5 MiB) | 37.07 s (download: 13.18 s, 171.6 MiB) | 32.82 s (download: 10.41 s, 165.0 MiB) |
| | after | 3.69 s (download: 2.17 s, 8.8 MiB) (+42 % **!**) | 5.77 s (download: 2.48 s, 15.6 MiB) (+77 % **!**) | 11.51 s (download: 4.23 s, 28.9 MiB) (+94 % **!**) | 14.96 s (download: 6.52 s, 55.9 MiB) (+29 % **!**) | 24.38 s (download: 8.27 s, 109.5 MiB) (+24 %) | 42.28 s (download: 15.47 s, 171.6 MiB) (+14 %) | 34.67 s (download: 12.31 s, 165.0 MiB) (+6 %) |
| warm | before | 0.50 s | 1.05 s | 1.82 s | 2.56 s | 4.62 s | 9.62 s | 9.79 s |
| | after | 0.72 s (+45 % **!**) | 1.71 s (+64 % **!**) | 2.76 s (+51 % **!**) | 3.05 s (+19 %) | 5.78 s (+25 % **!**) | 11.94 s (+24 %) | 10.60 s (+8 %) |
| first run after restart | before | 0.91 s | 1.00 s | 1.65 s | 3.05 s | 6.60 s | 10.17 s | 8.76 s |
| | after | 0.82 s (-10 %) | 0.76 s (-24 %) | 2.38 s (+45 % **!**) | 3.19 s (+5 %) | 7.00 s (+6 %) | 11.68 s (+15 %) | 14.25 s (+63 % **!**) |
| repeated after restart | before | 0.41 s | 0.77 s | 1.42 s | 2.91 s | 5.37 s | 8.54 s | 8.39 s |
| | after | 0.78 s (+90 % **!**) | 0.74 s (-4 %) | 1.60 s (+13 %) | 3.78 s (+30 % **!**) | 6.44 s (+20 %) | 14.53 s (+70 % **!**) | 15.11 s (+80 % **!**) |

| by stage | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| key load | before | 0.14 s | 0.02 s | 0.04 s | 0.07 s | 0.15 s | 0.34 s | 0.19 s |
| | after | 0.11 s (-22 %) | 0.03 s (+38 % **!**) | 0.05 s (+30 % **!**) | 0.07 s (-3 %) | 0.24 s (+58 % **!**) | 0.27 s (-19 %) | 0.56 s (+190 % **!**) |
| witness | before | 0.00 s | 0.00 s | 0.00 s | 0.01 s | 0.01 s | 0.02 s | 0.02 s |
| | after | 0.00 s (+0 %) | 0.00 s (+350 % **!**) | 0.00 s (+33 % **!**) | 0.01 s (+55 % **!**) | 0.02 s (+42 % **!**) | 0.03 s (+23 %) | 0.02 s (+25 % **!**) |
| prove | before | 0.49 s | 0.87 s | 1.66 s | 2.91 s | 5.20 s | 9.71 s | 9.16 s |
| | after | 0.71 s (+44 % **!**) | 0.96 s (+10 %) | 2.51 s (+52 % **!**) | 3.08 s (+6 %) | 6.09 s (+17 %) | 11.87 s (+22 %) | 12.12 s (+32 % **!**) |
| verify | before | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s |
| | after | 0.00 s (+100 % **!**) | 0.00 s (+100 % **!**) | 0.00 s (+0 %) | 0.00 s (+100 % **!**) | 0.00 s (+0 %) | 0.00 s (+100 % **!**) | 0.00 s (+50 % **!**) |

### Warm proof, last 3 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`9bcc5ac`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bcc5ac53d37d862a2ca0b9768cb32a00844cf4b) | 2026-10-17 | 0.72 s | 1.71 s | 2.76 s | 3.05 s | 5.78 s | 11.94 s | 10.60 s |
| [`a702d13`](https://github.com/helius-labs/zolana-mobile-sdk/commit/a702d1357d8232522ddbb0be877975fa57253658) | 2026-10-17 | 0.50 s | 1.05 s | 1.82 s | 2.56 s | 4.62 s | 9.62 s | 9.79 s |
| [`2db799e`](https://github.com/helius-labs/zolana-mobile-sdk/commit/2db799e2721bc382679855b46880fd887dd18113) | 2026-10-17 | 0.39 s | 0.82 s | 1.37 s | 2.66 s | 6.67 s | 12.57 s | 14.11 s |

### First proof after a restart, last 3 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`9bcc5ac`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bcc5ac53d37d862a2ca0b9768cb32a00844cf4b) | 2026-10-17 | 0.82 s | 0.76 s | 2.38 s | 3.19 s | 7.00 s | 11.68 s | 14.25 s |
| [`a702d13`](https://github.com/helius-labs/zolana-mobile-sdk/commit/a702d1357d8232522ddbb0be877975fa57253658) | 2026-10-17 | 0.91 s | 1.00 s | 1.65 s | 3.05 s | 6.60 s | 10.17 s | 8.76 s |
| [`2db799e`](https://github.com/helius-labs/zolana-mobile-sdk/commit/2db799e2721bc382679855b46880fd887dd18113) | 2026-10-17 | 0.93 s | 1.21 s | 1.98 s | 3.43 s | 8.90 s | 11.03 s | 10.76 s |
