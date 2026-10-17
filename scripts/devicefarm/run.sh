#!/usr/bin/env bash
# One proof timing run on AWS Device Farm: uploads the bench app and its
# test package (scripts/test-consumer.sh bench-apk or bench-ipa), on
# Android with testspec.yml, runs them on a device pool, waits, and leaves
# the device's timing lines in ZOLANA_TIMING_DIR as <label>.log for
# scripts/proof-timing-report.py and scripts/proof-timing-ci.sh. Needs AWS
# credentials with Device Farm access.
#
#   devicefarm/run.sh <project arn> <device pool arn> <bench dir> <label> [android|ios]
set -euo pipefail

project="$1"
pool="$2"
apks="$3"
label="$4"
platform="${5:-android}"
region=us-west-2
timing="${ZOLANA_TIMING_DIR:?set ZOLANA_TIMING_DIR}"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export AWS_PAGER=""

# upload <type> <file>: the upload's ARN once Device Farm has processed it.
upload() {
  local type="$1" file="$2" arn url status
  read -r arn url < <(aws devicefarm create-upload --region "$region" --project-arn "$project" \
    --name "$(basename "$file")" --type "$type" --query 'upload.[arn,url]' --output text)
  curl -sfS -T "$file" "$url"
  while :; do
    status="$(aws devicefarm get-upload --region "$region" --arn "$arn" --query upload.status --output text)"
    case "$status" in
      SUCCEEDED) break ;;
      FAILED) echo "upload of $file failed: $(aws devicefarm get-upload --region "$region" --arn "$arn" --query upload.message --output text)" >&2; return 1 ;;
      *) sleep 5 ;;
    esac
  done
  echo "$arn"
}

case "$platform" in
  android)
    app="$(upload ANDROID_APP "$apks/app.apk")"
    tests="$(upload INSTRUMENTATION_TEST_PACKAGE "$apks/app-androidTest.apk")"
    spec="$(upload INSTRUMENTATION_TEST_SPEC "$here/testspec.yml")"
    test="type=INSTRUMENTATION,testPackageArn=$tests,testSpecArn=$spec" ;;
  ios)
    # XCTest runs in Device Farm's standard environment: no spec, one run
    # per install, so the app makes both rows itself (bench-ipa).
    app="$(upload IOS_APP "$apks/app.ipa")"
    tests="$(upload XCTEST_TEST_PACKAGE "$apks/tests.zip")"
    test="type=XCTEST,testPackageArn=$tests" ;;
  *) echo "platform must be android or ios" >&2; exit 2 ;;
esac
run="$(aws devicefarm schedule-run --region "$region" --project-arn "$project" --app-arn "$app" \
  --device-pool-arn "$pool" --name "proof timing $label $(date -u +%Y-%m-%dT%H:%M)" \
  --test "$test" \
  --execution-configuration "jobTimeoutMinutes=120,videoCapture=false" \
  --query run.arn --output text)"
echo "run $run"
echo "console https://us-west-2.console.aws.amazon.com/devicefarm/home#/mobile/projects/${project##*:}/runs/${run##*/}"

while :; do
  read -r status result < <(aws devicefarm get-run --region "$region" --arn "$run" --query 'run.[status,result]' --output text)
  echo "$(date -u +%H:%M:%S) $status $result"
  [[ "$status" == COMPLETED ]] && break
  sleep 60
done

# The test spec's $DEVICEFARM_LOG_DIR comes back as the run's customer
# artifact; should its timing.log be empty, Device Farm's own logcat
# artifacts carry the same lines.
mkdir -p "$timing"
work="$(mktemp -d)"
log="$timing/$label.log"
: > "$log"
artifact() {
  aws devicefarm list-artifacts --region "$region" --arn "$run" --type FILE \
    --query "artifacts[?type=='$1'].url" --output text | tr '\t' '\n' | grep '^http' || true
}
artifact CUSTOMER_ARTIFACT | while read -r url; do
  curl -sfS -o "$work/customer.zip" "$url"
  unzip -q -o "$work/customer.zip" -d "$work/customer"
done
find "$work/customer" -name timing.log -exec cat {} + >> "$log" 2> /dev/null || true
if ! grep -q 'ZOLANA_TIMING {' "$log"; then
  echo "the test spec captured no timing lines; reading Device Farm's logcat"
  artifact DEVICE_LOG | while read -r url; do
    curl -sfS "$url" | grep -o 'ZOLANA_TIMING {.*' >> "$log" || true
  done
fi
minutes="$(aws devicefarm get-run --region "$region" --arn "$run" --query 'run.deviceMinutes.total' --output text)"
lines="$(grep -c 'ZOLANA_TIMING {' "$log" || true)"
echo "$lines timing lines in $log (result $result, $minutes device minutes)"
[[ "$lines" -gt 0 ]]
