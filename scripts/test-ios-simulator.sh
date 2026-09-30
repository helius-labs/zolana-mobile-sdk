#!/usr/bin/env bash
# Runs the clean consumer integration test on an iOS simulator with a deadline
# on every phase. Hosted macOS runners sometimes never finish a simulator boot,
# and an unpatched Flutter tool can miss the app's VM Service URL and wait
# forever (see scripts/patch-flutter-tool.sh); without a deadline the job waits
# for the 6-hour runner limit. A boot that misses its deadline is retried once
# on a fresh device. A test that misses its deadline fails, and prints the
# process tree and simulator log so the stuck phase is visible. The test
# deadline covers the Xcode build, which takes up to 18 minutes on a slow
# runner.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
boot_seconds="${ZOLANA_BOOT_SECONDS:-600}"
test_seconds="${ZOLANA_TEST_SECONDS:-2400}"
diagnostics="${ZOLANA_DIAGNOSTICS_DIR:-${RUNNER_TEMP:-${TMPDIR:-/tmp}}/ios-simulator-diagnostics}"
mkdir -p "$diagnostics"

log() {
  printf '[%s] %s\n' "$(date -u +%H:%M:%S)" "$*"
}

# Diagnostics and cleanup talk to a simulator that may be wedged, so each call
# gets a short deadline of its own.
quick() {
  perl -e 'alarm shift; exec @ARGV' 60 "$@"
}

device=""
created=()
cleanup() {
  for udid in ${created[@]+"${created[@]}"}; do
    quick xcrun simctl shutdown "$udid" >/dev/null 2>&1 || true
    quick xcrun simctl delete "$udid" >/dev/null 2>&1 || true
  done
}
trap cleanup EXIT

# Prints what is still running and what the simulator logged, so a missed
# deadline shows whether it was the boot, the build, the install, the launch,
# or the wait for the app's VM service.
diagnose() {
  local file="$diagnostics/$1"
  echo "::group::Diagnostics: $1"
  echo "--- processes"
  ps -axo pid,ppid,etime,command | grep -E 'flutter|dart|xcodebuild|xcrun|simctl|cargo|go build|Runner' | grep -v grep || true
  echo "--- simulators"
  quick xcrun simctl list devices booted || true
  echo "--- CoreSimulator.log"
  grep -Ev 'Sending load request|Connecting to|Unable to discover' "$HOME/Library/Logs/CoreSimulator/CoreSimulator.log" 2>/dev/null | tail -50 || true
  if [[ -n "$device" ]]; then
    echo "--- app processes"
    quick xcrun simctl spawn "$device" launchctl list > "$file.launchctl.txt" 2>&1 || true
    grep -i UIKitApplication "$file.launchctl.txt" || echo "none"
    quick xcrun simctl spawn "$device" log show --last 15m --style compact > "$file.simulator.log" 2>&1 || true
    echo "--- Runner log, last 200 lines (full simulator log: $file.simulator.log)"
    grep -E '\bRunner\b' "$file.simulator.log" | tail -200 || true
  fi
  echo "::endgroup::"
}

# Runs "$@" in its own process group and kills the group after $1 seconds.
bounded() {
  local seconds="$1" label="$2"
  shift 2
  set -m
  "$@" &
  local pid=$!
  set +m
  local waited=0
  while kill -0 "$pid" 2>/dev/null; do
    if (( waited >= seconds )); then
      echo "::error::$label did not finish in ${seconds}s"
      diagnose "$label"
      kill -TERM -- "-$pid" 2>/dev/null || true
      sleep 10
      kill -KILL -- "-$pid" 2>/dev/null || true
      wait "$pid" 2>/dev/null || true
      return 124
    fi
    sleep 5
    waited=$((waited + 5))
  done
  wait "$pid"
}

read -r udid runtime device_type < <(
  xcrun simctl list devices available -j | ruby -rjson -e '
    JSON.parse(STDIN.read).fetch("devices").each do |runtime, devices|
      device = devices.find { |entry| entry.fetch("name").start_with?("iPhone") && entry.fetch("state") == "Shutdown" }
      next unless device
      puts [device.fetch("udid"), runtime, device.fetch("deviceTypeIdentifier")].join(" ")
      exit
    end
    abort("No available iPhone simulator")'
)

for attempt in 1 2; do
  device="$udid"
  log "booting $device ($device_type, $runtime), attempt $attempt"
  if bounded "$boot_seconds" "boot-$attempt" xcrun simctl bootstatus "$device" -b; then
    break
  fi
  if (( attempt == 2 )); then
    echo "::error::The simulator did not boot on a fresh device either" >&2
    exit 1
  fi
  quick xcrun simctl shutdown "$device" >/dev/null 2>&1 || true
  udid="$(quick xcrun simctl create "zolana-ci-$attempt" "$device_type" "$runtime")"
  created+=("$udid")
done
log "booted $device"

export ZOLANA_TEST_DEVICE="$device"
bounded "$test_seconds" test bash "$repo_root/scripts/test-consumer.sh" device
log "test passed"
