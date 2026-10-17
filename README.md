# Proof timing on main

Written by CI after every push to `main` (`scripts/proof-timing-history.py` on `main`), from the proof timing report of each runner. Runner numbers compare commits, not phones: about 20 % between two commits can be the runner. The data is in `history/`, a line per commit.

## android-api35

**Before:** [`9bcc5ac`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bcc5ac53d37d862a2ca0b9768cb32a00844cf4b) of 2026-10-17. **After:** [`872811d`](https://github.com/helius-labs/zolana-mobile-sdk/commit/872811d21ca00ba6b7af7f9bca3f28df21a19c3a) of 2026-10-17.

| by scenario | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| first run, including download | before | 4.24 s (download: 2.49 s, 8.8 MiB) | 6.20 s (download: 3.13 s, 15.6 MiB) | 11.15 s (download: 5.58 s, 28.9 MiB) | 22.37 s (download: 10.56 s, 55.9 MiB) | 40.87 s (download: 20.57 s, 109.5 MiB) | 64.61 s (download: 31.59 s, 171.6 MiB) | 61.86 s (download: 29.94 s, 165.0 MiB) |
| | after | 5.12 s (download: 3.07 s, 8.8 MiB) (+21 %) | 7.67 s (download: 4.12 s, 15.6 MiB) (+24 %) | 14.01 s (download: 7.42 s, 28.9 MiB) (+26 % **!**) | 29.84 s (download: 14.67 s, 55.9 MiB) (+33 % **!**) | 51.46 s (download: 27.52 s, 109.5 MiB) (+26 % **!**) | 80.57 s (download: 42.10 s, 171.6 MiB) (+25 %) | 77.98 s (download: 40.51 s, 165.0 MiB) (+26 % **!**) |
| warm | before | 0.84 s | 1.34 s | 2.41 s | 4.45 s | 8.32 s | 13.88 s | 13.45 s |
| | after | 0.95 s (+13 %) | 1.56 s (+17 %) | 2.79 s (+16 %) | 5.46 s (+23 %) | 9.72 s (+17 %) | 15.95 s (+15 %) | 15.35 s (+14 %) |
| first run after restart | before | 0.90 s | 1.36 s | 2.53 s | 4.61 s | 8.49 s | 14.44 s | 13.74 s |
| | after | 1.29 s (+44 % **!**) | 1.65 s (+22 %) | 2.90 s (+15 %) | 5.32 s (+15 %) | 9.73 s (+15 %) | 16.14 s (+12 %) | 15.31 s (+11 %) |
| repeated after restart | before | 0.78 s | 1.33 s | 2.31 s | 4.42 s | 8.08 s | 14.06 s | 13.46 s |
| | after | 0.93 s (+20 %) | 1.55 s (+17 %) | 2.75 s (+19 %) | 5.02 s (+14 %) | 9.36 s (+16 %) | 15.40 s (+10 %) | 15.07 s (+12 %) |

| by stage | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| key load | before | 0.07 s | 0.03 s | 0.04 s | 0.07 s | 0.12 s | 0.20 s | 0.21 s |
| | after | 0.05 s | 0.03 s | 0.06 s | 0.09 s | 0.15 s | 0.24 s | 0.24 s |
| witness | before | 0.00 s | 0.00 s | 0.01 s | 0.01 s | 0.02 s | 0.03 s | 0.03 s |
| | after | 0.00 s | 0.00 s | 0.01 s | 0.01 s | 0.03 s | 0.04 s | 0.03 s |
| prove | before | 0.80 s | 1.33 s | 2.36 s | 4.44 s | 8.33 s | 13.99 s | 13.42 s |
| | after | 0.94 s (+17 %) | 1.55 s (+16 %) | 2.78 s (+18 %) | 5.24 s (+18 %) | 9.62 s (+16 %) | 15.88 s (+14 %) | 15.19 s (+13 %) |

### Warm proof, last 4 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`872811d`](https://github.com/helius-labs/zolana-mobile-sdk/commit/872811d21ca00ba6b7af7f9bca3f28df21a19c3a) | 2026-10-17 | 0.95 s | 1.56 s | 2.79 s | 5.46 s | 9.72 s | 15.95 s | 15.35 s |
| [`9bcc5ac`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bcc5ac53d37d862a2ca0b9768cb32a00844cf4b) | 2026-10-17 | 0.84 s | 1.34 s | 2.41 s | 4.45 s | 8.32 s | 13.88 s | 13.45 s |
| [`a702d13`](https://github.com/helius-labs/zolana-mobile-sdk/commit/a702d1357d8232522ddbb0be877975fa57253658) | 2026-10-17 | 0.94 s | 1.54 s | 2.71 s | 5.08 s | 9.29 s | 15.41 s | 14.83 s |
| [`2db799e`](https://github.com/helius-labs/zolana-mobile-sdk/commit/2db799e2721bc382679855b46880fd887dd18113) | 2026-10-17 | 0.61 s | 1.07 s | 1.84 s | 3.49 s | 6.69 s | 10.57 s | 9.54 s |

### First proof after a restart, last 4 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`872811d`](https://github.com/helius-labs/zolana-mobile-sdk/commit/872811d21ca00ba6b7af7f9bca3f28df21a19c3a) | 2026-10-17 | 1.29 s | 1.65 s | 2.90 s | 5.32 s | 9.73 s | 16.14 s | 15.31 s |
| [`9bcc5ac`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bcc5ac53d37d862a2ca0b9768cb32a00844cf4b) | 2026-10-17 | 0.90 s | 1.36 s | 2.53 s | 4.61 s | 8.49 s | 14.44 s | 13.74 s |
| [`a702d13`](https://github.com/helius-labs/zolana-mobile-sdk/commit/a702d1357d8232522ddbb0be877975fa57253658) | 2026-10-17 | 1.17 s | 1.58 s | 2.84 s | 5.36 s | 9.74 s | 16.09 s | 15.49 s |
| [`2db799e`](https://github.com/helius-labs/zolana-mobile-sdk/commit/2db799e2721bc382679855b46880fd887dd18113) | 2026-10-17 | 0.86 s | 1.14 s | 2.07 s | 3.94 s | 7.68 s | 12.06 s | 10.28 s |

## ios-simulator

**Before:** [`9bcc5ac`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bcc5ac53d37d862a2ca0b9768cb32a00844cf4b) of 2026-10-17. **After:** [`872811d`](https://github.com/helius-labs/zolana-mobile-sdk/commit/872811d21ca00ba6b7af7f9bca3f28df21a19c3a) of 2026-10-17.

| by scenario | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| first run, including download | before | 3.69 s (download: 2.17 s, 8.8 MiB) | 5.77 s (download: 2.48 s, 15.6 MiB) | 11.51 s (download: 4.23 s, 28.9 MiB) | 14.96 s (download: 6.52 s, 55.9 MiB) | 24.38 s (download: 8.27 s, 109.5 MiB) | 42.28 s (download: 15.47 s, 171.6 MiB) | 34.67 s (download: 12.31 s, 165.0 MiB) |
| | after | 3.26 s (download: 1.61 s, 8.8 MiB) (-12 %) | 3.69 s (download: 1.49 s, 15.6 MiB) (-36 % **!**) | 9.07 s (download: 3.40 s, 28.9 MiB) (-21 %) | 14.39 s (download: 5.71 s, 55.9 MiB) (-4 %) | 29.35 s (download: 12.85 s, 109.5 MiB) (+20 %) | 47.69 s (download: 15.24 s, 171.6 MiB) (+13 %) | 40.83 s (download: 13.94 s, 165.0 MiB) (+18 %) |
| warm | before | 0.72 s | 1.71 s | 2.76 s | 3.05 s | 5.78 s | 11.94 s | 10.60 s |
| | after | 0.57 s (-21 %) | 0.93 s (-46 % **!**) | 1.88 s (-32 % **!**) | 3.56 s (+17 %) | 5.58 s (-3 %) | 10.17 s (-15 %) | 11.23 s (+6 %) |
| first run after restart | before | 0.82 s | 0.76 s | 2.38 s | 3.19 s | 7.00 s | 11.68 s | 14.25 s |
| | after | 1.09 s (+32 % **!**) | 1.50 s (+96 % **!**) | 2.35 s | 4.82 s (+51 % **!**) | 8.74 s (+25 %) | 19.50 s (+67 % **!**) | 15.14 s (+6 %) |
| repeated after restart | before | 0.78 s | 0.74 s | 1.60 s | 3.78 s | 6.44 s | 14.53 s | 15.11 s |
| | after | 0.78 s | 1.18 s (+60 % **!**) | 2.33 s (+45 % **!**) | 4.45 s (+18 %) | 8.90 s (+38 % **!**) | 12.32 s (-15 %) | 13.31 s (-12 %) |

| by stage | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| key load | before | 0.11 s | 0.03 s | 0.05 s | 0.07 s | 0.24 s | 0.27 s | 0.56 s |
| | after | 0.10 s | 0.06 s | 0.05 s | 0.17 s (+133 % **!**) | 0.17 s (-30 % **!**) | 0.51 s (+86 % **!**) | 0.48 s (-15 %) |
| witness | before | 0.00 s | 0.00 s | 0.00 s | 0.01 s | 0.02 s | 0.03 s | 0.02 s |
| | after | 0.00 s | 0.00 s | 0.00 s | 0.01 s | 0.02 s | 0.05 s | 0.04 s |
| prove | before | 0.71 s | 0.96 s | 2.51 s | 3.08 s | 6.09 s | 11.87 s | 12.12 s |
| | after | 0.75 s | 1.17 s (+21 %) | 2.12 s (-16 %) | 4.16 s (+35 % **!**) | 7.37 s (+21 %) | 13.43 s (+13 %) | 12.23 s (+1 %) |

### Warm proof, last 4 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`872811d`](https://github.com/helius-labs/zolana-mobile-sdk/commit/872811d21ca00ba6b7af7f9bca3f28df21a19c3a) | 2026-10-17 | 0.57 s | 0.93 s | 1.88 s | 3.56 s | 5.58 s | 10.17 s | 11.23 s |
| [`9bcc5ac`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bcc5ac53d37d862a2ca0b9768cb32a00844cf4b) | 2026-10-17 | 0.72 s | 1.71 s | 2.76 s | 3.05 s | 5.78 s | 11.94 s | 10.60 s |
| [`a702d13`](https://github.com/helius-labs/zolana-mobile-sdk/commit/a702d1357d8232522ddbb0be877975fa57253658) | 2026-10-17 | 0.50 s | 1.05 s | 1.82 s | 2.56 s | 4.62 s | 9.62 s | 9.79 s |
| [`2db799e`](https://github.com/helius-labs/zolana-mobile-sdk/commit/2db799e2721bc382679855b46880fd887dd18113) | 2026-10-17 | 0.39 s | 0.82 s | 1.37 s | 2.66 s | 6.67 s | 12.57 s | 14.11 s |

### First proof after a restart, last 4 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`872811d`](https://github.com/helius-labs/zolana-mobile-sdk/commit/872811d21ca00ba6b7af7f9bca3f28df21a19c3a) | 2026-10-17 | 1.09 s | 1.50 s | 2.35 s | 4.82 s | 8.74 s | 19.50 s | 15.14 s |
| [`9bcc5ac`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bcc5ac53d37d862a2ca0b9768cb32a00844cf4b) | 2026-10-17 | 0.82 s | 0.76 s | 2.38 s | 3.19 s | 7.00 s | 11.68 s | 14.25 s |
| [`a702d13`](https://github.com/helius-labs/zolana-mobile-sdk/commit/a702d1357d8232522ddbb0be877975fa57253658) | 2026-10-17 | 0.91 s | 1.00 s | 1.65 s | 3.05 s | 6.60 s | 10.17 s | 8.76 s |
| [`2db799e`](https://github.com/helius-labs/zolana-mobile-sdk/commit/2db799e2721bc382679855b46880fd887dd18113) | 2026-10-17 | 0.93 s | 1.21 s | 1.98 s | 3.43 s | 8.90 s | 11.03 s | 10.76 s |
