# Proof timing on main

Written by CI after every push to `main` (`scripts/proof-timing-history.py` on `main`), from release builds on the phones of `scripts/devicefarm/devices.json` in AWS Device Farm. Two runs of one commit differ by up to 5 % on the Galaxy A16 and A56 and up to 15 % on the S25 and the iPhones. The data is in `history/`, a line per commit.

## Samsung Galaxy A16, Android 14, low tier

**Before:** [`8c6af08`](https://github.com/helius-labs/zolana-mobile-sdk/commit/8c6af08e61ca4b0bc4b68ee3c75cbefa1b2667b6) of 2026-10-27. **After:** [`afda065`](https://github.com/helius-labs/zolana-mobile-sdk/commit/afda06572dc104ec6c1cb2e096958dbfce896fd0) of 2026-10-27.

| by scenario | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| first run, including download | before | 5.06 s (download: 1.94 s, 6.5 MiB) | 10.21 s (download: 4.94 s, 11.6 MiB) | 14.87 s (download: 5.77 s, 21.6 MiB) | 35.52 s (download: 18.84 s, 41.9 MiB) | 69.65 s (download: 37.50 s, 84.2 MiB) | 111.65 s (download: 61.06 s, 138.6 MiB) | 106.45 s (download: 57.70 s, 133.1 MiB) |
| | after | 5.85 s (download: 2.86 s, 6.5 MiB) (+15 %) | 10.22 s (download: 5.17 s, 11.6 MiB) | 15.38 s (download: 6.46 s, 21.6 MiB) (+3 %) | 35.74 s (download: 18.99 s, 41.9 MiB) (+1 %) | 69.92 s (download: 38.36 s, 84.2 MiB) | 112.07 s (download: 61.78 s, 138.6 MiB) | 108.35 s (download: 59.65 s, 133.1 MiB) (+2 %) |
| warm | before | 0.81 s | 1.45 s | 2.44 s | 4.54 s | 7.81 s | 12.92 s | 12.03 s |
| | after | 0.87 s (+7 %) | 1.48 s | 2.44 s | 4.53 s | 7.94 s (+2 %) | 12.47 s (-3 %) | 12.20 s (+1 %) |
| first run after restart | before | 0.98 s | 1.66 s | 2.79 s | 5.04 s | 8.56 s | 13.44 s | 13.10 s |
| | after | 0.96 s | 1.63 s | 2.83 s | 4.84 s (-4 %) | 8.66 s (+1 %) | 13.54 s (+1 %) | 12.81 s (-2 %) |
| repeated after restart | before | 0.94 s | 1.42 s | 2.44 s | 4.64 s | 8.40 s | 12.77 s | 12.47 s |
| | after | 0.82 s (-12 %) | 1.45 s | 2.44 s | 4.51 s (-3 %) | 7.92 s (-6 %) | 12.38 s (-3 %) | 11.80 s (-5 %) |

| by stage | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| key load | before | 0.20 s | 0.27 s | 0.29 s | 0.38 s | 0.57 s | 0.79 s | 0.83 s |
| | after | 0.20 s | 0.26 s | 0.29 s | 0.39 s | 0.53 s | 0.96 s (+22 % **!**) | 0.69 s (-17 %) |
| witness | before | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.01 s | 0.01 s |
| | after | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.01 s | 0.01 s |
| prove | before | 0.80 s | 1.40 s | 2.41 s | 4.57 s | 7.98 s | 12.68 s | 12.13 s |
| | after | 0.81 s | 1.42 s | 2.41 s | 4.50 s (-1 %) | 7.90 s (-1 %) | 12.41 s (-2 %) | 12.13 s |

### Proof, last 18 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`afda065`](https://github.com/helius-labs/zolana-mobile-sdk/commit/afda06572dc104ec6c1cb2e096958dbfce896fd0) | 2026-10-27 | 0.81 s | 1.42 s | 2.41 s | 4.50 s | 7.90 s | 12.41 s | 12.13 s |
| [`8c6af08`](https://github.com/helius-labs/zolana-mobile-sdk/commit/8c6af08e61ca4b0bc4b68ee3c75cbefa1b2667b6) | 2026-10-27 | 0.80 s | 1.40 s | 2.41 s | 4.57 s | 7.98 s | 12.68 s | 12.13 s |
| [`e30b328`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e30b328a5d18f58935ab297a695d7fc12ad8c53a) | 2026-10-26 | 0.87 s | 1.36 s | 2.49 s | 4.44 s | 8.09 s | 12.65 s | 12.58 s |
| [`418d376`](https://github.com/helius-labs/zolana-mobile-sdk/commit/418d37618e67cd4ba68d8b1f5ff50890b54039a8) | 2026-10-26 | 0.82 s | 1.39 s | 2.43 s | 4.54 s | 8.28 s | 12.53 s | 12.17 s |
| [`455be9e`](https://github.com/helius-labs/zolana-mobile-sdk/commit/455be9ee158cbba0fa0c63ca37e5de678fb8fe6d) | 2026-10-26 | 0.82 s | 1.37 s | 2.39 s | 4.45 s | 8.02 s | 12.67 s | 12.15 s |
| [`842ed6b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/842ed6b9dc8953fe88cf1c3bf79dfc3d8b1d525b) | 2026-10-26 | 0.82 s | 1.35 s | 2.43 s | 4.44 s | 7.80 s | 12.77 s | 12.37 s |
| [`3457ad0`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3457ad00eca9cacb4294e39032c6b339cf47e019) | 2026-10-25 | 0.82 s | 1.37 s | 2.45 s | 4.52 s | 8.08 s | 12.52 s | 12.33 s |
| [`c13ac6c`](https://github.com/helius-labs/zolana-mobile-sdk/commit/c13ac6cb4072d7a9bac4067fcfd1f7db1ec90f60) | 2026-10-23 | 0.81 s | 1.39 s | 2.44 s | 4.47 s | 7.93 s | 12.54 s | 12.13 s |
| [`964d396`](https://github.com/helius-labs/zolana-mobile-sdk/commit/964d396cf2443cecce962763d87f78655226c7cf) | 2026-10-23 | 0.82 s | 1.42 s | 2.45 s | 4.44 s | 8.18 s | 12.74 s | 12.50 s |
| [`3dd795c`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3dd795c879d76401167acd449f0c32a31d24e021) | 2026-10-23 | 0.84 s | 1.39 s | 2.45 s | 4.39 s | 7.88 s | 12.61 s | 12.27 s |
| [`030dc90`](https://github.com/helius-labs/zolana-mobile-sdk/commit/030dc902f09661f34b94700f3c8e5fc192051ca9) | 2026-10-23 | 0.81 s | 1.44 s | 2.42 s | 4.43 s | 8.08 s | 12.51 s | 12.10 s |
| [`a9931b5`](https://github.com/helius-labs/zolana-mobile-sdk/commit/a9931b55f4dd54246aa2939dbe21056bc6b6696e) | 2026-10-21 | 1.20 s | 2.04 s | 3.69 s | 7.03 s | 15.02 s | 25.15 s | 21.56 s |
| [`3ca9427`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3ca94276c3576e8bcf54e80e3fea0741b3ba5872) | 2026-10-21 | 1.19 s | 2.04 s | 3.62 s | 6.92 s | 14.86 s | 25.51 s | 22.29 s |
| [`7919e1b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/7919e1b763751e7f483b08f8b757bd0aade73625) | 2026-10-20 | 1.19 s | 2.01 s | 3.64 s | 7.07 s | 14.97 s | 25.17 s | 21.78 s |
| [`9bde901`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bde901088fa31b64651906cdbefcf755372c6f8) | 2026-10-19 | 1.20 s | 2.00 s | 3.65 s | 7.01 s | 14.83 s | 25.25 s | 22.09 s |
| [`00b2dbe`](https://github.com/helius-labs/zolana-mobile-sdk/commit/00b2dbe2265c90ce984c6634e5227b65027730c2) | 2026-10-19 | 1.19 s | 2.04 s | 3.65 s | 7.03 s | 14.89 s | 24.47 s | 22.16 s |
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 1.18 s | 2.02 s | 3.66 s | 7.07 s | 14.96 s | 24.88 s | 22.09 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 1.19 s | 2.01 s | 3.69 s | 6.99 s | 14.92 s | 23.81 s | 21.72 s |

### First run after a restart, last 18 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`afda065`](https://github.com/helius-labs/zolana-mobile-sdk/commit/afda06572dc104ec6c1cb2e096958dbfce896fd0) | 2026-10-27 | 0.96 s | 1.63 s | 2.83 s | 4.84 s | 8.66 s | 13.54 s | 12.81 s |
| [`8c6af08`](https://github.com/helius-labs/zolana-mobile-sdk/commit/8c6af08e61ca4b0bc4b68ee3c75cbefa1b2667b6) | 2026-10-27 | 0.98 s | 1.66 s | 2.79 s | 5.04 s | 8.56 s | 13.44 s | 13.10 s |
| [`e30b328`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e30b328a5d18f58935ab297a695d7fc12ad8c53a) | 2026-10-26 | 1.02 s | 1.66 s | 2.95 s | 4.99 s | 8.74 s | 13.57 s | 13.41 s |
| [`418d376`](https://github.com/helius-labs/zolana-mobile-sdk/commit/418d37618e67cd4ba68d8b1f5ff50890b54039a8) | 2026-10-26 | 1.05 s | 1.69 s | 2.75 s | 4.75 s | 9.16 s | 13.53 s | 12.73 s |
| [`455be9e`](https://github.com/helius-labs/zolana-mobile-sdk/commit/455be9ee158cbba0fa0c63ca37e5de678fb8fe6d) | 2026-10-26 | 0.99 s | 1.78 s | 2.70 s | 4.88 s | 8.66 s | 13.75 s | 12.99 s |
| [`842ed6b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/842ed6b9dc8953fe88cf1c3bf79dfc3d8b1d525b) | 2026-10-26 | 0.99 s | 1.66 s | 2.72 s | 5.04 s | 8.99 s | 13.81 s | 13.41 s |
| [`3457ad0`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3457ad00eca9cacb4294e39032c6b339cf47e019) | 2026-10-25 | 0.99 s | 1.68 s | 2.71 s | 4.77 s | 8.64 s | 13.24 s | 13.23 s |
| [`c13ac6c`](https://github.com/helius-labs/zolana-mobile-sdk/commit/c13ac6cb4072d7a9bac4067fcfd1f7db1ec90f60) | 2026-10-23 | 1.03 s | 1.64 s | 2.77 s | 5.01 s | 8.53 s | 13.61 s | 12.86 s |
| [`964d396`](https://github.com/helius-labs/zolana-mobile-sdk/commit/964d396cf2443cecce962763d87f78655226c7cf) | 2026-10-23 | 0.99 s | 1.73 s | 2.99 s | 4.86 s | 8.59 s | 13.48 s | 13.41 s |
| [`3dd795c`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3dd795c879d76401167acd449f0c32a31d24e021) | 2026-10-23 | 1.00 s | 1.72 s | 2.86 s | 4.77 s | 8.54 s | 13.81 s | 12.91 s |
| [`030dc90`](https://github.com/helius-labs/zolana-mobile-sdk/commit/030dc902f09661f34b94700f3c8e5fc192051ca9) | 2026-10-23 | 0.98 s | 1.76 s | 2.75 s | 4.87 s | 8.54 s | 13.70 s | 13.22 s |
| [`a9931b5`](https://github.com/helius-labs/zolana-mobile-sdk/commit/a9931b55f4dd54246aa2939dbe21056bc6b6696e) | 2026-10-21 | 1.22 s | 2.14 s | 3.87 s | 7.58 s | 15.71 s | 26.07 s | 22.60 s |
| [`3ca9427`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3ca94276c3576e8bcf54e80e3fea0741b3ba5872) | 2026-10-21 | 1.25 s | 2.17 s | 3.91 s | 7.15 s | 15.55 s | 26.42 s | 22.93 s |
| [`7919e1b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/7919e1b763751e7f483b08f8b757bd0aade73625) | 2026-10-20 | 1.19 s | 2.18 s | 3.85 s | 7.36 s | 15.73 s | 26.20 s | 23.13 s |
| [`9bde901`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bde901088fa31b64651906cdbefcf755372c6f8) | 2026-10-19 | 1.21 s | 2.14 s | 3.99 s | 7.31 s | 15.69 s | 26.41 s | 22.79 s |
| [`00b2dbe`](https://github.com/helius-labs/zolana-mobile-sdk/commit/00b2dbe2265c90ce984c6634e5227b65027730c2) | 2026-10-19 | 1.19 s | 2.23 s | 3.87 s | 7.47 s | 15.73 s | 26.60 s | 23.12 s |
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 1.15 s | 2.18 s | 3.80 s | 7.49 s | 15.63 s | 25.46 s | 23.25 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 1.27 s | 2.12 s | 3.93 s | 7.28 s | 15.57 s | 25.73 s | 22.95 s |

## Samsung Galaxy A56, Android 15, mid tier

**Before:** [`8c6af08`](https://github.com/helius-labs/zolana-mobile-sdk/commit/8c6af08e61ca4b0bc4b68ee3c75cbefa1b2667b6) of 2026-10-27. **After:** [`afda065`](https://github.com/helius-labs/zolana-mobile-sdk/commit/afda06572dc104ec6c1cb2e096958dbfce896fd0) of 2026-10-27.

| by scenario | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| first run, including download | before | 4.10 s (download: 3.05 s, 6.5 MiB) | 5.89 s (download: 4.13 s, 11.6 MiB) | 7.36 s (download: 4.28 s, 21.6 MiB) | 18.09 s (download: 12.28 s, 41.9 MiB) | 26.94 s (download: 15.56 s, 84.2 MiB) | 39.05 s (download: 21.09 s, 138.6 MiB) | 37.52 s (download: 20.08 s, 133.1 MiB) |
| | after | 2.17 s (download: 1.06 s, 6.5 MiB) (-47 % **!**) | 3.60 s (download: 1.84 s, 11.6 MiB) (-39 % **!**) | 6.58 s (download: 3.43 s, 21.6 MiB) (-11 %) | 12.03 s (download: 6.22 s, 41.9 MiB) (-33 % **!**) | 23.94 s (download: 12.59 s, 84.2 MiB) (-11 %) | 38.81 s (download: 20.83 s, 138.6 MiB) (-1 %) | 37.22 s (download: 19.79 s, 133.1 MiB) (-1 %) |
| warm | before | 0.26 s | 0.42 s | 0.76 s | 1.40 s | 2.72 s | 4.37 s | 4.28 s |
| | after | 0.28 s | 0.43 s | 0.84 s (+11 %) | 1.40 s | 2.72 s | 4.35 s | 4.23 s |
| first run after restart | before | 0.51 s | 0.58 s | 0.96 s | 1.63 s | 3.34 s | 4.78 s | 4.66 s |
| | after | 0.53 s | 0.61 s | 0.95 s | 1.65 s | 3.13 s (-6 %) | 4.93 s (+3 %) | 4.84 s (+4 %) |
| repeated after restart | before | 0.26 s | 0.43 s | 0.76 s | 1.40 s | 2.67 s | 4.37 s | 4.28 s |
| | after | 0.26 s | 0.43 s | 0.77 s | 1.38 s | 2.73 s (+2 %) | 4.38 s | 4.23 s (-1 %) |

| by stage | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| key load | before | 0.20 s | 0.12 s | 0.15 s | 0.19 s | 0.44 s | 0.36 s | 0.36 s |
| | after | 0.25 s (+29 % **!**) | 0.14 s | 0.15 s | 0.21 s | 0.30 s (-31 % **!**) | 0.42 s (+18 %) | 0.44 s (+24 % **!**) |
| witness | before | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s |
| | after | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s |
| prove | before | 0.26 s | 0.43 s | 0.76 s | 1.40 s | 2.75 s | 4.36 s | 4.28 s |
| | after | 0.26 s | 0.43 s | 0.81 s | 1.40 s | 2.75 s | 4.37 s | 4.26 s |

### Proof, last 18 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`afda065`](https://github.com/helius-labs/zolana-mobile-sdk/commit/afda06572dc104ec6c1cb2e096958dbfce896fd0) | 2026-10-27 | 0.26 s | 0.43 s | 0.81 s | 1.40 s | 2.75 s | 4.37 s | 4.26 s |
| [`8c6af08`](https://github.com/helius-labs/zolana-mobile-sdk/commit/8c6af08e61ca4b0bc4b68ee3c75cbefa1b2667b6) | 2026-10-27 | 0.26 s | 0.43 s | 0.76 s | 1.40 s | 2.75 s | 4.36 s | 4.28 s |
| [`e30b328`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e30b328a5d18f58935ab297a695d7fc12ad8c53a) | 2026-10-26 | 0.26 s | 0.43 s | 0.76 s | 1.41 s | 2.77 s | 4.40 s | 4.31 s |
| [`418d376`](https://github.com/helius-labs/zolana-mobile-sdk/commit/418d37618e67cd4ba68d8b1f5ff50890b54039a8) | 2026-10-26 | 0.25 s | 0.43 s | 0.76 s | 1.40 s | 2.76 s | 4.38 s | 4.24 s |
| [`455be9e`](https://github.com/helius-labs/zolana-mobile-sdk/commit/455be9ee158cbba0fa0c63ca37e5de678fb8fe6d) | 2026-10-26 | 0.26 s | 0.42 s | 0.75 s | 1.41 s | 2.74 s | 4.36 s | 4.27 s |
| [`842ed6b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/842ed6b9dc8953fe88cf1c3bf79dfc3d8b1d525b) | 2026-10-26 | 0.26 s | 0.44 s | 0.76 s | 1.41 s | 2.79 s | 4.37 s | 4.29 s |
| [`3457ad0`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3457ad00eca9cacb4294e39032c6b339cf47e019) | 2026-10-25 | 0.27 s | 0.43 s | 0.77 s | 1.40 s | 2.74 s | 4.36 s | 4.28 s |
| [`c13ac6c`](https://github.com/helius-labs/zolana-mobile-sdk/commit/c13ac6cb4072d7a9bac4067fcfd1f7db1ec90f60) | 2026-10-23 | 0.26 s | 0.43 s | 0.76 s | 1.40 s | 2.75 s | 4.41 s | 4.27 s |
| [`964d396`](https://github.com/helius-labs/zolana-mobile-sdk/commit/964d396cf2443cecce962763d87f78655226c7cf) | 2026-10-23 | 0.26 s | 0.43 s | 0.76 s | 1.39 s | 2.71 s | 4.41 s | 4.26 s |
| [`3dd795c`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3dd795c879d76401167acd449f0c32a31d24e021) | 2026-10-23 | 0.26 s | 0.43 s | 0.76 s | 1.40 s | 2.75 s | 4.40 s | 4.29 s |
| [`030dc90`](https://github.com/helius-labs/zolana-mobile-sdk/commit/030dc902f09661f34b94700f3c8e5fc192051ca9) | 2026-10-23 | 0.25 s | 0.43 s | 0.77 s | 1.40 s | 2.75 s | 4.39 s | 4.28 s |
| [`a9931b5`](https://github.com/helius-labs/zolana-mobile-sdk/commit/a9931b55f4dd54246aa2939dbe21056bc6b6696e) | 2026-10-21 | 0.34 s | 0.58 s | 1.06 s | 1.99 s | 3.81 s | 6.00 s | 5.77 s |
| [`3ca9427`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3ca94276c3576e8bcf54e80e3fea0741b3ba5872) | 2026-10-21 | 0.34 s | 0.58 s | 1.05 s | 1.98 s | 3.79 s | 6.00 s | 5.76 s |
| [`7919e1b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/7919e1b763751e7f483b08f8b757bd0aade73625) | 2026-10-20 | 0.34 s | 0.58 s | 1.06 s | 1.99 s | 3.77 s | 6.06 s | 5.80 s |
| [`9bde901`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bde901088fa31b64651906cdbefcf755372c6f8) | 2026-10-19 | 0.34 s | 0.58 s | 1.07 s | 2.00 s | 3.92 s | 6.03 s | 5.76 s |
| [`00b2dbe`](https://github.com/helius-labs/zolana-mobile-sdk/commit/00b2dbe2265c90ce984c6634e5227b65027730c2) | 2026-10-19 | 0.34 s | 0.58 s | 1.05 s | 2.00 s | 3.73 s | 6.05 s | 5.70 s |
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 0.34 s | 0.58 s | 1.06 s | 2.00 s | 3.90 s | 6.08 s | 5.80 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.35 s | 0.57 s | 1.06 s | 1.98 s | 3.87 s | 6.00 s | 5.78 s |

### First run after a restart, last 18 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`afda065`](https://github.com/helius-labs/zolana-mobile-sdk/commit/afda06572dc104ec6c1cb2e096958dbfce896fd0) | 2026-10-27 | 0.53 s | 0.61 s | 0.95 s | 1.65 s | 3.13 s | 4.93 s | 4.84 s |
| [`8c6af08`](https://github.com/helius-labs/zolana-mobile-sdk/commit/8c6af08e61ca4b0bc4b68ee3c75cbefa1b2667b6) | 2026-10-27 | 0.51 s | 0.58 s | 0.96 s | 1.63 s | 3.34 s | 4.78 s | 4.66 s |
| [`e30b328`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e30b328a5d18f58935ab297a695d7fc12ad8c53a) | 2026-10-26 | 0.55 s | 0.61 s | 0.99 s | 1.62 s | 3.03 s | 4.75 s | 4.68 s |
| [`418d376`](https://github.com/helius-labs/zolana-mobile-sdk/commit/418d37618e67cd4ba68d8b1f5ff50890b54039a8) | 2026-10-26 | 0.54 s | 0.59 s | 0.95 s | 1.66 s | 3.10 s | 4.76 s | 4.60 s |
| [`455be9e`](https://github.com/helius-labs/zolana-mobile-sdk/commit/455be9ee158cbba0fa0c63ca37e5de678fb8fe6d) | 2026-10-26 | 0.53 s | 0.55 s | 0.92 s | 1.61 s | 3.09 s | 4.73 s | 4.67 s |
| [`842ed6b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/842ed6b9dc8953fe88cf1c3bf79dfc3d8b1d525b) | 2026-10-26 | 0.52 s | 0.57 s | 0.94 s | 1.67 s | 3.24 s | 4.71 s | 4.63 s |
| [`3457ad0`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3457ad00eca9cacb4294e39032c6b339cf47e019) | 2026-10-25 | 0.52 s | 0.60 s | 1.01 s | 1.60 s | 3.10 s | 4.82 s | 4.68 s |
| [`c13ac6c`](https://github.com/helius-labs/zolana-mobile-sdk/commit/c13ac6cb4072d7a9bac4067fcfd1f7db1ec90f60) | 2026-10-23 | 0.53 s | 0.59 s | 0.94 s | 1.62 s | 3.19 s | 4.87 s | 4.79 s |
| [`964d396`](https://github.com/helius-labs/zolana-mobile-sdk/commit/964d396cf2443cecce962763d87f78655226c7cf) | 2026-10-23 | 0.51 s | 0.56 s | 0.95 s | 1.63 s | 3.08 s | 4.88 s | 4.76 s |
| [`3dd795c`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3dd795c879d76401167acd449f0c32a31d24e021) | 2026-10-23 | 0.52 s | 0.60 s | 0.95 s | 1.65 s | 3.17 s | 4.75 s | 4.71 s |
| [`030dc90`](https://github.com/helius-labs/zolana-mobile-sdk/commit/030dc902f09661f34b94700f3c8e5fc192051ca9) | 2026-10-23 | 0.55 s | 0.58 s | 0.93 s | 1.69 s | 3.17 s | 4.91 s | 4.86 s |
| [`a9931b5`](https://github.com/helius-labs/zolana-mobile-sdk/commit/a9931b55f4dd54246aa2939dbe21056bc6b6696e) | 2026-10-21 | 0.55 s | 0.68 s | 1.12 s | 2.11 s | 4.10 s | 6.24 s | 6.13 s |
| [`3ca9427`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3ca94276c3576e8bcf54e80e3fea0741b3ba5872) | 2026-10-21 | 0.56 s | 0.66 s | 1.15 s | 2.07 s | 4.11 s | 6.32 s | 6.14 s |
| [`7919e1b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/7919e1b763751e7f483b08f8b757bd0aade73625) | 2026-10-20 | 0.56 s | 0.65 s | 1.14 s | 2.13 s | 4.07 s | 6.37 s | 6.13 s |
| [`9bde901`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bde901088fa31b64651906cdbefcf755372c6f8) | 2026-10-19 | 0.57 s | 0.64 s | 1.17 s | 2.13 s | 4.15 s | 6.36 s | 6.12 s |
| [`00b2dbe`](https://github.com/helius-labs/zolana-mobile-sdk/commit/00b2dbe2265c90ce984c6634e5227b65027730c2) | 2026-10-19 | 0.56 s | 0.65 s | 1.15 s | 2.16 s | 4.08 s | 6.37 s | 5.93 s |
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 0.56 s | 0.65 s | 1.17 s | 2.16 s | 4.24 s | 6.41 s | 6.11 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.55 s | 0.64 s | 1.15 s | 2.15 s | 4.13 s | 6.32 s | 5.93 s |

## Samsung Galaxy S25, Android 15, high tier

**Before:** [`8c6af08`](https://github.com/helius-labs/zolana-mobile-sdk/commit/8c6af08e61ca4b0bc4b68ee3c75cbefa1b2667b6) of 2026-10-27. **After:** [`afda065`](https://github.com/helius-labs/zolana-mobile-sdk/commit/afda06572dc104ec6c1cb2e096958dbfce896fd0) of 2026-10-27.

| by scenario | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| first run, including download | before | 13.76 s (download: 1.40 s, 6.5 MiB) | 3.09 s (download: 1.89 s, 11.6 MiB) | 4.11 s (download: 2.32 s, 21.6 MiB) | 7.53 s (download: 4.61 s, 41.9 MiB) | 14.44 s (download: 8.97 s, 84.2 MiB) | 22.94 s (download: 14.33 s, 138.6 MiB) | 22.62 s (download: 14.15 s, 133.1 MiB) |
| | after | 13.99 s (download: 1.00 s, 6.5 MiB) (+2 %) | 2.63 s (download: 1.30 s, 11.6 MiB) (-15 %) | 4.28 s (download: 2.21 s, 21.6 MiB) (+4 %) | 7.65 s (download: 4.64 s, 41.9 MiB) (+2 %) | 14.15 s (download: 8.52 s, 84.2 MiB) (-2 %) | 26.72 s (download: 17.73 s, 138.6 MiB) (+16 %) | 22.94 s (download: 14.20 s, 133.1 MiB) (+1 %) |
| warm | before | 0.16 s | 0.20 s | 0.32 s | 0.54 s | 0.85 s | 1.53 s | 1.50 s |
| | after | 0.18 s | 0.23 s | 0.36 s | 0.54 s | 0.93 s (+9 %) | 1.63 s (+6 %) | 1.60 s (+7 %) |
| first run after restart | before | 0.50 s | 0.53 s | 0.66 s | 1.14 s | 1.84 s | 2.86 s | 2.78 s |
| | after | 0.55 s | 0.66 s (+25 % **!**) | 0.94 s (+41 % **!**) | 1.94 s (+70 % **!**) | 2.20 s (+19 %) | 4.34 s (+52 % **!**) | 4.30 s (+54 % **!**) |
| repeated after restart | before | 0.14 s | 0.20 s | 0.31 s | 0.66 s | 1.15 s | 1.85 s | 1.83 s |
| | after | 0.15 s | 0.19 s | 0.34 s | 0.65 s | 1.29 s (+13 %) | 2.49 s (+35 % **!**) | 2.42 s (+33 % **!**) |

| by stage | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| key load | before | 0.35 s | 0.33 s | 0.35 s | 0.48 s | 0.70 s | 0.99 s | 0.97 s |
| | after | 0.40 s | 0.46 s (+38 % **!**) | 0.51 s (+44 % **!**) | 0.76 s (+58 % **!**) | 1.05 s (+49 % **!**) | 1.83 s (+84 % **!**) | 1.86 s (+93 % **!**) |
| witness | before | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s |
| | after | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s |
| prove | before | 0.14 s | 0.19 s | 0.30 s | 0.59 s | 1.01 s | 1.68 s | 1.66 s |
| | after | 0.15 s | 0.20 s | 0.35 s | 0.59 s | 1.04 s | 2.05 s (+22 % **!**) | 2.00 s (+21 % **!**) |

### Proof, last 18 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`afda065`](https://github.com/helius-labs/zolana-mobile-sdk/commit/afda06572dc104ec6c1cb2e096958dbfce896fd0) | 2026-10-27 | 0.15 s | 0.20 s | 0.35 s | 0.59 s | 1.04 s | 2.05 s | 2.00 s |
| [`8c6af08`](https://github.com/helius-labs/zolana-mobile-sdk/commit/8c6af08e61ca4b0bc4b68ee3c75cbefa1b2667b6) | 2026-10-27 | 0.14 s | 0.19 s | 0.30 s | 0.59 s | 1.01 s | 1.68 s | 1.66 s |
| [`e30b328`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e30b328a5d18f58935ab297a695d7fc12ad8c53a) | 2026-10-26 | 0.14 s | 0.19 s | 0.32 s | 0.48 s | 0.88 s | 1.89 s | 1.82 s |
| [`418d376`](https://github.com/helius-labs/zolana-mobile-sdk/commit/418d37618e67cd4ba68d8b1f5ff50890b54039a8) | 2026-10-26 | 0.14 s | 0.19 s | 0.30 s | 0.48 s | 0.85 s | 1.39 s | 1.36 s |
| [`455be9e`](https://github.com/helius-labs/zolana-mobile-sdk/commit/455be9ee158cbba0fa0c63ca37e5de678fb8fe6d) | 2026-10-26 | 0.15 s | 0.19 s | 0.33 s | 0.52 s | 0.87 s | 1.78 s | 1.75 s |
| [`842ed6b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/842ed6b9dc8953fe88cf1c3bf79dfc3d8b1d525b) | 2026-10-26 | 0.15 s | 0.19 s | 0.30 s | 0.49 s | 0.86 s | 1.40 s | 1.37 s |
| [`3457ad0`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3457ad00eca9cacb4294e39032c6b339cf47e019) | 2026-10-25 | 0.15 s | 0.20 s | 0.32 s | 0.63 s | 1.13 s | 1.94 s | 2.04 s |
| [`c13ac6c`](https://github.com/helius-labs/zolana-mobile-sdk/commit/c13ac6cb4072d7a9bac4067fcfd1f7db1ec90f60) | 2026-10-23 | 0.15 s | 0.19 s | 0.30 s | 0.51 s | 0.89 s | 1.46 s | 1.42 s |
| [`964d396`](https://github.com/helius-labs/zolana-mobile-sdk/commit/964d396cf2443cecce962763d87f78655226c7cf) | 2026-10-23 | 0.14 s | 0.20 s | 0.31 s | 0.48 s | 0.85 s | 1.40 s | 1.36 s |
| [`3dd795c`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3dd795c879d76401167acd449f0c32a31d24e021) | 2026-10-23 | 0.14 s | 0.19 s | 0.30 s | 0.48 s | 0.85 s | 1.40 s | 1.36 s |
| [`030dc90`](https://github.com/helius-labs/zolana-mobile-sdk/commit/030dc902f09661f34b94700f3c8e5fc192051ca9) | 2026-10-23 | 0.14 s | 0.19 s | 0.30 s | 0.48 s | 0.86 s | 1.71 s | 2.12 s |
| [`a9931b5`](https://github.com/helius-labs/zolana-mobile-sdk/commit/a9931b55f4dd54246aa2939dbe21056bc6b6696e) | 2026-10-21 | 0.19 s | 0.33 s | 0.58 s | 1.12 s | 2.14 s | 3.46 s | 3.32 s |
| [`3ca9427`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3ca94276c3576e8bcf54e80e3fea0741b3ba5872) | 2026-10-21 | 0.19 s | 0.32 s | 0.56 s | 1.05 s | 2.11 s | 3.37 s | 3.27 s |
| [`7919e1b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/7919e1b763751e7f483b08f8b757bd0aade73625) | 2026-10-20 | 0.19 s | 0.31 s | 0.55 s | 1.08 s | 2.06 s | 3.30 s | 3.22 s |
| [`9bde901`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bde901088fa31b64651906cdbefcf755372c6f8) | 2026-10-19 | 0.19 s | 0.32 s | 0.57 s | 1.07 s | 2.27 s | 4.91 s | 3.45 s |
| [`00b2dbe`](https://github.com/helius-labs/zolana-mobile-sdk/commit/00b2dbe2265c90ce984c6634e5227b65027730c2) | 2026-10-19 | 0.19 s | 0.33 s | 0.56 s | 1.10 s | 2.13 s | 3.42 s | 3.28 s |
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 0.20 s | 0.32 s | 0.58 s | 1.12 s | 2.22 s | 3.47 s | 4.65 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.20 s | 0.33 s | 0.59 s | 1.31 s | 2.36 s | 3.89 s | 4.68 s |

### First run after a restart, last 18 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`afda065`](https://github.com/helius-labs/zolana-mobile-sdk/commit/afda06572dc104ec6c1cb2e096958dbfce896fd0) | 2026-10-27 | 0.55 s | 0.66 s | 0.94 s | 1.94 s | 2.20 s | 4.34 s | 4.30 s |
| [`8c6af08`](https://github.com/helius-labs/zolana-mobile-sdk/commit/8c6af08e61ca4b0bc4b68ee3c75cbefa1b2667b6) | 2026-10-27 | 0.50 s | 0.53 s | 0.66 s | 1.14 s | 1.84 s | 2.86 s | 2.78 s |
| [`e30b328`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e30b328a5d18f58935ab297a695d7fc12ad8c53a) | 2026-10-26 | 0.48 s | 0.53 s | 0.70 s | 1.08 s | 1.75 s | 3.48 s | 3.37 s |
| [`418d376`](https://github.com/helius-labs/zolana-mobile-sdk/commit/418d37618e67cd4ba68d8b1f5ff50890b54039a8) | 2026-10-26 | 0.47 s | 0.49 s | 0.65 s | 0.94 s | 1.51 s |  |  |
| [`455be9e`](https://github.com/helius-labs/zolana-mobile-sdk/commit/455be9ee158cbba0fa0c63ca37e5de678fb8fe6d) | 2026-10-26 | 0.57 s | 0.64 s | 0.80 s | 1.05 s | 1.73 s | 3.25 s | 3.19 s |
| [`842ed6b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/842ed6b9dc8953fe88cf1c3bf79dfc3d8b1d525b) | 2026-10-26 | 0.49 s | 0.50 s | 0.65 s | 1.00 s | 1.65 s |  |  |
| [`3457ad0`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3457ad00eca9cacb4294e39032c6b339cf47e019) | 2026-10-25 | 0.64 s | 0.67 s | 0.94 s | 1.45 s | 2.24 s | 3.69 s | 3.51 s |
| [`c13ac6c`](https://github.com/helius-labs/zolana-mobile-sdk/commit/c13ac6cb4072d7a9bac4067fcfd1f7db1ec90f60) | 2026-10-23 | 0.51 s | 0.54 s | 0.66 s | 1.00 s | 1.62 s | 2.48 s |  |
| [`964d396`](https://github.com/helius-labs/zolana-mobile-sdk/commit/964d396cf2443cecce962763d87f78655226c7cf) | 2026-10-23 | 0.51 s | 0.55 s | 0.72 s | 1.02 s | 1.68 s | 2.55 s |  |
| [`3dd795c`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3dd795c879d76401167acd449f0c32a31d24e021) | 2026-10-23 | 0.51 s | 0.54 s | 0.70 s | 0.96 s | 1.52 s | 2.33 s |  |
| [`030dc90`](https://github.com/helius-labs/zolana-mobile-sdk/commit/030dc902f09661f34b94700f3c8e5fc192051ca9) | 2026-10-23 | 0.51 s | 0.50 s | 0.65 s | 0.97 s | 1.59 s | 2.94 s | 4.60 s |
| [`a9931b5`](https://github.com/helius-labs/zolana-mobile-sdk/commit/a9931b55f4dd54246aa2939dbe21056bc6b6696e) | 2026-10-21 | 0.27 s | 0.35 s | 0.61 s | 1.21 s | 2.37 s | 3.87 s | 3.58 s |
| [`3ca9427`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3ca94276c3576e8bcf54e80e3fea0741b3ba5872) | 2026-10-21 | 0.26 s | 0.33 s | 0.61 s | 1.11 s | 2.30 s | 3.75 s | 3.51 s |
| [`7919e1b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/7919e1b763751e7f483b08f8b757bd0aade73625) | 2026-10-20 | 0.27 s | 0.33 s | 0.59 s | 1.17 s | 2.28 s | 3.57 s | 3.53 s |
| [`9bde901`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bde901088fa31b64651906cdbefcf755372c6f8) | 2026-10-19 | 0.27 s | 0.34 s | 0.61 s | 1.17 s | 2.57 s | 6.88 s | 4.66 s |
| [`00b2dbe`](https://github.com/helius-labs/zolana-mobile-sdk/commit/00b2dbe2265c90ce984c6634e5227b65027730c2) | 2026-10-19 | 0.28 s | 0.34 s | 0.63 s | 1.20 s | 2.32 s | 3.77 s | 3.58 s |
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 0.29 s | 0.35 s | 0.63 s | 1.26 s | 2.50 s | 3.80 s | 6.47 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.29 s | 0.37 s | 0.67 s | 1.38 s | 2.73 s | 4.61 s | 6.40 s |

## iPhone 12, iOS 18.5, low tier

**Before:** [`8c6af08`](https://github.com/helius-labs/zolana-mobile-sdk/commit/8c6af08e61ca4b0bc4b68ee3c75cbefa1b2667b6) of 2026-10-27. **After:** [`afda065`](https://github.com/helius-labs/zolana-mobile-sdk/commit/afda06572dc104ec6c1cb2e096958dbfce896fd0) of 2026-10-27.

| by scenario | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| first run, including download | before | 2.88 s (download: 0.41 s, 6.5 MiB) | 2.01 s (download: 0.52 s, 11.6 MiB) | 3.50 s (download: 0.93 s, 21.6 MiB) | 6.50 s (download: 1.79 s, 41.9 MiB) |  |  |  |
| | after | 2.84 s (download: 0.39 s, 6.5 MiB) | 1.96 s (download: 0.52 s, 11.6 MiB) (-3 %) | 3.46 s (download: 0.94 s, 21.6 MiB) | 6.43 s (download: 1.81 s, 41.9 MiB) (-1 %) | 13.10 s (download: 3.81 s, 84.2 MiB) | 21.00 s (download: 5.92 s, 138.6 MiB) | 20.36 s (download: 5.74 s, 133.1 MiB) |
| warm | before | 0.15 s | 0.23 s | 0.42 s | 0.68 s |  |  |  |
| | after | 0.14 s | 0.23 s | 0.41 s | 0.68 s | 1.24 s | 2.13 s | 2.16 s |
| first run after restart | before |  |  |  |  |  |  |  |
| | after | 0.29 s | 0.37 s | 0.67 s | 1.10 s | 1.92 s | 3.08 s | 3.05 s |
| repeated after restart | before |  |  |  |  |  |  |  |
| | after | 0.15 s | 0.25 s | 0.47 s | 0.82 s | 1.48 s | 2.46 s | 2.88 s |

| by stage | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| key load | before |  |  |  |  |  |  |  |
| | after | 0.10 s | 0.13 s | 0.20 s | 0.30 s | 0.49 s | 0.72 s | 0.68 s |
| witness | before | 0.00 s | 0.00 s | 0.00 s | 0.00 s |  |  |  |
| | after | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s |
| prove | before | 0.14 s | 0.23 s | 0.42 s | 0.67 s |  |  |  |
| | after | 0.15 s | 0.24 s | 0.46 s | 0.75 s (+11 %) | 1.33 s | 2.24 s | 2.26 s |

### Proof, last 18 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`afda065`](https://github.com/helius-labs/zolana-mobile-sdk/commit/afda06572dc104ec6c1cb2e096958dbfce896fd0) | 2026-10-27 | 0.15 s | 0.24 s | 0.46 s | 0.75 s | 1.33 s | 2.24 s | 2.26 s |
| [`8c6af08`](https://github.com/helius-labs/zolana-mobile-sdk/commit/8c6af08e61ca4b0bc4b68ee3c75cbefa1b2667b6) | 2026-10-27 | 0.14 s | 0.23 s | 0.42 s | 0.67 s |  |  |  |
| [`e30b328`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e30b328a5d18f58935ab297a695d7fc12ad8c53a) | 2026-10-26 | 0.15 s | 0.24 s | 0.46 s | 0.75 s | 1.34 s | 2.22 s | 2.18 s |
| [`418d376`](https://github.com/helius-labs/zolana-mobile-sdk/commit/418d37618e67cd4ba68d8b1f5ff50890b54039a8) | 2026-10-26 | 0.15 s | 0.23 s |  |  |  |  |  |
| [`455be9e`](https://github.com/helius-labs/zolana-mobile-sdk/commit/455be9ee158cbba0fa0c63ca37e5de678fb8fe6d) | 2026-10-26 | 0.14 s | 0.23 s | 0.43 s | 0.71 s | 1.25 s | 2.03 s | 2.02 s |
| [`842ed6b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/842ed6b9dc8953fe88cf1c3bf79dfc3d8b1d525b) | 2026-10-26 | 0.15 s | 0.23 s | 0.43 s | 0.72 s | 1.25 s | 2.04 s | 2.03 s |
| [`3457ad0`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3457ad00eca9cacb4294e39032c6b339cf47e019) | 2026-10-25 | 0.14 s | 0.23 s | 0.45 s | 0.74 s | 1.29 s | 2.17 s | 2.14 s |
| [`c13ac6c`](https://github.com/helius-labs/zolana-mobile-sdk/commit/c13ac6cb4072d7a9bac4067fcfd1f7db1ec90f60) | 2026-10-23 | 0.14 s | 0.23 s | 0.46 s | 0.73 s | 1.30 s | 2.20 s | 2.32 s |
| [`964d396`](https://github.com/helius-labs/zolana-mobile-sdk/commit/964d396cf2443cecce962763d87f78655226c7cf) | 2026-10-23 | 0.14 s | 0.24 s | 0.45 s | 0.73 s | 1.30 s | 2.18 s | 2.31 s |
| [`3dd795c`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3dd795c879d76401167acd449f0c32a31d24e021) | 2026-10-23 | 0.15 s | 0.24 s | 0.45 s | 0.75 s | 1.33 s | 2.21 s | 2.18 s |
| [`030dc90`](https://github.com/helius-labs/zolana-mobile-sdk/commit/030dc902f09661f34b94700f3c8e5fc192051ca9) | 2026-10-23 | 0.15 s | 0.23 s | 0.44 s | 0.72 s | 1.26 s | 2.08 s | 2.07 s |
| [`a9931b5`](https://github.com/helius-labs/zolana-mobile-sdk/commit/a9931b55f4dd54246aa2939dbe21056bc6b6696e) | 2026-10-21 | 0.37 s | 0.59 s | 1.08 s | 2.04 s | 3.91 s | 6.44 s | 6.43 s |
| [`3ca9427`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3ca94276c3576e8bcf54e80e3fea0741b3ba5872) | 2026-10-21 | 0.34 s | 0.56 s | 1.00 s | 1.88 s | 3.52 s | 5.97 s | 5.85 s |
| [`7919e1b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/7919e1b763751e7f483b08f8b757bd0aade73625) | 2026-10-20 | 0.36 s | 0.60 s | 1.09 s | 2.06 s | 3.92 s | 6.44 s | 6.49 s |
| [`9bde901`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bde901088fa31b64651906cdbefcf755372c6f8) | 2026-10-19 | 0.36 s | 0.59 s | 1.07 s | 1.99 s | 3.78 s | 6.28 s | 6.22 s |
| [`00b2dbe`](https://github.com/helius-labs/zolana-mobile-sdk/commit/00b2dbe2265c90ce984c6634e5227b65027730c2) | 2026-10-19 | 0.37 s | 0.62 s | 1.08 s | 2.03 s | 3.81 s | 6.34 s | 6.40 s |
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 0.37 s | 0.59 s | 1.03 s | 1.93 s | 3.69 s | 6.12 s | 6.17 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.37 s | 0.60 s | 1.08 s | 2.04 s | 3.96 s | 6.49 s | 6.49 s |

### First run after a restart, last 18 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`afda065`](https://github.com/helius-labs/zolana-mobile-sdk/commit/afda06572dc104ec6c1cb2e096958dbfce896fd0) | 2026-10-27 | 0.29 s | 0.37 s | 0.67 s | 1.10 s | 1.92 s | 3.08 s | 3.05 s |
| [`8c6af08`](https://github.com/helius-labs/zolana-mobile-sdk/commit/8c6af08e61ca4b0bc4b68ee3c75cbefa1b2667b6) | 2026-10-27 |  |  |  |  |  |  |  |
| [`e30b328`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e30b328a5d18f58935ab297a695d7fc12ad8c53a) | 2026-10-26 | 0.29 s | 0.36 s | 0.66 s | 1.08 s | 1.86 s | 3.05 s | 2.90 s |
| [`418d376`](https://github.com/helius-labs/zolana-mobile-sdk/commit/418d37618e67cd4ba68d8b1f5ff50890b54039a8) | 2026-10-26 |  |  |  |  |  |  |  |
| [`455be9e`](https://github.com/helius-labs/zolana-mobile-sdk/commit/455be9ee158cbba0fa0c63ca37e5de678fb8fe6d) | 2026-10-26 | 0.30 s | 0.36 s | 0.66 s | 1.06 s | 1.85 s | 2.99 s | 2.89 s |
| [`842ed6b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/842ed6b9dc8953fe88cf1c3bf79dfc3d8b1d525b) | 2026-10-26 | 0.31 s | 0.36 s | 0.67 s | 1.08 s | 1.90 s | 3.02 s | 2.90 s |
| [`3457ad0`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3457ad00eca9cacb4294e39032c6b339cf47e019) | 2026-10-25 | 0.29 s | 0.38 s | 0.70 s | 1.12 s | 1.95 s | 3.15 s | 3.05 s |
| [`c13ac6c`](https://github.com/helius-labs/zolana-mobile-sdk/commit/c13ac6cb4072d7a9bac4067fcfd1f7db1ec90f60) | 2026-10-23 | 0.28 s | 0.36 s | 0.67 s | 1.06 s | 1.88 s | 3.02 s | 3.35 s |
| [`964d396`](https://github.com/helius-labs/zolana-mobile-sdk/commit/964d396cf2443cecce962763d87f78655226c7cf) | 2026-10-23 | 0.29 s | 0.36 s | 0.67 s | 1.07 s | 1.87 s | 2.97 s | 3.30 s |
| [`3dd795c`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3dd795c879d76401167acd449f0c32a31d24e021) | 2026-10-23 | 0.29 s | 0.35 s | 0.67 s | 1.09 s | 1.89 s | 3.02 s | 2.90 s |
| [`030dc90`](https://github.com/helius-labs/zolana-mobile-sdk/commit/030dc902f09661f34b94700f3c8e5fc192051ca9) | 2026-10-23 | 0.30 s | 0.36 s | 0.66 s | 1.07 s | 1.90 s | 3.06 s | 2.96 s |
| [`a9931b5`](https://github.com/helius-labs/zolana-mobile-sdk/commit/a9931b55f4dd54246aa2939dbe21056bc6b6696e) | 2026-10-21 | 0.41 s | 0.69 s | 1.28 s | 2.33 s | 4.42 s | 7.09 s | 6.98 s |
| [`3ca9427`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3ca94276c3576e8bcf54e80e3fea0741b3ba5872) | 2026-10-21 | 0.37 s | 0.64 s | 1.14 s | 2.11 s | 3.97 s | 6.46 s | 6.33 s |
| [`7919e1b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/7919e1b763751e7f483b08f8b757bd0aade73625) | 2026-10-20 | 0.40 s | 0.71 s | 1.27 s | 2.35 s | 4.43 s | 7.12 s | 6.98 s |
| [`9bde901`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bde901088fa31b64651906cdbefcf755372c6f8) | 2026-10-19 | 0.43 s | 0.69 s | 1.27 s | 2.28 s | 4.27 s | 6.95 s | 6.77 s |
| [`00b2dbe`](https://github.com/helius-labs/zolana-mobile-sdk/commit/00b2dbe2265c90ce984c6634e5227b65027730c2) | 2026-10-19 | 0.41 s | 0.72 s | 1.27 s | 2.35 s | 4.33 s | 6.91 s | 6.70 s |
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 0.40 s | 0.72 s | 1.25 s | 2.27 s | 4.21 s | 6.90 s | 6.67 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.41 s | 0.71 s | 1.28 s | 2.33 s | 4.41 s | 7.47 s | 6.96 s |

## iPhone 15, iOS 18.5, mid tier

**Before:** [`8c6af08`](https://github.com/helius-labs/zolana-mobile-sdk/commit/8c6af08e61ca4b0bc4b68ee3c75cbefa1b2667b6) of 2026-10-27. **After:** [`afda065`](https://github.com/helius-labs/zolana-mobile-sdk/commit/afda06572dc104ec6c1cb2e096958dbfce896fd0) of 2026-10-27.

| by scenario | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| first run, including download | before | 2.37 s (download: 0.36 s, 6.5 MiB) | 1.66 s (download: 0.50 s, 11.6 MiB) | 2.95 s (download: 0.91 s, 21.6 MiB) | 5.50 s (download: 1.77 s, 41.9 MiB) | 10.85 s (download: 3.50 s, 84.2 MiB) | 17.86 s (download: 5.85 s, 138.6 MiB) | 17.05 s (download: 5.56 s, 133.1 MiB) |
| | after | 2.34 s (download: 0.37 s, 6.5 MiB) | 1.87 s (download: 0.51 s, 11.6 MiB) (+12 %) | 3.36 s (download: 0.94 s, 21.6 MiB) (+14 %) | 8.40 s (download: 3.64 s, 41.9 MiB) (+53 % **!**) | 12.87 s (download: 3.67 s, 84.2 MiB) (+19 %) | 20.04 s (download: 6.07 s, 138.6 MiB) (+12 %) | 19.44 s (download: 5.73 s, 133.1 MiB) (+14 %) |
| warm | before | 0.11 s | 0.18 s | 0.32 s | 0.50 s | 0.88 s | 1.45 s | 1.42 s |
| | after | 0.11 s | 0.18 s | 0.33 s | 0.61 s (+21 % **!**) | 1.02 s (+16 %) | 1.51 s (+4 %) | 1.48 s (+4 %) |
| first run after restart | before | 0.22 s | 0.27 s | 0.45 s | 0.76 s | 1.32 s | 2.17 s | 2.04 s |
| | after | 0.25 s | 0.36 s (+32 % **!**) | 0.58 s (+27 % **!**) | 0.98 s (+29 % **!**) | 1.68 s (+26 % **!**) | 2.70 s (+24 % **!**) | 2.54 s (+24 % **!**) |
| repeated after restart | before | 0.12 s | 0.18 s | 0.32 s | 0.51 s | 0.91 s | 1.52 s | 1.53 s |
| | after | 0.15 s | 0.25 s (+39 % **!**) | 0.43 s (+35 % **!**) | 0.78 s (+51 % **!**) | 1.38 s (+51 % **!**) | 2.21 s (+45 % **!**) | 2.05 s (+34 % **!**) |

| by stage | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| key load | before | 0.09 s | 0.10 s | 0.14 s | 0.24 s | 0.43 s | 0.66 s | 0.56 s |
| | after | 0.10 s | 0.13 s | 0.17 s | 0.28 s | 0.48 s (+12 %) | 0.74 s (+13 %) | 0.71 s (+26 % **!**) |
| witness | before | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s |
| | after | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s |
| prove | before | 0.11 s | 0.18 s | 0.32 s | 0.51 s | 0.89 s | 1.49 s | 1.45 s |
| | after | 0.15 s | 0.21 s | 0.37 s (+18 %) | 0.66 s (+30 % **!**) | 1.10 s (+24 % **!**) | 1.73 s (+16 %) | 1.65 s (+14 %) |

### Proof, last 18 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`afda065`](https://github.com/helius-labs/zolana-mobile-sdk/commit/afda06572dc104ec6c1cb2e096958dbfce896fd0) | 2026-10-27 | 0.15 s | 0.21 s | 0.37 s | 0.66 s | 1.10 s | 1.73 s | 1.65 s |
| [`8c6af08`](https://github.com/helius-labs/zolana-mobile-sdk/commit/8c6af08e61ca4b0bc4b68ee3c75cbefa1b2667b6) | 2026-10-27 | 0.11 s | 0.18 s | 0.32 s | 0.51 s | 0.89 s | 1.49 s | 1.45 s |
| [`e30b328`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e30b328a5d18f58935ab297a695d7fc12ad8c53a) | 2026-10-26 | 0.12 s | 0.18 s | 0.32 s | 0.51 s | 0.90 s | 1.52 s | 1.48 s |
| [`418d376`](https://github.com/helius-labs/zolana-mobile-sdk/commit/418d37618e67cd4ba68d8b1f5ff50890b54039a8) | 2026-10-26 | 0.11 s | 0.18 s | 0.32 s | 0.51 s | 0.90 s | 1.50 s | 1.48 s |
| [`455be9e`](https://github.com/helius-labs/zolana-mobile-sdk/commit/455be9ee158cbba0fa0c63ca37e5de678fb8fe6d) | 2026-10-26 | 0.13 s | 0.19 s | 0.34 s | 0.56 s | 1.00 s | 1.63 s | 1.58 s |
| [`842ed6b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/842ed6b9dc8953fe88cf1c3bf79dfc3d8b1d525b) | 2026-10-26 | 0.15 s | 0.25 s | 0.43 s | 0.70 s | 1.13 s | 1.69 s | 1.82 s |
| [`3457ad0`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3457ad00eca9cacb4294e39032c6b339cf47e019) | 2026-10-25 | 0.11 s | 0.17 s | 0.31 s | 0.50 s | 0.88 s | 1.49 s | 1.45 s |
| [`c13ac6c`](https://github.com/helius-labs/zolana-mobile-sdk/commit/c13ac6cb4072d7a9bac4067fcfd1f7db1ec90f60) | 2026-10-23 | 0.13 s | 0.18 s | 0.32 s | 0.52 s | 0.92 s | 1.45 s | 1.44 s |
| [`964d396`](https://github.com/helius-labs/zolana-mobile-sdk/commit/964d396cf2443cecce962763d87f78655226c7cf) | 2026-10-23 | 0.15 s | 0.21 s | 0.38 s | 0.65 s | 1.15 s | 1.79 s | 1.70 s |
| [`3dd795c`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3dd795c879d76401167acd449f0c32a31d24e021) | 2026-10-23 | 0.12 s | 0.18 s | 0.32 s | 0.52 s | 0.94 s | 1.52 s | 1.50 s |
| [`030dc90`](https://github.com/helius-labs/zolana-mobile-sdk/commit/030dc902f09661f34b94700f3c8e5fc192051ca9) | 2026-10-23 | 0.11 s | 0.17 s | 0.32 s | 0.50 s | 0.88 s | 1.47 s | 1.44 s |
| [`a9931b5`](https://github.com/helius-labs/zolana-mobile-sdk/commit/a9931b55f4dd54246aa2939dbe21056bc6b6696e) | 2026-10-21 | 0.29 s | 0.48 s | 0.86 s | 1.63 s | 2.96 s | 4.83 s | 4.89 s |
| [`3ca9427`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3ca94276c3576e8bcf54e80e3fea0741b3ba5872) | 2026-10-21 | 0.29 s | 0.46 s | 0.84 s | 1.55 s | 2.90 s | 4.83 s | 4.79 s |
| [`7919e1b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/7919e1b763751e7f483b08f8b757bd0aade73625) | 2026-10-20 | 0.34 s | 0.55 s | 0.98 s | 1.79 s | 3.15 s | 5.47 s | 5.75 s |
| [`9bde901`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bde901088fa31b64651906cdbefcf755372c6f8) | 2026-10-19 | 0.29 s | 0.48 s | 0.86 s | 1.59 s | 2.89 s | 4.72 s | 4.70 s |
| [`00b2dbe`](https://github.com/helius-labs/zolana-mobile-sdk/commit/00b2dbe2265c90ce984c6634e5227b65027730c2) | 2026-10-19 | 0.30 s | 0.49 s | 0.88 s | 1.60 s | 3.02 s | 4.89 s | 4.81 s |
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 0.36 s | 0.59 s | 1.04 s | 1.96 s | 3.70 s | 5.70 s | 5.59 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.29 s | 0.46 s | 0.81 s | 1.50 s | 2.80 s | 4.68 s | 4.65 s |

### First run after a restart, last 18 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`afda065`](https://github.com/helius-labs/zolana-mobile-sdk/commit/afda06572dc104ec6c1cb2e096958dbfce896fd0) | 2026-10-27 | 0.25 s | 0.36 s | 0.58 s | 0.98 s | 1.68 s | 2.70 s | 2.54 s |
| [`8c6af08`](https://github.com/helius-labs/zolana-mobile-sdk/commit/8c6af08e61ca4b0bc4b68ee3c75cbefa1b2667b6) | 2026-10-27 | 0.22 s | 0.27 s | 0.45 s | 0.76 s | 1.32 s | 2.17 s | 2.04 s |
| [`e30b328`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e30b328a5d18f58935ab297a695d7fc12ad8c53a) | 2026-10-26 | 0.22 s | 0.29 s | 0.47 s | 0.77 s | 1.40 s | 2.27 s | 2.07 s |
| [`418d376`](https://github.com/helius-labs/zolana-mobile-sdk/commit/418d37618e67cd4ba68d8b1f5ff50890b54039a8) | 2026-10-26 | 0.22 s | 0.28 s | 0.46 s | 0.76 s | 1.34 s | 1.99 s | 1.95 s |
| [`455be9e`](https://github.com/helius-labs/zolana-mobile-sdk/commit/455be9ee158cbba0fa0c63ca37e5de678fb8fe6d) | 2026-10-26 | 0.21 s | 0.30 s | 0.51 s | 0.86 s | 1.57 s | 2.37 s | 2.28 s |
| [`842ed6b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/842ed6b9dc8953fe88cf1c3bf79dfc3d8b1d525b) | 2026-10-26 | 0.32 s | 0.55 s | 0.85 s | 1.15 s | 1.91 s | 2.81 s | 2.97 s |
| [`3457ad0`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3457ad00eca9cacb4294e39032c6b339cf47e019) | 2026-10-25 | 0.21 s | 0.28 s | 0.46 s | 0.77 s | 1.36 s | 2.10 s | 1.93 s |
| [`c13ac6c`](https://github.com/helius-labs/zolana-mobile-sdk/commit/c13ac6cb4072d7a9bac4067fcfd1f7db1ec90f60) | 2026-10-23 | 0.22 s | 0.29 s | 0.48 s | 0.80 s | 1.41 s | 2.18 s | 1.94 s |
| [`964d396`](https://github.com/helius-labs/zolana-mobile-sdk/commit/964d396cf2443cecce962763d87f78655226c7cf) | 2026-10-23 | 0.24 s | 0.38 s | 0.59 s | 1.00 s | 1.69 s | 2.75 s | 2.63 s |
| [`3dd795c`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3dd795c879d76401167acd449f0c32a31d24e021) | 2026-10-23 | 0.22 s | 0.29 s | 0.48 s | 0.79 s | 1.39 s | 2.27 s | 2.04 s |
| [`030dc90`](https://github.com/helius-labs/zolana-mobile-sdk/commit/030dc902f09661f34b94700f3c8e5fc192051ca9) | 2026-10-23 | 0.22 s | 0.27 s | 0.46 s | 0.74 s | 1.32 s | 2.16 s | 2.01 s |
| [`a9931b5`](https://github.com/helius-labs/zolana-mobile-sdk/commit/a9931b55f4dd54246aa2939dbe21056bc6b6696e) | 2026-10-21 | 0.32 s | 0.53 s | 0.95 s | 1.72 s | 3.18 s | 5.21 s | 5.26 s |
| [`3ca9427`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3ca94276c3576e8bcf54e80e3fea0741b3ba5872) | 2026-10-21 | 0.31 s | 0.55 s | 0.94 s | 1.73 s | 3.17 s | 5.24 s | 5.15 s |
| [`7919e1b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/7919e1b763751e7f483b08f8b757bd0aade73625) | 2026-10-20 | 0.34 s | 0.59 s | 1.05 s | 1.96 s | 3.54 s | 5.72 s | 6.52 s |
| [`9bde901`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bde901088fa31b64651906cdbefcf755372c6f8) | 2026-10-19 | 0.31 s | 0.52 s | 0.92 s | 1.68 s | 3.15 s | 5.07 s | 4.97 s |
| [`00b2dbe`](https://github.com/helius-labs/zolana-mobile-sdk/commit/00b2dbe2265c90ce984c6634e5227b65027730c2) | 2026-10-19 | 0.32 s | 0.53 s | 0.95 s | 1.74 s | 3.22 s | 5.25 s | 5.11 s |
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 0.39 s | 0.63 s | 1.09 s | 1.98 s | 4.20 s | 6.37 s | 5.80 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.31 s | 0.51 s | 0.92 s | 1.67 s | 3.08 s | 5.07 s | 4.96 s |

## iPhone 17 Pro, iOS 26.3.1, high tier

**Before:** [`8c6af08`](https://github.com/helius-labs/zolana-mobile-sdk/commit/8c6af08e61ca4b0bc4b68ee3c75cbefa1b2667b6) of 2026-10-27. **After:** [`afda065`](https://github.com/helius-labs/zolana-mobile-sdk/commit/afda06572dc104ec6c1cb2e096958dbfce896fd0) of 2026-10-27.

| by scenario | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| first run, including download | before | 8.69 s (download: 0.90 s, 6.5 MiB) | 2.01 s (download: 1.15 s, 11.6 MiB) | 3.79 s (download: 2.27 s, 21.6 MiB) | 6.91 s (download: 4.07 s, 41.9 MiB) | 13.41 s (download: 7.83 s, 84.2 MiB) | 23.11 s (download: 14.52 s, 138.6 MiB) | 22.73 s (download: 14.45 s, 133.1 MiB) |
| | after | 8.26 s (download: 0.27 s, 6.5 MiB) (-5 %) | 1.24 s (download: 0.33 s, 11.6 MiB) (-38 % **!**) | 2.16 s (download: 0.56 s, 21.6 MiB) (-43 % **!**) | 4.16 s (download: 1.08 s, 41.9 MiB) (-40 % **!**) | 7.95 s (download: 2.01 s, 84.2 MiB) (-41 % **!**) | 12.52 s (download: 3.30 s, 138.6 MiB) (-46 % **!**) | 12.04 s (download: 3.00 s, 133.1 MiB) (-47 % **!**) |
| warm | before | 0.08 s | 0.11 s | 0.19 s | 0.33 s | 0.58 s | 0.75 s | 0.75 s |
| | after | 0.08 s | 0.12 s | 0.20 s | 0.34 s | 0.59 s | 0.82 s (+8 %) | 0.80 s (+7 %) |
| first run after restart | before | 0.16 s | 0.19 s | 0.30 s | 0.50 s | 0.78 s | 1.08 s | 1.09 s |
| | after | 0.16 s | 0.21 s | 0.33 s | 0.51 s | 0.83 s (+6 %) | 1.17 s (+9 %) | 1.18 s (+7 %) |
| repeated after restart | before | 0.09 s | 0.12 s | 0.21 s | 0.35 s | 0.58 s | 0.82 s | 0.81 s |
| | after | 0.10 s | 0.13 s | 0.23 s | 0.37 s | 0.61 s | 0.89 s (+8 %) | 0.86 s (+7 %) |

| by stage | | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| key load | before | 0.07 s | 0.07 s | 0.09 s | 0.14 s | 0.20 s | 0.28 s | 0.29 s |
| | after | 0.07 s | 0.08 s | 0.11 s | 0.14 s | 0.21 s | 0.29 s | 0.30 s |
| witness | before | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s |
| | after | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s | 0.00 s |
| prove | before | 0.08 s | 0.11 s | 0.20 s | 0.34 s | 0.57 s | 0.78 s | 0.77 s |
| | after | 0.09 s | 0.12 s | 0.21 s | 0.36 s | 0.60 s | 0.85 s (+9 %) | 0.83 s (+7 %) |

### Proof, last 18 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`afda065`](https://github.com/helius-labs/zolana-mobile-sdk/commit/afda06572dc104ec6c1cb2e096958dbfce896fd0) | 2026-10-27 | 0.09 s | 0.12 s | 0.21 s | 0.36 s | 0.60 s | 0.85 s | 0.83 s |
| [`8c6af08`](https://github.com/helius-labs/zolana-mobile-sdk/commit/8c6af08e61ca4b0bc4b68ee3c75cbefa1b2667b6) | 2026-10-27 | 0.08 s | 0.11 s | 0.20 s | 0.34 s | 0.57 s | 0.78 s | 0.77 s |
| [`e30b328`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e30b328a5d18f58935ab297a695d7fc12ad8c53a) | 2026-10-26 | 0.08 s | 0.11 s | 0.19 s | 0.33 s | 0.55 s | 0.78 s | 0.76 s |
| [`418d376`](https://github.com/helius-labs/zolana-mobile-sdk/commit/418d37618e67cd4ba68d8b1f5ff50890b54039a8) | 2026-10-26 | 0.09 s | 0.13 s | 0.23 s | 0.38 s | 0.64 s | 0.92 s | 0.89 s |
| [`455be9e`](https://github.com/helius-labs/zolana-mobile-sdk/commit/455be9ee158cbba0fa0c63ca37e5de678fb8fe6d) | 2026-10-26 | 0.09 s | 0.12 s | 0.21 s | 0.35 s | 0.57 s | 0.83 s | 0.81 s |
| [`842ed6b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/842ed6b9dc8953fe88cf1c3bf79dfc3d8b1d525b) | 2026-10-26 | 0.08 s | 0.11 s | 0.20 s | 0.34 s | 0.55 s | 0.78 s | 0.77 s |
| [`3457ad0`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3457ad00eca9cacb4294e39032c6b339cf47e019) | 2026-10-25 | 0.09 s | 0.12 s | 0.21 s | 0.35 s | 0.58 s | 0.84 s | 0.80 s |
| [`c13ac6c`](https://github.com/helius-labs/zolana-mobile-sdk/commit/c13ac6cb4072d7a9bac4067fcfd1f7db1ec90f60) | 2026-10-23 | 0.09 s | 0.12 s | 0.24 s | 0.37 s | 0.60 s | 0.86 s | 0.85 s |
| [`964d396`](https://github.com/helius-labs/zolana-mobile-sdk/commit/964d396cf2443cecce962763d87f78655226c7cf) | 2026-10-23 | 0.09 s | 0.12 s | 0.20 s | 0.34 s | 0.56 s | 0.81 s | 0.79 s |
| [`3dd795c`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3dd795c879d76401167acd449f0c32a31d24e021) | 2026-10-23 | 0.08 s | 0.11 s | 0.20 s | 0.34 s | 0.55 s | 0.79 s | 0.78 s |
| [`030dc90`](https://github.com/helius-labs/zolana-mobile-sdk/commit/030dc902f09661f34b94700f3c8e5fc192051ca9) | 2026-10-23 | 0.09 s | 0.12 s | 0.20 s | 0.34 s | 0.58 s | 0.81 s | 0.78 s |
| [`a9931b5`](https://github.com/helius-labs/zolana-mobile-sdk/commit/a9931b55f4dd54246aa2939dbe21056bc6b6696e) | 2026-10-21 | 0.21 s | 0.35 s | 0.62 s | 1.16 s | 2.16 s | 3.54 s | 3.43 s |
| [`3ca9427`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3ca94276c3576e8bcf54e80e3fea0741b3ba5872) | 2026-10-21 | 0.22 s | 0.35 s | 0.62 s | 1.17 s | 2.19 s | 3.58 s | 3.49 s |
| [`7919e1b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/7919e1b763751e7f483b08f8b757bd0aade73625) | 2026-10-20 | 0.21 s | 0.35 s | 0.63 s | 1.20 s | 2.23 s | 3.62 s | 3.58 s |
| [`9bde901`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bde901088fa31b64651906cdbefcf755372c6f8) | 2026-10-19 | 0.21 s | 0.35 s | 0.63 s | 1.18 s | 2.20 s | 3.56 s | 3.50 s |
| [`00b2dbe`](https://github.com/helius-labs/zolana-mobile-sdk/commit/00b2dbe2265c90ce984c6634e5227b65027730c2) | 2026-10-19 | 0.20 s | 0.34 s | 0.62 s | 1.14 s | 2.13 s | 3.46 s | 3.40 s |
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 0.21 s | 0.34 s | 0.62 s | 1.16 s | 2.12 s | 3.54 s | 3.47 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.20 s | 0.35 s | 0.63 s | 1.17 s | 2.17 s | 3.57 s | 3.46 s |

### First run after a restart, last 18 commits

| commit | date | 1 → 2 | 2 → 2 | 4 → 2 | 8 → 2 | 16 → 2 | 24 → 2 | merge 24 → 1 |
|---|---|---|---|---|---|---|---|---|
| [`afda065`](https://github.com/helius-labs/zolana-mobile-sdk/commit/afda06572dc104ec6c1cb2e096958dbfce896fd0) | 2026-10-27 | 0.16 s | 0.21 s | 0.33 s | 0.51 s | 0.83 s | 1.17 s | 1.18 s |
| [`8c6af08`](https://github.com/helius-labs/zolana-mobile-sdk/commit/8c6af08e61ca4b0bc4b68ee3c75cbefa1b2667b6) | 2026-10-27 | 0.16 s | 0.19 s | 0.30 s | 0.50 s | 0.78 s | 1.08 s | 1.09 s |
| [`e30b328`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e30b328a5d18f58935ab297a695d7fc12ad8c53a) | 2026-10-26 | 0.15 s | 0.19 s | 0.29 s | 0.47 s | 0.80 s | 1.09 s | 1.08 s |
| [`418d376`](https://github.com/helius-labs/zolana-mobile-sdk/commit/418d37618e67cd4ba68d8b1f5ff50890b54039a8) | 2026-10-26 | 0.17 s | 0.23 s | 0.36 s | 0.54 s | 0.92 s | 1.29 s | 1.28 s |
| [`455be9e`](https://github.com/helius-labs/zolana-mobile-sdk/commit/455be9ee158cbba0fa0c63ca37e5de678fb8fe6d) | 2026-10-26 | 0.16 s | 0.21 s | 0.32 s | 0.50 s | 0.80 s | 1.16 s | 1.14 s |
| [`842ed6b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/842ed6b9dc8953fe88cf1c3bf79dfc3d8b1d525b) | 2026-10-26 | 0.15 s | 0.19 s | 0.29 s | 0.48 s | 0.76 s | 1.09 s | 1.09 s |
| [`3457ad0`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3457ad00eca9cacb4294e39032c6b339cf47e019) | 2026-10-25 | 0.17 s | 0.21 s | 0.33 s | 0.51 s | 0.81 s | 1.17 s | 1.15 s |
| [`c13ac6c`](https://github.com/helius-labs/zolana-mobile-sdk/commit/c13ac6cb4072d7a9bac4067fcfd1f7db1ec90f60) | 2026-10-23 | 0.17 s | 0.22 s | 0.34 s | 0.53 s | 0.85 s | 1.21 s | 1.18 s |
| [`964d396`](https://github.com/helius-labs/zolana-mobile-sdk/commit/964d396cf2443cecce962763d87f78655226c7cf) | 2026-10-23 | 0.16 s | 0.19 s | 0.31 s | 0.50 s | 0.79 s | 1.14 s | 1.13 s |
| [`3dd795c`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3dd795c879d76401167acd449f0c32a31d24e021) | 2026-10-23 | 0.15 s | 0.19 s | 0.30 s | 0.48 s | 0.78 s | 1.09 s | 1.10 s |
| [`030dc90`](https://github.com/helius-labs/zolana-mobile-sdk/commit/030dc902f09661f34b94700f3c8e5fc192051ca9) | 2026-10-23 | 0.16 s | 0.20 s | 0.32 s | 0.49 s | 0.81 s | 1.14 s | 1.12 s |
| [`a9931b5`](https://github.com/helius-labs/zolana-mobile-sdk/commit/a9931b55f4dd54246aa2939dbe21056bc6b6696e) | 2026-10-21 | 0.24 s | 0.39 s | 0.66 s | 1.21 s | 2.24 s | 3.68 s | 3.58 s |
| [`3ca9427`](https://github.com/helius-labs/zolana-mobile-sdk/commit/3ca94276c3576e8bcf54e80e3fea0741b3ba5872) | 2026-10-21 | 0.23 s | 0.38 s | 0.67 s | 1.25 s | 2.30 s | 3.74 s | 3.64 s |
| [`7919e1b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/7919e1b763751e7f483b08f8b757bd0aade73625) | 2026-10-20 | 0.23 s | 0.40 s | 0.69 s | 1.26 s | 2.33 s | 3.77 s | 3.76 s |
| [`9bde901`](https://github.com/helius-labs/zolana-mobile-sdk/commit/9bde901088fa31b64651906cdbefcf755372c6f8) | 2026-10-19 | 0.23 s | 0.38 s | 0.68 s | 1.24 s | 2.30 s | 3.72 s | 3.62 s |
| [`00b2dbe`](https://github.com/helius-labs/zolana-mobile-sdk/commit/00b2dbe2265c90ce984c6634e5227b65027730c2) | 2026-10-19 | 0.21 s | 0.36 s | 0.64 s | 1.19 s | 2.24 s | 3.66 s | 3.64 s |
| [`70becb6`](https://github.com/helius-labs/zolana-mobile-sdk/commit/70becb6f171f86d89298c1a25dc15890a4ac96bc) | 2026-10-19 | 0.23 s | 0.38 s | 0.68 s | 1.23 s | 2.27 s | 3.70 s | 3.63 s |
| [`e28587b`](https://github.com/helius-labs/zolana-mobile-sdk/commit/e28587bf51d4dcfde3e979a469bed69ee1b48930) | 2026-10-17 | 0.22 s | 0.38 s | 0.67 s | 1.23 s | 2.26 s | 3.68 s | 3.60 s |
