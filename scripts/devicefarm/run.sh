#!/usr/bin/env bash
# One proof timing run on AWS Device Farm: uploads the bench app and its
# test package (scripts/test-consumer.sh bench-apk) with testspec.yml, runs
# them on a device pool, waits, and leaves the device's timing lines in
# ZOLANA_TIMING_DIR as <label>.log for scripts/proof-timing-report.py and
# scripts/proof-timing-ci.sh. Needs AWS credentials with Device Farm access.
#
#   devicefarm/run.sh <project arn> <device pool arn> <apk dir> <label>
set -euo pipefail

project="$1"
pool="$2"
apks="$3"
label="$4"
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

app="$(upload ANDROID_APP "$apks/app.apk")"
tests="$(upload INSTRUMENTATION_TEST_PACKAGE "$apks/app-androidTest.apk")"
spec="$(upload INSTRUMENTATION_TEST_SPEC "$here/testspec.yml")"
run="$(aws devicefarm schedule-run --region "$region" --project-arn "$project" --app-arn "$app" \
  --device-pool-arn "$pool" --name "proof timing $label $(date -u +%Y-%m-%dT%H:%M)" \
  --test "type=INSTRUMENTATION,testPackageArn=$tests,testSpecArn=$spec" \
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

# The test spec's $DEVICEFARM_LOG_DIR comes back as the "Customer Artifacts"
# zip of the job.
mkdir -p "$timing"
work="$(mktemp -d)"
aws devicefarm list-artifacts --region "$region" --arn "$run" --type FILE \
  --query 'artifacts[?contains(name, `Customer Artifacts`)].url' --output text | tr '\t' '\n' | while read -r url; do
  [[ -n "$url" ]] || continue
  curl -sfS -o "$work/artifacts.zip" "$url"
  unzip -q -o "$work/artifacts.zip" -d "$work/artifacts"
done
found="$(find "$work/artifacts" -name timing.log | head -1 || true)"
if [[ -z "$found" ]]; then
  echo "no timing.log among the run's artifacts; result was $result" >&2
  find "$work/artifacts" -type f | head -20 >&2
  exit 1
fi
cp "$found" "$timing/$label.log"
echo "$(grep -c 'ZOLANA_TIMING {' "$timing/$label.log") timing lines in $timing/$label.log (result $result)"
