# Proof timing on main

Written by CI after every push to `main` (`scripts/proof-timing-history.py` on `main`), from the proof timing report of each runner. Runner numbers compare commits, not phones: about 20 % between two commits can be the runner. The data is in `history/`, a line per commit.

## android-api35

**Before:** [`2db799e`](https://github.com/helius-labs/zolana-mobile-sdk/commit/2db799e2721bc382679855b46880fd887dd18113) of 2026-10-17. **After:** [`a702d13`](https://github.com/helius-labs/zolana-mobile-sdk/commit/a702d1357d8232522ddbb0be877975fa57253658) of 2026-10-17.

| by scenario | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| first run, including download | before | 4.36 s (download: 2.94 s, 8.8 MiB) | 8.04 s (download: 5.67 s, 15.6 MiB) | 14.85 s (download: 10.56 s, 28.9 MiB) | 27.74 s (download: 19.18 s, 55.9 MiB) | 53.28 s (download: 37.63 s, 109.5 MiB) | 82.16 s (download: 57.24 s, 171.6 MiB) | 77.93 s (download: 55.21 s, 165.0 MiB) |
| | after | 5.41 s (download: 3.31 s, 8.8 MiB) (+24 %) | 7.81 s (download: 4.35 s, 15.6 MiB) (-3 %) | 13.81 s (download: 7.38 s, 28.9 MiB) (-7 %) | 28.59 s (download: 14.10 s, 55.9 MiB) (+3 %) | 50.91 s (download: 27.64 s, 109.5 MiB) (-4 %) | 80.02 s (download: 42.58 s, 171.6 MiB) (-3 %) | 76.91 s (download: 40.62 s, 165.0 MiB) (-1 %) |
| warm | before | 0.61 s | 1.07 s | 1.84 s | 3.49 s | 6.69 s | 10.57 s | 9.54 s |
| | after | 0.94 s (+53 % **!**) | 1.54 s (+44 % **!**) | 2.71 s (+47 % **!**) | 5.08 s (+46 % **!**) | 9.29 s (+39 % **!**) | 15.41 s (+46 % **!**) | 14.83 s (+56 % **!**) |
| first run after restart | before | 0.86 s | 1.14 s | 2.07 s | 3.94 s | 7.68 s | 12.06 s | 10.28 s |
| | after | 1.17 s (+35 % **!**) | 1.58 s (+39 % **!**) | 2.84 s (+37 % **!**) | 5.36 s (+36 % **!**) | 9.74 s (+27 % **!**) | 16.09 s (+33 % **!**) | 15.49 s (+51 % **!**) |
| repeated after restart | before | 0.57 s | 1.02 s | 1.78 s | 3.33 s | 6.13 s | 10.29 s | 9.89 s |
| | after | 0.89 s (+57 % **!**) | 1.54 s (+51 % **!**) | 2.71 s (+52 % **!**) | 5.04 s (+51 % **!**) | 9.21 s (+50 % **!**) | 15.38 s (+49 % **!**) | 14.93 s (+51 % **!**) |

| by stage | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| key load | before | 0.03 s | 0.02 s | 0.07 s | 0.09 s | 0.16 s | 0.24 s | 0.25 s |
| | after | 0.06 s (+140 % **!**) | 0.03 s (+43 % **!**) | 0.04 s (-41 % **!**) | 0.09 s (-2 %) | 0.15 s (-6 %) | 0.24 s (+0 %) | 0.27 s (+9 %) |
| witness | before | 0.00 s | 0.00 s | 0.00 s | 0.01 s | 0.01 s | 0.02 s | 0.01 s |
| | after | 0.00 s (+100 % **!**) | 0.00 s (+100 % **!**) | 0.01 s (+86 % **!**) | 0.01 s (+25 % **!**) | 0.03 s (+89 % **!**) | 0.04 s (+69 % **!**) | 0.04 s (+394 % **!**) |
| prove | before | 0.62 s | 1.06 s | 1.84 s | 3.46 s | 6.83 s | 10.97 s | 9.74 s |
| | after | 0.94 s (+52 % **!**) | 1.53 s (+45 % **!**) | 2.71 s (+48 % **!**) | 5.05 s (+46 % **!**) | 9.39 s (+37 % **!**) | 15.44 s (+41 % **!**) | 14.90 s (+53 % **!**) |
| verify | before | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s |
| | after | 0.00 s (+50 % **!**) | 0.00 s (+0 %) | 0.00 s (+0 %) | 0.00 s (+100 % **!**) | 0.00 s (+0 %) | 0.00 s (+0 %) | 0.00 s (+0 %) |

### Warm proof, last 2 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`a702d13`](https://github.com/helius-labs/zolana-mobile-sdk/commit/a702d1357d8232522ddbb0be877975fa57253658) | 2026-10-17 | 0.94 s | 1.54 s | 2.71 s | 5.08 s | 9.29 s | 15.41 s | 14.83 s |
| [`2db799e`](https://github.com/helius-labs/zolana-mobile-sdk/commit/2db799e2721bc382679855b46880fd887dd18113) | 2026-10-17 | 0.61 s | 1.07 s | 1.84 s | 3.49 s | 6.69 s | 10.57 s | 9.54 s |

### First proof after a restart, last 2 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`a702d13`](https://github.com/helius-labs/zolana-mobile-sdk/commit/a702d1357d8232522ddbb0be877975fa57253658) | 2026-10-17 | 1.17 s | 1.58 s | 2.84 s | 5.36 s | 9.74 s | 16.09 s | 15.49 s |
| [`2db799e`](https://github.com/helius-labs/zolana-mobile-sdk/commit/2db799e2721bc382679855b46880fd887dd18113) | 2026-10-17 | 0.86 s | 1.14 s | 2.07 s | 3.94 s | 7.68 s | 12.06 s | 10.28 s |

## ios-simulator

**Before:** [`2db799e`](https://github.com/helius-labs/zolana-mobile-sdk/commit/2db799e2721bc382679855b46880fd887dd18113) of 2026-10-17. **After:** [`a702d13`](https://github.com/helius-labs/zolana-mobile-sdk/commit/a702d1357d8232522ddbb0be877975fa57253658) of 2026-10-17.

| by scenario | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| first run, including download | before | 6.19 s (download: 5.01 s, 8.8 MiB) | 7.02 s (download: 4.65 s, 15.6 MiB) | 5.48 s (download: 1.85 s, 28.9 MiB) | 9.98 s (download: 3.94 s, 55.9 MiB) | 27.01 s (download: 10.85 s, 109.5 MiB) | 44.12 s (download: 16.99 s, 171.6 MiB) | 90.07 s (download: 50.05 s, 165.0 MiB) |
| | after | 2.61 s (download: 1.42 s, 8.8 MiB) (-58 % **!**) | 3.26 s (download: 1.07 s, 15.6 MiB) (-54 % **!**) | 5.93 s (download: 1.87 s, 28.9 MiB) (+8 %) | 11.57 s (download: 3.51 s, 55.9 MiB) (+16 %) | 19.63 s (download: 6.78 s, 109.5 MiB) (-27 % **!**) | 37.07 s (download: 13.18 s, 171.6 MiB) (-16 %) | 32.82 s (download: 10.41 s, 165.0 MiB) (-64 % **!**) |
| warm | before | 0.39 s | 0.82 s | 1.37 s | 2.66 s | 6.67 s | 12.57 s | 14.11 s |
| | after | 0.50 s (+26 % **!**) | 1.05 s (+27 % **!**) | 1.82 s (+33 % **!**) | 2.56 s (-4 %) | 4.62 s (-31 % **!**) | 9.62 s (-24 %) | 9.79 s (-31 % **!**) |
| first run after restart | before | 0.93 s | 1.21 s | 1.98 s | 3.43 s | 8.90 s | 11.03 s | 10.76 s |
| | after | 0.91 s (-2 %) | 1.00 s (-17 %) | 1.65 s (-17 %) | 3.05 s (-11 %) | 6.60 s (-26 % **!**) | 10.17 s (-8 %) | 8.76 s (-19 %) |
| repeated after restart | before | 0.81 s | 0.88 s | 1.73 s | 3.92 s | 7.30 s | 11.27 s | 8.54 s |
| | after | 0.41 s (-49 % **!**) | 0.77 s (-12 %) | 1.42 s (-18 %) | 2.91 s (-26 % **!**) | 5.37 s (-26 % **!**) | 8.54 s (-24 %) | 8.39 s (-2 %) |

| by stage | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| key load | before | 0.14 s | 0.04 s | 0.09 s | 0.07 s | 0.20 s | 0.29 s | 0.39 s |
| | after | 0.14 s (-1 %) | 0.02 s (-45 % **!**) | 0.04 s (-58 % **!**) | 0.07 s (+3 %) | 0.15 s (-24 %) | 0.34 s (+15 %) | 0.19 s (-50 % **!**) |
| witness | before | 0.00 s | 0.00 s | 0.00 s | 0.01 s | 0.02 s | 0.02 s | 0.02 s |
| | after | 0.00 s (+0 %) | 0.00 s (-50 % **!**) | 0.00 s (+0 %) | 0.01 s (+0 %) | 0.01 s (-44 % **!**) | 0.02 s (+0 %) | 0.02 s (-9 %) |
| prove | before | 0.63 s | 0.86 s | 1.61 s | 3.21 s | 6.96 s | 11.04 s | 12.22 s |
| | after | 0.49 s (-22 %) | 0.87 s (+1 %) | 1.66 s (+3 %) | 2.91 s (-9 %) | 5.20 s (-25 % **!**) | 9.71 s (-12 %) | 9.16 s (-25 % **!**) |
| verify | before | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s |
| | after | 0.00 s (-33 % **!**) | 0.00 s (-33 % **!**) | 0.00 s (+0 %) | 0.00 s (+0 %) | 0.00 s (-33 % **!**) | 0.00 s (-50 % **!**) | 0.00 s (-33 % **!**) |

### Warm proof, last 2 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`a702d13`](https://github.com/helius-labs/zolana-mobile-sdk/commit/a702d1357d8232522ddbb0be877975fa57253658) | 2026-10-17 | 0.50 s | 1.05 s | 1.82 s | 2.56 s | 4.62 s | 9.62 s | 9.79 s |
| [`2db799e`](https://github.com/helius-labs/zolana-mobile-sdk/commit/2db799e2721bc382679855b46880fd887dd18113) | 2026-10-17 | 0.39 s | 0.82 s | 1.37 s | 2.66 s | 6.67 s | 12.57 s | 14.11 s |

### First proof after a restart, last 2 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`a702d13`](https://github.com/helius-labs/zolana-mobile-sdk/commit/a702d1357d8232522ddbb0be877975fa57253658) | 2026-10-17 | 0.91 s | 1.00 s | 1.65 s | 3.05 s | 6.60 s | 10.17 s | 8.76 s |
| [`2db799e`](https://github.com/helius-labs/zolana-mobile-sdk/commit/2db799e2721bc382679855b46880fd887dd18113) | 2026-10-17 | 0.93 s | 1.21 s | 1.98 s | 3.43 s | 8.90 s | 11.03 s | 10.76 s |
