# The proof timing bench on AWS Device Farm

`.github/workflows/device-bench.yml` runs the proof timing harness on a
phone: nightly on `main`, on a pull request labelled `bench-device`, or by
hand. `scripts/test-consumer.sh bench-apk` builds the bench app and its
instrumentation for arm64; `run.sh` uploads them with `testspec.yml`,
which runs the instrumentation twice on one install (the harness takes the
first run, with no keys on the device, as the fresh one and the second as
the restart), waits, and brings back the timing lines for the usual report.

One run is about 35 device minutes: $0.17 a minute after the account's
trial minutes, or an unmetered slot.

Setup, once, in `us-west-2` (Device Farm's only region):

```sh
aws devicefarm create-project --name zolana-mobile-bench --default-job-timeout-minutes 120
aws devicefarm create-device-pool --project-arn <project arn> --name galaxy-a14-5g \
  --rules '[{"attribute":"ARN","operator":"IN","value":"[\"arn:aws:devicefarm:us-west-2::device:79F9A58D576F4D2AB1A8AAF683201ADB\"]"}]'
```

The pool pins one model (`aws devicefarm list-devices` lists them with
their ARNs), so that runs compare. The workflow assumes an IAM role through
GitHub's OIDC provider: `iam-trust.json` is its trust policy and
`iam-policy.json` its permissions, both with `<account>` and `<project id>`
to fill in. The repository variables `DEVICEFARM_ROLE_ARN`,
`DEVICEFARM_PROJECT_ARN` and `DEVICEFARM_POOL_ARN` carry the ARNs.

Locally, with AWS credentials:

```sh
ZOLANA_CONSUMER_DIR=/tmp/consumer ZOLANA_BENCH_APK_DIR=/tmp/apks ZOLANA_TIMING_LABEL=galaxy-a14-5g \
  bash scripts/test-consumer.sh bench-apk
ZOLANA_TIMING_DIR=/tmp/timing bash scripts/devicefarm/run.sh <project arn> <pool arn> /tmp/apks galaxy-a14-5g
python3 scripts/proof-timing-report.py --logs /tmp/timing
```
