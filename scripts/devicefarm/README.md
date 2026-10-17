# The proof timing bench on AWS Device Farm

`.github/workflows/device-bench.yml` runs the proof timing harness, in
release builds, on two phones, a Samsung Galaxy A16 and an iPhone 12: on
every pull request, every push to `main`, or by hand. The jobs skip until
the repository variable `DEVICEFARM_ROLE_ARN` is set. `scripts/test-consumer.sh
bench-apk` builds the bench app and its instrumentation for arm64, and
`bench-ipa` the iOS app and its XCTest bundle, unsigned (Device Farm
re-signs); `run.sh` uploads them, waits, and reads the timing lines from Device
Farm's device log for the usual report. On Android `testspec.yml` runs the instrumentation
twice on one install, and the harness takes the first run, with no keys on
the device, as the fresh one and the second as the restart; iOS tests get
one run per install, so there the harness makes the restart rows after the
fresh ones in one process, closing and reopening the wallet between
(`ZOLANA_TIMING_RUN=both`): the keys load again, the process does not
start again.

A run is 10 to 35 device minutes per phone.

Setup, once, in `us-west-2` (Device Farm's only region):

```sh
aws devicefarm create-project --name zolana-mobile-bench --default-job-timeout-minutes 120
aws devicefarm create-device-pool --project-arn <project arn> --name galaxy-a16 \
  --rules '[{"attribute":"ARN","operator":"IN","value":"[\"arn:aws:devicefarm:us-west-2::device:E10A10A4F8AD4DA6926C1A03E4BF854F\"]"}]'
aws devicefarm create-device-pool --project-arn <project arn> --name iphone-12 \
  --rules '[{"attribute":"ARN","operator":"IN","value":"[\"arn:aws:devicefarm:us-west-2::device:3D9D3DF8F5E6491A9BCEFE9B84759D8C\"]"}]'
```

Each pool pins one model (`aws devicefarm list-devices` lists them with
their ARNs), so that runs compare. The workflow assumes an IAM role through
GitHub's OIDC provider: `iam-trust.json` is its trust policy and
`iam-policy.json` its permissions, both with `<account>` and `<project id>`
to fill in; Device Farm's list calls take no resource, so those two are
allowed on `*`. The role is `zolana-mobile-devicefarm-bench`. The repository variables `DEVICEFARM_ROLE_ARN`,
`DEVICEFARM_PROJECT_ARN`, `DEVICEFARM_POOL_ARN` (Android) and
`DEVICEFARM_POOL_IOS_ARN` carry the ARNs.

Locally, with AWS credentials:

```sh
ZOLANA_CONSUMER_DIR=/tmp/consumer ZOLANA_BENCH_DIR=/tmp/apks ZOLANA_TIMING_LABEL=galaxy-a16 \
  bash scripts/test-consumer.sh bench-apk
ZOLANA_TIMING_DIR=/tmp/timing bash scripts/devicefarm/run.sh <project arn> <pool arn> /tmp/apks galaxy-a16
ZOLANA_CONSUMER_DIR=/tmp/consumer ZOLANA_BENCH_DIR=/tmp/ipa ZOLANA_TIMING_LABEL=iphone-12 \
  bash scripts/test-consumer.sh bench-ipa
ZOLANA_TIMING_DIR=/tmp/timing bash scripts/devicefarm/run.sh <project arn> <ios pool arn> /tmp/ipa iphone-12 ios
python3 scripts/proof-timing-report.py --logs /tmp/timing
```
