# Proof timing on main

Written by CI after every push to `main` (`scripts/proof-timing-history.py` on `main`), from the proof timing report of each runner. Runner numbers compare commits, not phones: about 20 % between two commits can be the runner. The data is in `history/`, a line per commit.

## android-api35

**After:** [`2db799e`](https://github.com/helius-labs/zolana-mobile-sdk/commit/2db799e2721bc382679855b46880fd887dd18113) of 2026-10-17. No report of `main` to compare with yet.

| scenario | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|
| first run, including download | 4.36 s (download: 2.94 s, 8.8 MiB) | 8.04 s (download: 5.67 s, 15.6 MiB) | 14.85 s (download: 10.56 s, 28.9 MiB) | 27.74 s (download: 19.18 s, 55.9 MiB) | 53.28 s (download: 37.63 s, 109.5 MiB) | 82.16 s (download: 57.24 s, 171.6 MiB) | 77.93 s (download: 55.21 s, 165.0 MiB) |
| warm | 0.61 s | 1.07 s | 1.84 s | 3.49 s | 6.69 s | 10.57 s | 9.54 s |
| first run after restart | 0.86 s | 1.14 s | 2.07 s | 3.94 s | 7.68 s | 12.06 s | 10.28 s |
| repeated after restart | 0.57 s | 1.02 s | 1.78 s | 3.33 s | 6.13 s | 10.29 s | 9.89 s |

| stage | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|
| key load | 0.03 s | 0.02 s | 0.07 s | 0.09 s | 0.16 s | 0.24 s | 0.25 s |
| witness | 0.00 s | 0.00 s | 0.00 s | 0.01 s | 0.01 s | 0.02 s | 0.01 s |
| prove | 0.62 s | 1.06 s | 1.84 s | 3.46 s | 6.83 s | 10.97 s | 9.74 s |
| verify | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s |

### Warm proof, last 1 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`2db799e`](https://github.com/helius-labs/zolana-mobile-sdk/commit/2db799e2721bc382679855b46880fd887dd18113) | 2026-10-17 | 0.61 s | 1.07 s | 1.84 s | 3.49 s | 6.69 s | 10.57 s | 9.54 s |

### First proof after a restart, last 1 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`2db799e`](https://github.com/helius-labs/zolana-mobile-sdk/commit/2db799e2721bc382679855b46880fd887dd18113) | 2026-10-17 | 0.86 s | 1.14 s | 2.07 s | 3.94 s | 7.68 s | 12.06 s | 10.28 s |

## ios-simulator

**After:** [`2db799e`](https://github.com/helius-labs/zolana-mobile-sdk/commit/2db799e2721bc382679855b46880fd887dd18113) of 2026-10-17. No report of `main` to compare with yet.

| scenario | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|
| first run, including download | 6.19 s (download: 5.01 s, 8.8 MiB) | 7.02 s (download: 4.65 s, 15.6 MiB) | 5.48 s (download: 1.85 s, 28.9 MiB) | 9.98 s (download: 3.94 s, 55.9 MiB) | 27.01 s (download: 10.85 s, 109.5 MiB) | 44.12 s (download: 16.99 s, 171.6 MiB) | 90.07 s (download: 50.05 s, 165.0 MiB) |
| warm | 0.39 s | 0.82 s | 1.37 s | 2.66 s | 6.67 s | 12.57 s | 14.11 s |
| first run after restart | 0.93 s | 1.21 s | 1.98 s | 3.43 s | 8.90 s | 11.03 s | 10.76 s |
| repeated after restart | 0.81 s | 0.88 s | 1.73 s | 3.92 s | 7.30 s | 11.27 s | 8.54 s |

| stage | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|
| key load | 0.14 s | 0.04 s | 0.09 s | 0.07 s | 0.20 s | 0.29 s | 0.39 s |
| witness | 0.00 s | 0.00 s | 0.00 s | 0.01 s | 0.02 s | 0.02 s | 0.02 s |
| prove | 0.63 s | 0.86 s | 1.61 s | 3.21 s | 6.96 s | 11.04 s | 12.22 s |
| verify | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s |

### Warm proof, last 1 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`2db799e`](https://github.com/helius-labs/zolana-mobile-sdk/commit/2db799e2721bc382679855b46880fd887dd18113) | 2026-10-17 | 0.39 s | 0.82 s | 1.37 s | 2.66 s | 6.67 s | 12.57 s | 14.11 s |

### First proof after a restart, last 1 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`2db799e`](https://github.com/helius-labs/zolana-mobile-sdk/commit/2db799e2721bc382679855b46880fd887dd18113) | 2026-10-17 | 0.93 s | 1.21 s | 1.98 s | 3.43 s | 8.90 s | 11.03 s | 10.76 s |
