# The proof timing bench on AWS Device Farm

`.github/workflows/device-bench.yml` runs the proof timing harness, in
release builds, on the phones of `devices.json`: a low, a mid and a high
tier Android phone (Samsung Galaxy A16, A56, S25) and iPhone (12, 15, 17
Pro), on every pull request, every push to `main`, or by hand. The jobs
skip until the repository variable
`DEVICEFARM_ROLE_ARN` is set. `scripts/test-consumer.sh bench-apk` builds
the bench app and its instrumentation for arm64, and `bench-ipa` the iOS app
and its XCTest bundle, unsigned (Device Farm re-signs); `run.sh` uploads one
platform's build once, runs it on that platform's phones at once, waits,
and reads each phone's timing lines from Device Farm's device log into
`<label>/timing.log` for the report. On Android `testspec.yml` runs the
instrumentation twice on one install, and the harness takes the first run,
with no keys on the device, as the fresh one and the second as the
restart; iOS tests get one run per install, so there the harness makes the
restart rows after the fresh ones in one process, closing and reopening the
wallet between (`ZOLANA_TIMING_RUN=both`): the keys load again, the process
does not start again.

A run is 3 (Galaxy S25) to 14 (Galaxy A16) device minutes per phone, about
55 for all six.

Setup, once, in `us-west-2` (Device Farm's only region):

```sh
aws devicefarm create-project --name zolana-mobile-bench --default-job-timeout-minutes 120
```

`devices.json` pins each phone by its device ARN (`aws devicefarm
list-devices` lists them), so that runs compare. The workflow assumes an
IAM role through GitHub's OIDC provider: `iam-trust.json` is its trust
policy and `iam-policy.json` its permissions, both with `<account>` and
`<project id>` to fill in; Device Farm's list calls take no resource, so
those two are allowed on `*`. GitHub signs this organization's tokens with
immutable subjects (`repo:<owner>@<owner id>/<repo>@<repo id>:...`), which
the trust condition must match. The role is
`zolana-mobile-devicefarm-bench`. The repository variables
`DEVICEFARM_ROLE_ARN` and `DEVICEFARM_PROJECT_ARN` carry the ARNs.

Locally, with AWS credentials (`low` for the low tier phone only):

```sh
ZOLANA_CONSUMER_DIR=/tmp/consumer ZOLANA_BENCH_DIR=/tmp/apks bash scripts/test-consumer.sh bench-apk
ZOLANA_TIMING_DIR=/tmp/timing bash scripts/devicefarm/run.sh <project arn> /tmp/apks android all
ZOLANA_CONSUMER_DIR=/tmp/consumer ZOLANA_BENCH_DIR=/tmp/ipa bash scripts/test-consumer.sh bench-ipa
ZOLANA_TIMING_DIR=/tmp/timing bash scripts/devicefarm/run.sh <project arn> /tmp/ipa ios all
python3 scripts/proof-timing-report.py --logs /tmp/timing
```
