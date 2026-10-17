# Proof timing on main

Written by CI after every push to `main` (`scripts/proof-timing-history.py` on `main`), from the proof timing report of each runner. Runner numbers compare commits, not phones: about 20 % between two commits can be the runner. The data is in `history/`, a line per commit.

## android-api35

**Before:** [`872811d`](https://github.com/helius-labs/zolana-mobile-sdk/commit/872811d21ca00ba6b7af7f9bca3f28df21a19c3a) of 2026-10-17. **After:** [`c5d1789`](https://github.com/helius-labs/zolana-mobile-sdk/commit/c5d178922db98de6aa9c2e2083f1e2a7975296e1) of 2026-10-17.

| by scenario | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| first run, including download | before | 5.12 s (download: 3.07 s, 8.8 MiB) | 7.67 s (download: 4.12 s, 15.6 MiB) | 14.01 s (download: 7.42 s, 28.9 MiB) | 29.84 s (download: 14.67 s, 55.9 MiB) | 51.46 s (download: 27.52 s, 109.5 MiB) | 80.57 s (download: 42.10 s, 171.6 MiB) | 77.98 s (download: 40.51 s, 165.0 MiB) |
| | after | 3.29 s (download: 1.90 s, 8.8 MiB) (-36 % **!**) | 4.96 s (download: 2.47 s, 15.6 MiB) (-35 % **!**) | 8.44 s (download: 4.20 s, 28.9 MiB) (-40 % **!**) | 16.48 s (download: 8.49 s, 55.9 MiB) (-45 % **!**) | 32.28 s (download: 16.55 s, 109.5 MiB) (-37 % **!**) | 51.71 s (download: 25.87 s, 171.6 MiB) (-36 % **!**) | 47.13 s (download: 23.84 s, 165.0 MiB) (-40 % **!**) |
| warm | before | 0.95 s | 1.56 s | 2.79 s | 5.46 s | 9.72 s | 15.95 s | 15.35 s |
| | after | 0.61 s (-36 % **!**) | 1.04 s (-33 % **!**) | 1.80 s (-36 % **!**) | 3.33 s (-39 % **!**) | 6.60 s (-32 % **!**) | 10.65 s (-33 % **!**) | 9.88 s (-36 % **!**) |
| first run after restart | before | 1.29 s | 1.65 s | 2.90 s | 5.32 s | 9.73 s | 16.14 s | 15.31 s |
| | after | 0.86 s (-33 % **!**) | 1.20 s (-27 % **!**) | 2.04 s (-30 % **!**) | 4.22 s (-21 %) | 7.59 s (-22 %) | 12.52 s (-22 %) | 10.43 s (-32 % **!**) |
| repeated after restart | before | 0.93 s | 1.55 s | 2.75 s | 5.02 s | 9.36 s | 15.40 s | 15.07 s |
| | after | 0.58 s (-38 % **!**) | 1.07 s (-31 % **!**) | 1.82 s (-34 % **!**) | 3.43 s (-32 % **!**) | 6.48 s (-31 % **!**) | 10.86 s (-29 % **!**) | 10.51 s (-30 % **!**) |

| by stage | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| key load | before | 0.05 s | 0.03 s | 0.06 s | 0.09 s | 0.15 s | 0.24 s | 0.24 s |
| | after | 0.04 s | 0.03 s | 0.04 s | 0.05 s | 0.12 s | 0.16 s (-35 % **!**) | 0.16 s (-34 % **!**) |
| witness | before | 0.00 s | 0.00 s | 0.01 s | 0.01 s | 0.03 s | 0.04 s | 0.03 s |
| | after | 0.00 s | 0.00 s | 0.01 s | 0.01 s | 0.01 s | 0.03 s | 0.02 s |
| prove | before | 0.94 s | 1.55 s | 2.78 s | 5.24 s | 9.62 s | 15.88 s | 15.19 s |
| | after | 0.60 s (-36 % **!**) | 1.08 s (-31 % **!**) | 1.81 s (-35 % **!**) | 3.42 s (-35 % **!**) | 6.73 s (-30 % **!**) | 11.28 s (-29 % **!**) | 10.12 s (-33 % **!**) |

### Warm proof, last 5 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`c5d1789`](https://github.com/helius-labs/zolana-mobile-sdk/commit/c5d178922db98de6aa9c2e2083f1e2a7975296e1) | 2026-10-17 | 0.61 s | 1.04 s | 1.80 s | 3.33 s | 6.60 s | 10.65 s | 9.88 s |
| [`872811d`](https://github.com/helius-labs/zolana-mobile-sdk/commit/872811d21ca00ba6b7af7f9bca3f28df21a19c3a) | 2026-10-17 | 0.95 s | 1.56 s | 2.79 s | 5.46 s | 9.72 s | 15.95 s | 15.35 s |
| [`9bcc5ac`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bcc5ac53d37d862a2ca0b9768cb32a00844cf4b) | 2026-10-17 | 0.84 s | 1.34 s | 2.41 s | 4.45 s | 8.32 s | 13.88 s | 13.45 s |
| [`a702d13`](https://github.com/helius-labs/zolana-mobile-sdk/commit/a702d1357d8232522ddbb0be877975fa57253658) | 2026-10-17 | 0.94 s | 1.54 s | 2.71 s | 5.08 s | 9.29 s | 15.41 s | 14.83 s |
| [`2db799e`](https://github.com/helius-labs/zolana-mobile-sdk/commit/2db799e2721bc382679855b46880fd887dd18113) | 2026-10-17 | 0.61 s | 1.07 s | 1.84 s | 3.49 s | 6.69 s | 10.57 s | 9.54 s |

### First proof after a restart, last 5 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`c5d1789`](https://github.com/helius-labs/zolana-mobile-sdk/commit/c5d178922db98de6aa9c2e2083f1e2a7975296e1) | 2026-10-17 | 0.86 s | 1.20 s | 2.04 s | 4.22 s | 7.59 s | 12.52 s | 10.43 s |
| [`872811d`](https://github.com/helius-labs/zolana-mobile-sdk/commit/872811d21ca00ba6b7af7f9bca3f28df21a19c3a) | 2026-10-17 | 1.29 s | 1.65 s | 2.90 s | 5.32 s | 9.73 s | 16.14 s | 15.31 s |
| [`9bcc5ac`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bcc5ac53d37d862a2ca0b9768cb32a00844cf4b) | 2026-10-17 | 0.90 s | 1.36 s | 2.53 s | 4.61 s | 8.49 s | 14.44 s | 13.74 s |
| [`a702d13`](https://github.com/helius-labs/zolana-mobile-sdk/commit/a702d1357d8232522ddbb0be877975fa57253658) | 2026-10-17 | 1.17 s | 1.58 s | 2.84 s | 5.36 s | 9.74 s | 16.09 s | 15.49 s |
| [`2db799e`](https://github.com/helius-labs/zolana-mobile-sdk/commit/2db799e2721bc382679855b46880fd887dd18113) | 2026-10-17 | 0.86 s | 1.14 s | 2.07 s | 3.94 s | 7.68 s | 12.06 s | 10.28 s |

## ios-simulator

**Before:** [`872811d`](https://github.com/helius-labs/zolana-mobile-sdk/commit/872811d21ca00ba6b7af7f9bca3f28df21a19c3a) of 2026-10-17. **After:** [`c5d1789`](https://github.com/helius-labs/zolana-mobile-sdk/commit/c5d178922db98de6aa9c2e2083f1e2a7975296e1) of 2026-10-17.

| by scenario | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| first run, including download | before | 3.26 s (download: 1.61 s, 8.8 MiB) | 3.69 s (download: 1.49 s, 15.6 MiB) | 9.07 s (download: 3.40 s, 28.9 MiB) | 14.39 s (download: 5.71 s, 55.9 MiB) | 29.35 s (download: 12.85 s, 109.5 MiB) | 47.69 s (download: 15.24 s, 171.6 MiB) | 40.83 s (download: 13.94 s, 165.0 MiB) |
| | after | 4.88 s (download: 2.07 s, 8.8 MiB) (+50 % **!**) | 6.24 s (download: 2.63 s, 15.6 MiB) (+69 % **!**) | 10.22 s (download: 4.10 s, 28.9 MiB) (+13 %) | 14.23 s (download: 5.57 s, 55.9 MiB) (-1 %) | 26.36 s (download: 11.67 s, 109.5 MiB) (-10 %) | 43.96 s (download: 16.22 s, 171.6 MiB) (-8 %) | 36.26 s (download: 12.71 s, 165.0 MiB) (-11 %) |
| warm | before | 0.57 s | 0.93 s | 1.88 s | 3.56 s | 5.58 s | 10.17 s | 11.23 s |
| | after | 1.20 s (+110 % **!**) | 1.41 s (+51 % **!**) | 2.46 s (+31 % **!**) | 3.31 s (-7 %) | 6.80 s (+22 %) | 11.01 s (+8 %) | 9.21 s (-18 %) |
| first run after restart | before | 1.09 s | 1.50 s | 2.35 s | 4.82 s | 8.74 s | 19.50 s | 15.14 s |
| | after | 0.71 s (-35 % **!**) | 0.81 s (-46 % **!**) | 1.53 s (-35 % **!**) | 2.78 s (-42 % **!**) | 5.26 s (-40 % **!**) | 9.05 s (-54 % **!**) | 12.02 s (-21 %) |
| repeated after restart | before | 0.78 s | 1.18 s | 2.33 s | 4.45 s | 8.90 s | 12.32 s | 13.31 s |
| | after | 0.43 s (-45 % **!**) | 0.83 s (-30 % **!**) | 1.31 s (-44 % **!**) | 2.52 s (-43 % **!**) | 4.56 s (-49 % **!**) | 9.47 s (-23 %) | 8.79 s (-34 % **!**) |

| by stage | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| key load | before | 0.10 s | 0.06 s | 0.05 s | 0.17 s | 0.17 s | 0.51 s | 0.48 s |
| | after | 0.09 s | 0.04 s | 0.05 s | 0.06 s (-67 % **!**) | 0.13 s | 0.27 s (-48 % **!**) | 0.65 s (+35 % **!**) |
| witness | before | 0.00 s | 0.00 s | 0.00 s | 0.01 s | 0.02 s | 0.05 s | 0.04 s |
| | after | 0.00 s | 0.00 s | 0.01 s | 0.01 s | 0.01 s | 0.02 s | 0.02 s |
| prove | before | 0.75 s | 1.17 s | 2.12 s | 4.16 s | 7.37 s | 13.43 s | 12.23 s |
| | after | 0.67 s (-11 %) | 1.11 s (-5 %) | 1.83 s (-14 %) | 2.96 s (-29 % **!**) | 5.49 s (-25 % **!**) | 10.05 s (-25 % **!**) | 9.18 s (-25 %) |

### Warm proof, last 5 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`c5d1789`](https://github.com/helius-labs/zolana-mobile-sdk/commit/c5d178922db98de6aa9c2e2083f1e2a7975296e1) | 2026-10-17 | 1.20 s | 1.41 s | 2.46 s | 3.31 s | 6.80 s | 11.01 s | 9.21 s |
| [`872811d`](https://github.com/helius-labs/zolana-mobile-sdk/commit/872811d21ca00ba6b7af7f9bca3f28df21a19c3a) | 2026-10-17 | 0.57 s | 0.93 s | 1.88 s | 3.56 s | 5.58 s | 10.17 s | 11.23 s |
| [`9bcc5ac`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bcc5ac53d37d862a2ca0b9768cb32a00844cf4b) | 2026-10-17 | 0.72 s | 1.71 s | 2.76 s | 3.05 s | 5.78 s | 11.94 s | 10.60 s |
| [`a702d13`](https://github.com/helius-labs/zolana-mobile-sdk/commit/a702d1357d8232522ddbb0be877975fa57253658) | 2026-10-17 | 0.50 s | 1.05 s | 1.82 s | 2.56 s | 4.62 s | 9.62 s | 9.79 s |
| [`2db799e`](https://github.com/helius-labs/zolana-mobile-sdk/commit/2db799e2721bc382679855b46880fd887dd18113) | 2026-10-17 | 0.39 s | 0.82 s | 1.37 s | 2.66 s | 6.67 s | 12.57 s | 14.11 s |

### First proof after a restart, last 5 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`c5d1789`](https://github.com/helius-labs/zolana-mobile-sdk/commit/c5d178922db98de6aa9c2e2083f1e2a7975296e1) | 2026-10-17 | 0.71 s | 0.81 s | 1.53 s | 2.78 s | 5.26 s | 9.05 s | 12.02 s |
| [`872811d`](https://github.com/helius-labs/zolana-mobile-sdk/commit/872811d21ca00ba6b7af7f9bca3f28df21a19c3a) | 2026-10-17 | 1.09 s | 1.50 s | 2.35 s | 4.82 s | 8.74 s | 19.50 s | 15.14 s |
| [`9bcc5ac`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bcc5ac53d37d862a2ca0b9768cb32a00844cf4b) | 2026-10-17 | 0.82 s | 0.76 s | 2.38 s | 3.19 s | 7.00 s | 11.68 s | 14.25 s |
| [`a702d13`](https://github.com/helius-labs/zolana-mobile-sdk/commit/a702d1357d8232522ddbb0be877975fa57253658) | 2026-10-17 | 0.91 s | 1.00 s | 1.65 s | 3.05 s | 6.60 s | 10.17 s | 8.76 s |
| [`2db799e`](https://github.com/helius-labs/zolana-mobile-sdk/commit/2db799e2721bc382679855b46880fd887dd18113) | 2026-10-17 | 0.93 s | 1.21 s | 1.98 s | 3.43 s | 8.90 s | 11.03 s | 10.76 s |
