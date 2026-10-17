#!/usr/bin/env bash
# One proof timing run on AWS Device Farm: uploads the bench app and its
# test package of one platform (scripts/test-consumer.sh bench-apk or
# bench-ipa), on Android with testspec.yml, runs them on that platform's
# phones of devices.json at once, waits, and leaves each phone's timing
# lines in ZOLANA_TIMING_DIR/<label>/timing.log for
# scripts/proof-timing-report.py. `low` takes the low tier phone, `all`
# every phone. Needs AWS credentials with Device Farm access.
#
#   devicefarm/run.sh <project arn> <bench dir> <android|ios> [low|all]
set -euo pipefail

project="$1"
bench="$2"
platform="$3"
tiers="${4:-all}"
region=us-west-2
timing="${ZOLANA_TIMING_DIR:?set ZOLANA_TIMING_DIR}"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export AWS_PAGER=""

case "$tiers" in
  low) tier='.tier == "low"' ;;
  all) tier='true' ;;
  *) echo "tiers must be low or all" >&2; exit 2 ;;
esac
devices="$(jq -c --arg platform "$platform" "[.[] | select(.platform == \$platform and $tier)]" "$here/devices.json")"
if [[ "$(jq length <<< "$devices")" -eq 0 ]]; then
  echo "no $platform phones in devices.json" >&2
  exit 2
fi

# upload <type> <file>: the upload's ARN once Device Farm has processed it.
# It runs in a command substitution, where bash drops set -e, so a failed
# AWS call (a denied permission) returns explicitly instead of polling on.
upload() {
  local type="$1" file="$2" created arn url status
  created="$(aws devicefarm create-upload --region "$region" --project-arn "$project" \
    --name "$(basename "$file")" --type "$type" --query 'upload.[arn,url]' --output text)" || return 1
  read -r arn url <<< "$created"
  curl -sfS -T "$file" "$url" || return 1
  while :; do
    status="$(aws devicefarm get-upload --region "$region" --arn "$arn" --query upload.status --output text)" || return 1
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
    app="$(upload ANDROID_APP "$bench/app.apk")"
    tests="$(upload INSTRUMENTATION_TEST_PACKAGE "$bench/app-androidTest.apk")"
    spec="$(upload INSTRUMENTATION_TEST_SPEC "$here/testspec.yml")"
    test="type=INSTRUMENTATION,testPackageArn=$tests,testSpecArn=$spec" ;;
  ios)
    # XCTest runs in Device Farm's standard environment: no spec, one run
    # per install, so the app makes both rows itself (bench-ipa).
    app="$(upload IOS_APP "$bench/app.ipa")"
    tests="$(upload XCTEST_TEST_PACKAGE "$bench/tests.zip")"
    test="type=XCTEST,testPackageArn=$tests" ;;
  *) echo "platform must be android or ios" >&2; exit 2 ;;
esac

# A run left behind by a cancelled job would hold its phones to the end.
run=""
finished=""
# shellcheck disable=SC2329 # called by the trap
stop() {
  if [[ -n "$run" && -z "$finished" ]]; then
    aws devicefarm stop-run --region "$region" --arn "$run" > /dev/null && echo "stopped $run"
  fi
}
trap stop EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

run="$(aws devicefarm schedule-run --region "$region" --project-arn "$project" --app-arn "$app" \
  --device-selection-configuration "$(jq -c '{filters: [{attribute: "ARN", operator: "IN", values: map(.arn)}], maxDevices: length}' <<< "$devices")" \
  --name "proof timing $platform $tiers $(date -u +%Y-%m-%dT%H:%M)" \
  --test "$test" \
  --execution-configuration "jobTimeoutMinutes=120,videoCapture=false" \
  --query run.arn --output text)"
echo "run $run on $(jq -r 'map(.label) | join(", ")' <<< "$devices")"
echo "console https://us-west-2.console.aws.amazon.com/devicefarm/home#/mobile/projects/${project##*:}/runs/${run##*/}"

while :; do
  state="$(aws devicefarm get-run --region "$region" --arn "$run" --query 'run.[status,result]' --output text)"
  read -r status result <<< "$state"
  echo "$(date -u +%H:%M:%S) $status $result"
  [[ "$status" == COMPLETED ]] && break
  # In the background, so that a cancel stops the run at once.
  sleep 60 &
  wait $!
done
finished=1

# The harness's lines come from each phone's device log, which Device Farm
# captures itself through every phase (the test spec cannot: Device Farm
# clears the buffer when it restarts its capture). A phone with no lines,
# or whose test did not pass, fails the run once every phone is read.
failed=0
jobs="$(aws devicefarm list-jobs --region "$region" --arn "$run" --query 'jobs[].[arn,device.arn,result]' --output text)"
while read -r job device result <&3; do
  label="$(jq -r --arg arn "$device" '.[] | select(.arn == $arn) | .label' <<< "$devices")"
  mkdir -p "$timing/$label"
  log="$timing/$label/timing.log"
  : > "$log"
  urls="$(aws devicefarm list-artifacts --region "$region" --arn "$job" --type FILE \
    --query "artifacts[?type=='DEVICE_LOG'].url" --output text)"
  for url in $urls; do
    curl -sfS "$url" | grep -o 'ZOLANA_TIMING {.*' >> "$log" || true
  done
  lines="$(grep -c 'ZOLANA_TIMING {' "$log" || true)"
  echo "$label: $lines timing lines, $result"
  if [[ "$lines" -eq 0 || "$result" != PASSED ]]; then
    failed=1
  fi
done 3<<< "$jobs"
echo "$(aws devicefarm get-run --region "$region" --arn "$run" --query 'run.deviceMinutes.total' --output text) device minutes"
exit "$failed"
