# Proof timing on main

Written by CI after every push to `main` (`scripts/proof-timing-history.py` on `main`), from release builds on the phones of `scripts/devicefarm/devices.json` in AWS Device Farm. Two runs of one commit differ by up to 5 % on the Galaxy A16 and A56 and up to 15 % on the S25 and the iPhones. The data is in `history/`, a line per commit.

## Samsung Galaxy A16, Android 14, low tier

**Before:** [`7919e1b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/7919e1b763751e7f483b08f8b757bd0aade73625) of 2026-10-20. **After:** [`3ca9427`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3ca94276c3576e8bcf54e80e3fea0741b3ba5872) of 2026-10-21.

| by scenario | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| first run, including download | before | 5.93 s (download: 3.16 s, 8.8 MiB) | 11.70 s (download: 7.13 s, 15.6 MiB) | 21.21 s (download: 12.93 s, 28.9 MiB) | 40.37 s (download: 23.89 s, 55.9 MiB) | 81.12 s (download: 50.54 s, 109.5 MiB) | 128.67 s (download: 75.87 s, 171.6 MiB) | 115.64 s (download: 68.54 s, 165.0 MiB) |
| | after | 5.82 s (download: 3.06 s, 8.8 MiB) (-2 %) | 11.23 s (download: 6.70 s, 15.6 MiB) (-4 %) | 20.81 s (download: 12.44 s, 28.9 MiB) (-2 %) | 39.14 s (download: 23.45 s, 55.9 MiB) (-3 %) | 76.41 s (download: 46.06 s, 109.5 MiB) (-6 %) | 123.48 s (download: 70.87 s, 171.6 MiB) (-4 %) | 111.84 s (download: 64.36 s, 165.0 MiB) (-3 %) |
| warm | before | 1.23 s | 2.13 s | 3.70 s | 7.23 s | 15.33 s | 25.38 s | 22.56 s |
| | after | 1.25 s | 2.07 s (-3 %) | 3.72 s | 7.12 s (-1 %) | 15.21 s (-1 %) | 26.01 s (+2 %) | 22.89 s (+1 %) |
| first run after restart | before | 1.19 s | 2.18 s | 3.85 s | 7.36 s | 15.73 s | 26.20 s | 23.13 s |
| | after | 1.25 s (+5 %) | 2.17 s | 3.91 s (+2 %) | 7.15 s (-3 %) | 15.55 s (-1 %) | 26.42 s (+1 %) | 22.93 s (-1 %) |
| repeated after restart | before | 1.22 s | 2.06 s | 3.75 s | 7.14 s | 15.23 s | 26.02 s | 21.51 s |
| | after | 1.22 s | 2.07 s | 3.68 s (-2 %) | 7.17 s | 15.17 s | 26.19 s (+1 %) | 22.75 s (+6 %) |

| by stage | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| key load | before | 0.06 s | 0.11 s | 0.13 s | 0.24 s | 0.38 s | 0.68 s | 0.86 s |
| | after | 0.06 s | 0.09 s | 0.15 s | 0.20 s | 0.39 s | 0.61 s (-10 %) | 0.66 s (-23 % **!**) |
| witness | before | 0.03 s | 0.03 s | 0.05 s | 0.13 s | 0.27 s | 0.26 s | 0.09 s |
| | after | 0.02 s | 0.04 s | 0.07 s | 0.10 s | 0.26 s | 0.38 s (+46 % **!**) | 0.20 s (+134 % **!**) |
| prove | before | 1.19 s | 2.01 s | 3.64 s | 7.07 s | 14.97 s | 25.17 s | 21.78 s |
| | after | 1.19 s | 2.04 s | 3.62 s | 6.92 s (-2 %) | 14.86 s (-1 %) | 25.51 s (+1 %) | 22.29 s (+2 %) |

### Proof, last 6 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`3ca9427`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3ca94276c3576e8bcf54e80e3fea0741b3ba5872) | 2026-10-21 | 1.19 s | 2.04 s | 3.62 s | 6.92 s | 14.86 s | 25.51 s | 22.29 s |
| [`7919e1b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/7919e1b763751e7f483b08f8b757bd0aade73625) | 2026-10-20 | 1.19 s | 2.01 s | 3.64 s | 7.07 s | 14.97 s | 25.17 s | 21.78 s |
| [`9bde901`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bde901088fa31b64651906cdbefcf755372c6f8) | 2026-10-19 | 1.20 s | 2.00 s | 3.65 s | 7.01 s | 14.83 s | 25.25 s | 22.09 s |
| [`00b2dbe`](https://github.com/helius-labs/zolana-mobile-sdk/commit/00b2dbe2265c90ce984c6634e5227b65027730c2) | 2026-10-19 | 1.19 s | 2.04 s | 3.65 s | 7.03 s | 14.89 s | 24.47 s | 22.16 s |
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 1.18 s | 2.02 s | 3.66 s | 7.07 s | 14.96 s | 24.88 s | 22.09 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 1.19 s | 2.01 s | 3.69 s | 6.99 s | 14.92 s | 23.81 s | 21.72 s |

### First run after a restart, last 6 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`3ca9427`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3ca94276c3576e8bcf54e80e3fea0741b3ba5872) | 2026-10-21 | 1.25 s | 2.17 s | 3.91 s | 7.15 s | 15.55 s | 26.42 s | 22.93 s |
| [`7919e1b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/7919e1b763751e7f483b08f8b757bd0aade73625) | 2026-10-20 | 1.19 s | 2.18 s | 3.85 s | 7.36 s | 15.73 s | 26.20 s | 23.13 s |
| [`9bde901`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bde901088fa31b64651906cdbefcf755372c6f8) | 2026-10-19 | 1.21 s | 2.14 s | 3.99 s | 7.31 s | 15.69 s | 26.41 s | 22.79 s |
| [`00b2dbe`](https://github.com/helius-labs/zolana-mobile-sdk/commit/00b2dbe2265c90ce984c6634e5227b65027730c2) | 2026-10-19 | 1.19 s | 2.23 s | 3.87 s | 7.47 s | 15.73 s | 26.60 s | 23.12 s |
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 1.15 s | 2.18 s | 3.80 s | 7.49 s | 15.63 s | 25.46 s | 23.25 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 1.27 s | 2.12 s | 3.93 s | 7.28 s | 15.57 s | 25.73 s | 22.95 s |

## Samsung Galaxy A56, Android 15, mid tier

**Before:** [`7919e1b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/7919e1b763751e7f483b08f8b757bd0aade73625) of 2026-10-20. **After:** [`3ca9427`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3ca94276c3576e8bcf54e80e3fea0741b3ba5872) of 2026-10-21.

| by scenario | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| first run, including download | before | 2.53 s (download: 1.68 s, 8.8 MiB) | 3.64 s (download: 2.22 s, 15.6 MiB) | 7.63 s (download: 4.98 s, 28.9 MiB) | 12.87 s (download: 7.91 s, 55.9 MiB) | 25.98 s (download: 16.38 s, 109.5 MiB) | 40.36 s (download: 25.09 s, 171.6 MiB) | 39.97 s (download: 25.06 s, 165.0 MiB) |
| | after | 4.35 s (download: 3.48 s, 8.8 MiB) (+72 % **!**) | 6.81 s (download: 5.38 s, 15.6 MiB) (+87 % **!**) | 9.83 s (download: 7.25 s, 28.9 MiB) (+29 % **!**) | 25.69 s (download: 20.72 s, 55.9 MiB) (+100 % **!**) | 52.03 s (download: 42.23 s, 109.5 MiB) (+100 % **!**) | 135.49 s (download: 120.36 s, 171.6 MiB) (+236 % **!**) | 50.44 s (download: 35.75 s, 165.0 MiB) (+26 % **!**) |
| warm | before | 0.34 s | 0.58 s | 1.07 s | 2.01 s | 3.78 s | 6.11 s | 5.78 s |
| | after | 0.34 s | 0.57 s | 1.04 s | 2.00 s | 3.79 s | 6.04 s (-1 %) | 5.80 s |
| first run after restart | before | 0.56 s | 0.65 s | 1.14 s | 2.13 s | 4.07 s | 6.37 s | 6.13 s |
| | after | 0.56 s | 0.66 s | 1.15 s | 2.07 s (-3 %) | 4.11 s | 6.32 s | 6.14 s |
| repeated after restart | before | 0.35 s | 0.60 s | 1.08 s | 2.01 s | 3.75 s | 6.27 s | 5.90 s |
| | after | 0.35 s | 0.59 s | 1.06 s | 2.02 s | 3.80 s (+1 %) | 6.09 s (-3 %) | 5.82 s (-1 %) |

| by stage | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| key load | before | 0.06 s | 0.03 s | 0.04 s | 0.09 s | 0.15 s | 0.22 s | 0.20 s |
| | after | 0.11 s (+87 % **!**) | 0.03 s | 0.04 s | 0.07 s | 0.15 s | 0.23 s | 0.25 s |
| witness | before | 0.00 s | 0.01 s | 0.01 s | 0.02 s | 0.04 s | 0.06 s | 0.04 s |
| | after | 0.00 s | 0.01 s | 0.01 s | 0.02 s | 0.04 s | 0.05 s | 0.05 s |
| prove | before | 0.34 s | 0.58 s | 1.06 s | 1.99 s | 3.77 s | 6.06 s | 5.80 s |
| | after | 0.34 s | 0.58 s | 1.05 s | 1.98 s | 3.79 s | 6.00 s (-1 %) | 5.76 s |

### Proof, last 6 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`3ca9427`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3ca94276c3576e8bcf54e80e3fea0741b3ba5872) | 2026-10-21 | 0.34 s | 0.58 s | 1.05 s | 1.98 s | 3.79 s | 6.00 s | 5.76 s |
| [`7919e1b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/7919e1b763751e7f483b08f8b757bd0aade73625) | 2026-10-20 | 0.34 s | 0.58 s | 1.06 s | 1.99 s | 3.77 s | 6.06 s | 5.80 s |
| [`9bde901`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bde901088fa31b64651906cdbefcf755372c6f8) | 2026-10-19 | 0.34 s | 0.58 s | 1.07 s | 2.00 s | 3.92 s | 6.03 s | 5.76 s |
| [`00b2dbe`](https://github.com/helius-labs/zolana-mobile-sdk/commit/00b2dbe2265c90ce984c6634e5227b65027730c2) | 2026-10-19 | 0.34 s | 0.58 s | 1.05 s | 2.00 s | 3.73 s | 6.05 s | 5.70 s |
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 0.34 s | 0.58 s | 1.06 s | 2.00 s | 3.90 s | 6.08 s | 5.80 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.35 s | 0.57 s | 1.06 s | 1.98 s | 3.87 s | 6.00 s | 5.78 s |

### First run after a restart, last 6 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`3ca9427`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3ca94276c3576e8bcf54e80e3fea0741b3ba5872) | 2026-10-21 | 0.56 s | 0.66 s | 1.15 s | 2.07 s | 4.11 s | 6.32 s | 6.14 s |
| [`7919e1b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/7919e1b763751e7f483b08f8b757bd0aade73625) | 2026-10-20 | 0.56 s | 0.65 s | 1.14 s | 2.13 s | 4.07 s | 6.37 s | 6.13 s |
| [`9bde901`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bde901088fa31b64651906cdbefcf755372c6f8) | 2026-10-19 | 0.57 s | 0.64 s | 1.17 s | 2.13 s | 4.15 s | 6.36 s | 6.12 s |
| [`00b2dbe`](https://github.com/helius-labs/zolana-mobile-sdk/commit/00b2dbe2265c90ce984c6634e5227b65027730c2) | 2026-10-19 | 0.56 s | 0.65 s | 1.15 s | 2.16 s | 4.08 s | 6.37 s | 5.93 s |
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 0.56 s | 0.65 s | 1.17 s | 2.16 s | 4.24 s | 6.41 s | 6.11 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.55 s | 0.64 s | 1.15 s | 2.15 s | 4.13 s | 6.32 s | 5.93 s |

## Samsung Galaxy S25, Android 15, high tier

**Before:** [`7919e1b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/7919e1b763751e7f483b08f8b757bd0aade73625) of 2026-10-20. **After:** [`3ca9427`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3ca94276c3576e8bcf54e80e3fea0741b3ba5872) of 2026-10-21.

| by scenario | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| first run, including download | before | 2.10 s (download: 1.60 s, 8.8 MiB) | 2.81 s (download: 1.98 s, 15.6 MiB) | 4.80 s (download: 3.43 s, 28.9 MiB) | 9.90 s (download: 7.41 s, 55.9 MiB) | 18.49 s (download: 13.79 s, 109.5 MiB) | 26.54 s (download: 18.88 s, 171.6 MiB) | 32.29 s (download: 24.83 s, 165.0 MiB) |
| | after | 3.92 s (download: 3.42 s, 8.8 MiB) (+86 % **!**) | 8.97 s (download: 8.13 s, 15.6 MiB) (+219 % **!**) | 16.07 s (download: 14.63 s, 28.9 MiB) (+235 % **!**) | 23.70 s (download: 21.25 s, 55.9 MiB) (+139 % **!**) | 68.97 s (download: 64.33 s, 109.5 MiB) (+273 % **!**) | 135.81 s (download: 128.18 s, 171.6 MiB) (+412 % **!**) | 35.22 s (download: 27.66 s, 165.0 MiB) (+9 %) |
| warm | before | 0.20 s | 0.33 s | 0.56 s | 1.07 s | 2.07 s | 3.23 s | 3.19 s |
| | after | 0.19 s | 0.32 s | 0.55 s | 1.06 s | 2.10 s | 3.28 s | 3.14 s (-2 %) |
| first run after restart | before | 0.27 s | 0.33 s | 0.59 s | 1.17 s | 2.28 s | 3.57 s | 3.53 s |
| | after | 0.26 s | 0.33 s | 0.61 s | 1.11 s (-5 %) | 2.30 s | 3.75 s (+5 %) | 3.51 s |
| repeated after restart | before | 0.19 s | 0.33 s | 0.58 s | 1.14 s | 2.17 s | 3.59 s | 3.41 s |
| | after | 0.20 s | 0.33 s | 0.58 s | 1.15 s | 2.20 s | 3.61 s | 3.40 s |

| by stage | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| key load | before | 0.04 s | 0.01 s | 0.03 s | 0.04 s | 0.08 s | 0.12 s | 0.14 s |
| | after | 0.04 s | 0.02 s | 0.02 s | 0.04 s | 0.07 s | 0.11 s | 0.15 s |
| witness | before | 0.00 s | 0.01 s | 0.01 s | 0.03 s | 0.04 s | 0.06 s | 0.06 s |
| | after | 0.00 s | 0.01 s | 0.01 s | 0.03 s | 0.04 s | 0.07 s | 0.06 s |
| prove | before | 0.19 s | 0.31 s | 0.55 s | 1.08 s | 2.06 s | 3.30 s | 3.22 s |
| | after | 0.19 s | 0.32 s | 0.56 s | 1.05 s | 2.11 s | 3.37 s (+2 %) | 3.27 s |

### Proof, last 6 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`3ca9427`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3ca94276c3576e8bcf54e80e3fea0741b3ba5872) | 2026-10-21 | 0.19 s | 0.32 s | 0.56 s | 1.05 s | 2.11 s | 3.37 s | 3.27 s |
| [`7919e1b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/7919e1b763751e7f483b08f8b757bd0aade73625) | 2026-10-20 | 0.19 s | 0.31 s | 0.55 s | 1.08 s | 2.06 s | 3.30 s | 3.22 s |
| [`9bde901`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bde901088fa31b64651906cdbefcf755372c6f8) | 2026-10-19 | 0.19 s | 0.32 s | 0.57 s | 1.07 s | 2.27 s | 4.91 s | 3.45 s |
| [`00b2dbe`](https://github.com/helius-labs/zolana-mobile-sdk/commit/00b2dbe2265c90ce984c6634e5227b65027730c2) | 2026-10-19 | 0.19 s | 0.33 s | 0.56 s | 1.10 s | 2.13 s | 3.42 s | 3.28 s |
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 0.20 s | 0.32 s | 0.58 s | 1.12 s | 2.22 s | 3.47 s | 4.65 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.20 s | 0.33 s | 0.59 s | 1.31 s | 2.36 s | 3.89 s | 4.68 s |

### First run after a restart, last 6 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`3ca9427`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3ca94276c3576e8bcf54e80e3fea0741b3ba5872) | 2026-10-21 | 0.26 s | 0.33 s | 0.61 s | 1.11 s | 2.30 s | 3.75 s | 3.51 s |
| [`7919e1b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/7919e1b763751e7f483b08f8b757bd0aade73625) | 2026-10-20 | 0.27 s | 0.33 s | 0.59 s | 1.17 s | 2.28 s | 3.57 s | 3.53 s |
| [`9bde901`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bde901088fa31b64651906cdbefcf755372c6f8) | 2026-10-19 | 0.27 s | 0.34 s | 0.61 s | 1.17 s | 2.57 s | 6.88 s | 4.66 s |
| [`00b2dbe`](https://github.com/helius-labs/zolana-mobile-sdk/commit/00b2dbe2265c90ce984c6634e5227b65027730c2) | 2026-10-19 | 0.28 s | 0.34 s | 0.63 s | 1.20 s | 2.32 s | 3.77 s | 3.58 s |
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 0.29 s | 0.35 s | 0.63 s | 1.26 s | 2.50 s | 3.80 s | 6.47 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.29 s | 0.37 s | 0.67 s | 1.38 s | 2.73 s | 4.61 s | 6.40 s |

## iPhone 12, iOS 18.5, low tier

**Before:** [`7919e1b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/7919e1b763751e7f483b08f8b757bd0aade73625) of 2026-10-20. **After:** [`3ca9427`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3ca94276c3576e8bcf54e80e3fea0741b3ba5872) of 2026-10-21.

| by scenario | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| first run, including download | before | 1.47 s (download: 0.48 s, 8.8 MiB) | 2.05 s (download: 0.68 s, 15.6 MiB) | 3.77 s (download: 1.25 s, 28.9 MiB) | 7.29 s (download: 2.38 s, 55.9 MiB) | 14.43 s (download: 4.70 s, 109.5 MiB) | 23.43 s (download: 7.42 s, 171.6 MiB) | 23.06 s (download: 7.10 s, 165.0 MiB) |
| | after | 1.45 s (download: 0.49 s, 8.8 MiB) | 2.06 s (download: 0.69 s, 15.6 MiB) | 4.46 s (download: 1.98 s, 28.9 MiB) (+18 %) | 9.39 s (download: 4.65 s, 55.9 MiB) (+29 % **!**) | 17.45 s (download: 8.26 s, 109.5 MiB) (+21 % **!**) | 22.71 s (download: 7.39 s, 171.6 MiB) (-3 %) | 22.07 s (download: 7.13 s, 165.0 MiB) (-4 %) |
| warm | before | 0.32 s | 0.55 s | 0.98 s | 1.89 s | 3.65 s | 6.11 s | 6.35 s |
| | after | 0.31 s | 0.52 s | 0.95 s | 1.77 s (-6 %) | 3.32 s (-9 %) | 5.69 s (-7 %) | 5.64 s (-11 %) |
| first run after restart | before | 0.40 s | 0.71 s | 1.27 s | 2.35 s | 4.43 s | 7.12 s | 6.98 s |
| | after | 0.37 s | 0.64 s (-10 %) | 1.14 s (-10 %) | 2.11 s (-10 %) | 3.97 s (-10 %) | 6.46 s (-9 %) | 6.33 s (-9 %) |
| repeated after restart | before | 0.40 s | 0.67 s | 1.19 s | 2.22 s | 4.19 s | 6.80 s | 6.66 s |
| | after | 0.35 s | 0.59 s (-12 %) | 1.07 s (-10 %) | 1.99 s (-10 %) | 3.70 s (-12 %) | 6.26 s (-8 %) | 6.06 s (-9 %) |

| by stage | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| key load | before | 0.01 s | 0.02 s | 0.04 s | 0.08 s | 0.15 s | 0.22 s | 0.22 s |
| | after | 0.01 s | 0.03 s | 0.04 s | 0.09 s | 0.17 s | 0.23 s | 0.21 s |
| witness | before | 0.00 s | 0.00 s | 0.00 s | 0.01 s | 0.01 s | 0.01 s | 0.01 s |
| | after | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.01 s | 0.01 s | 0.01 s |
| prove | before | 0.36 s | 0.60 s | 1.09 s | 2.06 s | 3.92 s | 6.44 s | 6.49 s |
| | after | 0.34 s | 0.56 s | 1.00 s (-8 %) | 1.88 s (-9 %) | 3.52 s (-10 %) | 5.97 s (-7 %) | 5.85 s (-10 %) |

### Proof, last 6 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`3ca9427`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3ca94276c3576e8bcf54e80e3fea0741b3ba5872) | 2026-10-21 | 0.34 s | 0.56 s | 1.00 s | 1.88 s | 3.52 s | 5.97 s | 5.85 s |
| [`7919e1b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/7919e1b763751e7f483b08f8b757bd0aade73625) | 2026-10-20 | 0.36 s | 0.60 s | 1.09 s | 2.06 s | 3.92 s | 6.44 s | 6.49 s |
| [`9bde901`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bde901088fa31b64651906cdbefcf755372c6f8) | 2026-10-19 | 0.36 s | 0.59 s | 1.07 s | 1.99 s | 3.78 s | 6.28 s | 6.22 s |
| [`00b2dbe`](https://github.com/helius-labs/zolana-mobile-sdk/commit/00b2dbe2265c90ce984c6634e5227b65027730c2) | 2026-10-19 | 0.37 s | 0.62 s | 1.08 s | 2.03 s | 3.81 s | 6.34 s | 6.40 s |
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 0.37 s | 0.59 s | 1.03 s | 1.93 s | 3.69 s | 6.12 s | 6.17 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.37 s | 0.60 s | 1.08 s | 2.04 s | 3.96 s | 6.49 s | 6.49 s |

### First run after a restart, last 6 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`3ca9427`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3ca94276c3576e8bcf54e80e3fea0741b3ba5872) | 2026-10-21 | 0.37 s | 0.64 s | 1.14 s | 2.11 s | 3.97 s | 6.46 s | 6.33 s |
| [`7919e1b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/7919e1b763751e7f483b08f8b757bd0aade73625) | 2026-10-20 | 0.40 s | 0.71 s | 1.27 s | 2.35 s | 4.43 s | 7.12 s | 6.98 s |
| [`9bde901`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bde901088fa31b64651906cdbefcf755372c6f8) | 2026-10-19 | 0.43 s | 0.69 s | 1.27 s | 2.28 s | 4.27 s | 6.95 s | 6.77 s |
| [`00b2dbe`](https://github.com/helius-labs/zolana-mobile-sdk/commit/00b2dbe2265c90ce984c6634e5227b65027730c2) | 2026-10-19 | 0.41 s | 0.72 s | 1.27 s | 2.35 s | 4.33 s | 6.91 s | 6.70 s |
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 0.40 s | 0.72 s | 1.25 s | 2.27 s | 4.21 s | 6.90 s | 6.67 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.41 s | 0.71 s | 1.28 s | 2.33 s | 4.41 s | 7.47 s | 6.96 s |

## iPhone 15, iOS 18.5, mid tier

**Before:** [`7919e1b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/7919e1b763751e7f483b08f8b757bd0aade73625) of 2026-10-20. **After:** [`3ca9427`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3ca94276c3576e8bcf54e80e3fea0741b3ba5872) of 2026-10-21.

| by scenario | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| first run, including download | before | 1.41 s (download: 0.48 s, 8.8 MiB) | 1.94 s (download: 0.67 s, 15.6 MiB) | 3.59 s (download: 1.25 s, 28.9 MiB) | 6.83 s (download: 2.35 s, 55.9 MiB) | 12.52 s (download: 4.60 s, 109.5 MiB) | 21.15 s (download: 7.28 s, 171.6 MiB) | 21.01 s (download: 6.98 s, 165.0 MiB) |
| | after | 1.18 s (download: 0.47 s, 8.8 MiB) (-17 %) | 2.83 s (download: 1.68 s, 15.6 MiB) (+45 % **!**) | 5.36 s (download: 3.30 s, 28.9 MiB) (+49 % **!**) | 9.48 s (download: 5.57 s, 55.9 MiB) (+39 % **!**) | 17.49 s (download: 9.96 s, 109.5 MiB) (+40 % **!**) | 23.23 s (download: 10.75 s, 171.6 MiB) (+10 %) | 19.22 s (download: 6.88 s, 165.0 MiB) (-8 %) |
| warm | before | 0.32 s | 0.52 s | 0.96 s | 1.70 s | 2.90 s | 5.43 s | 5.29 s |
| | after | 0.28 s | 0.43 s (-18 %) | 0.78 s (-19 %) | 1.46 s (-14 %) | 2.77 s (-5 %) | 4.72 s (-13 %) | 4.70 s (-11 %) |
| first run after restart | before | 0.34 s | 0.59 s | 1.05 s | 1.96 s | 3.54 s | 5.72 s | 6.52 s |
| | after | 0.31 s | 0.55 s (-8 %) | 0.94 s (-10 %) | 1.73 s (-12 %) | 3.17 s (-10 %) | 5.24 s (-8 %) | 5.15 s (-21 % **!**) |
| repeated after restart | before | 0.34 s | 0.58 s | 1.02 s | 1.89 s | 3.48 s | 6.01 s | 6.23 s |
| | after | 0.30 s | 0.51 s (-12 %) | 0.90 s (-12 %) | 1.66 s (-12 %) | 3.05 s (-12 %) | 4.96 s (-17 %) | 4.91 s (-21 % **!**) |

| by stage | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| key load | before | 0.01 s | 0.02 s | 0.03 s | 0.06 s | 0.12 s | 0.18 s | 0.24 s |
| | after | 0.01 s | 0.02 s | 0.03 s | 0.05 s | 0.10 s | 0.16 s | 0.16 s (-35 % **!**) |
| witness | before | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.01 s | 0.01 s | 0.01 s |
| | after | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.01 s | 0.01 s | 0.01 s |
| prove | before | 0.34 s | 0.55 s | 0.98 s | 1.79 s | 3.15 s | 5.47 s | 5.75 s |
| | after | 0.29 s | 0.46 s (-15 %) | 0.84 s (-14 %) | 1.55 s (-13 %) | 2.90 s (-8 %) | 4.83 s (-12 %) | 4.79 s (-17 %) |

### Proof, last 6 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`3ca9427`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3ca94276c3576e8bcf54e80e3fea0741b3ba5872) | 2026-10-21 | 0.29 s | 0.46 s | 0.84 s | 1.55 s | 2.90 s | 4.83 s | 4.79 s |
| [`7919e1b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/7919e1b763751e7f483b08f8b757bd0aade73625) | 2026-10-20 | 0.34 s | 0.55 s | 0.98 s | 1.79 s | 3.15 s | 5.47 s | 5.75 s |
| [`9bde901`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bde901088fa31b64651906cdbefcf755372c6f8) | 2026-10-19 | 0.29 s | 0.48 s | 0.86 s | 1.59 s | 2.89 s | 4.72 s | 4.70 s |
| [`00b2dbe`](https://github.com/helius-labs/zolana-mobile-sdk/commit/00b2dbe2265c90ce984c6634e5227b65027730c2) | 2026-10-19 | 0.30 s | 0.49 s | 0.88 s | 1.60 s | 3.02 s | 4.89 s | 4.81 s |
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 0.36 s | 0.59 s | 1.04 s | 1.96 s | 3.70 s | 5.70 s | 5.59 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.29 s | 0.46 s | 0.81 s | 1.50 s | 2.80 s | 4.68 s | 4.65 s |

### First run after a restart, last 6 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`3ca9427`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3ca94276c3576e8bcf54e80e3fea0741b3ba5872) | 2026-10-21 | 0.31 s | 0.55 s | 0.94 s | 1.73 s | 3.17 s | 5.24 s | 5.15 s |
| [`7919e1b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/7919e1b763751e7f483b08f8b757bd0aade73625) | 2026-10-20 | 0.34 s | 0.59 s | 1.05 s | 1.96 s | 3.54 s | 5.72 s | 6.52 s |
| [`9bde901`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bde901088fa31b64651906cdbefcf755372c6f8) | 2026-10-19 | 0.31 s | 0.52 s | 0.92 s | 1.68 s | 3.15 s | 5.07 s | 4.97 s |
| [`00b2dbe`](https://github.com/helius-labs/zolana-mobile-sdk/commit/00b2dbe2265c90ce984c6634e5227b65027730c2) | 2026-10-19 | 0.32 s | 0.53 s | 0.95 s | 1.74 s | 3.22 s | 5.25 s | 5.11 s |
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 0.39 s | 0.63 s | 1.09 s | 1.98 s | 4.20 s | 6.37 s | 5.80 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.31 s | 0.51 s | 0.92 s | 1.67 s | 3.08 s | 5.07 s | 4.96 s |

## iPhone 17 Pro, iOS 26.3.1, high tier

**Before:** [`7919e1b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/7919e1b763751e7f483b08f8b757bd0aade73625) of 2026-10-20. **After:** [`3ca9427`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3ca94276c3576e8bcf54e80e3fea0741b3ba5872) of 2026-10-21.

| by scenario | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| first run, including download | before | 0.85 s (download: 0.32 s, 8.8 MiB) | 1.25 s (download: 0.35 s, 15.6 MiB) | 2.33 s (download: 0.69 s, 28.9 MiB) | 4.42 s (download: 1.32 s, 55.9 MiB) | 8.84 s (download: 2.80 s, 109.5 MiB) | 13.86 s (download: 3.98 s, 171.6 MiB) | 13.37 s (download: 3.80 s, 165.0 MiB) |
| | after | 1.01 s (download: 0.46 s, 8.8 MiB) (+18 %) | 1.53 s (download: 0.69 s, 15.6 MiB) (+22 % **!**) | 2.77 s (download: 1.22 s, 28.9 MiB) (+19 %) | 5.52 s (download: 2.54 s, 55.9 MiB) (+25 % **!**) | 10.30 s (download: 4.49 s, 109.5 MiB) (+17 %) | 16.78 s (download: 7.25 s, 171.6 MiB) (+21 % **!**) | 16.55 s (download: 7.29 s, 165.0 MiB) (+24 % **!**) |
| warm | before | 0.20 s | 0.34 s | 0.61 s | 1.17 s | 2.22 s | 3.60 s | 3.53 s |
| | after | 0.20 s | 0.34 s | 0.60 s | 1.14 s | 2.16 s (-3 %) | 3.55 s | 3.46 s (-2 %) |
| first run after restart | before | 0.23 s | 0.40 s | 0.69 s | 1.26 s | 2.33 s | 3.77 s | 3.76 s |
| | after | 0.23 s | 0.38 s | 0.67 s | 1.25 s | 2.30 s | 3.74 s | 3.64 s (-3 %) |
| repeated after restart | before | 0.23 s | 0.38 s | 0.66 s | 1.23 s | 2.27 s | 3.68 s | 3.86 s |
| | after | 0.22 s | 0.37 s | 0.65 s | 1.21 s | 2.24 s | 3.65 s | 3.58 s (-7 %) |

| by stage | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| key load | before | 0.00 s | 0.01 s | 0.02 s | 0.03 s | 0.06 s | 0.10 s | 0.09 s |
| | after | 0.00 s | 0.01 s | 0.01 s | 0.03 s | 0.06 s | 0.09 s | 0.09 s |
| witness | before | 0.00 s | 0.00 s | 0.00 s | 0.01 s | 0.01 s | 0.01 s | 0.01 s |
| | after | 0.00 s | 0.00 s | 0.00 s | 0.01 s | 0.01 s | 0.01 s | 0.01 s |
| prove | before | 0.21 s | 0.35 s | 0.63 s | 1.20 s | 2.23 s | 3.62 s | 3.58 s |
| | after | 0.22 s | 0.35 s | 0.62 s | 1.17 s | 2.19 s | 3.58 s | 3.49 s (-3 %) |

### Proof, last 6 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`3ca9427`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3ca94276c3576e8bcf54e80e3fea0741b3ba5872) | 2026-10-21 | 0.22 s | 0.35 s | 0.62 s | 1.17 s | 2.19 s | 3.58 s | 3.49 s |
| [`7919e1b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/7919e1b763751e7f483b08f8b757bd0aade73625) | 2026-10-20 | 0.21 s | 0.35 s | 0.63 s | 1.20 s | 2.23 s | 3.62 s | 3.58 s |
| [`9bde901`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bde901088fa31b64651906cdbefcf755372c6f8) | 2026-10-19 | 0.21 s | 0.35 s | 0.63 s | 1.18 s | 2.20 s | 3.56 s | 3.50 s |
| [`00b2dbe`](https://github.com/helius-labs/zolana-mobile-sdk/commit/00b2dbe2265c90ce984c6634e5227b65027730c2) | 2026-10-19 | 0.20 s | 0.34 s | 0.62 s | 1.14 s | 2.13 s | 3.46 s | 3.40 s |
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 0.21 s | 0.34 s | 0.62 s | 1.16 s | 2.12 s | 3.54 s | 3.47 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.20 s | 0.35 s | 0.63 s | 1.17 s | 2.17 s | 3.57 s | 3.46 s |

### First run after a restart, last 6 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`3ca9427`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3ca94276c3576e8bcf54e80e3fea0741b3ba5872) | 2026-10-21 | 0.23 s | 0.38 s | 0.67 s | 1.25 s | 2.30 s | 3.74 s | 3.64 s |
| [`7919e1b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/7919e1b763751e7f483b08f8b757bd0aade73625) | 2026-10-20 | 0.23 s | 0.40 s | 0.69 s | 1.26 s | 2.33 s | 3.77 s | 3.76 s |
| [`9bde901`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bde901088fa31b64651906cdbefcf755372c6f8) | 2026-10-19 | 0.23 s | 0.38 s | 0.68 s | 1.24 s | 2.30 s | 3.72 s | 3.62 s |
| [`00b2dbe`](https://github.com/helius-labs/zolana-mobile-sdk/commit/00b2dbe2265c90ce984c6634e5227b65027730c2) | 2026-10-19 | 0.21 s | 0.36 s | 0.64 s | 1.19 s | 2.24 s | 3.66 s | 3.64 s |
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 0.23 s | 0.38 s | 0.68 s | 1.23 s | 2.27 s | 3.70 s | 3.63 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.22 s | 0.38 s | 0.67 s | 1.23 s | 2.26 s | 3.68 s | 3.60 s |
