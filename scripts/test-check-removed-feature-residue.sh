#!/usr/bin/env bash
# test-check-removed-feature-residue: self-test for the residue scanner.
# Verifies that:
#   [1/5] the positive fixture passes (exit 0, no active findings),
#   [2/5] the negative toml fixture fails with exact generated/maxperf findings,
#   [3/5] the negative profile target-cpu fixture fails with the exact token,
#   [4/5] the negative profile pgo fixture fails with each PGO-context token,
#   [5/5] the real repository scan passes (exit 0, allowlisted=6, no active residue).
#
# Exits 0 on success, exits 1 on any failed assertion.
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$ROOT"

GATE="$ROOT/scripts/check-removed-feature-residue.sh"
POSITIVE="$ROOT/fixtures/removed-feature-residue/positive.toml"
NEGATIVE_TOML="$ROOT/fixtures/removed-feature-residue/negative.toml"
NEGATIVE_PROFILE_TARGET_CPU="$ROOT/fixtures/removed-feature-residue/negative_profile.txt"
NEGATIVE_PROFILE_PGO="$ROOT/fixtures/removed-feature-residue/negative_profile_pgo.txt"

if [[ ! -x "$GATE" ]]; then
  echo "AssertionFailed: gate script is missing or not executable: $GATE" >&2
  exit 1
fi
for fixture in "$POSITIVE" "$NEGATIVE_TOML" "$NEGATIVE_PROFILE_TARGET_CPU" "$NEGATIVE_PROFILE_PGO"; do
  if [[ ! -f "$fixture" ]]; then
    echo "AssertionFailed: fixture missing: $fixture" >&2
    exit 1
  fi
done

assert_exit() {
  local label="$1"
  local expected="$2"
  local actual="$3"
  local output="$4"
  if [[ "$expected" != "$actual" ]]; then
    printf 'AssertionFailed: %s expected exit %s, got %s\nOutput:\n%s\n' \
      "$label" "$expected" "$actual" "$output" >&2
    exit 1
  fi
}

run_gate_capture() {
  set +e
  local output
  output="$(bash "$GATE" "$@" 2>&1)"
  local exit_code=$?
  set -e
  GATE_OUTPUT="$output"
  GATE_EXIT=$exit_code
}

assert_output_contains() {
  local label="$1"
  local needle="$2"
  local haystack="$3"
  case "$haystack" in
    *"$needle"*) ;;
    *)
      printf 'AssertionFailed: %s missing %s\nOutput:\n%s\n' \
        "$label" "$needle" "$haystack" >&2
      exit 1
    ;;
  esac
}

assert_output_omits() {
  local label="$1"
  local needle="$2"
  local haystack="$3"
  case "$haystack" in
    *"$needle"*)
      printf 'AssertionFailed: %s unexpectedly contained %s\nOutput:\n%s\n' \
        "$label" "$needle" "$haystack" >&2
      exit 1
    ;;
  esac
}

printf '[1/5] positive fixture must PASS (exit 0, no active findings)\n'
run_gate_capture "$POSITIVE"
assert_exit "positive fixture" "0" "$GATE_EXIT" "$GATE_OUTPUT"
assert_output_contains "positive summary" "summary: active=0 allowlisted=0 files_scanned=1" "$GATE_OUTPUT"
assert_output_omits "positive active line" " REMOVED-FEATURE:" "$GATE_OUTPUT"
echo "  ok: exit 0"
echo "  ok: summary reports active=0 allowlisted=0 files_scanned=1"
echo "  ok: no REMOVED-FEATURE line in output"

printf '[2/5] negative toml fixture must FAIL (exit 1, file:line finding)\n'
run_gate_capture "$NEGATIVE_TOML"
assert_exit "negative toml fixture" "1" "$GATE_EXIT" "$GATE_OUTPUT"
assert_output_contains "negative toml file:line" \
  "fixtures/removed-feature-residue/negative.toml:" "$GATE_OUTPUT"
assert_output_contains "negative toml generated token" \
  "REMOVED-FEATURE: generated: feature identifier 'generated =' inside [features] block" \
  "$GATE_OUTPUT"
assert_output_contains "negative toml maxperf token" \
  "REMOVED-FEATURE: maxperf: feature identifier 'maxperf =' inside [features] block" \
  "$GATE_OUTPUT"
assert_output_contains "negative toml summary" "summary: active=2 allowlisted=0 files_scanned=1" "$GATE_OUTPUT"
echo "  ok: exit 1 with file:line finding"
echo "  ok: summary reports active=2 allowlisted=0 files_scanned=1"
echo "  ok: exact generated/maxperf findings present"

printf '[3/5] negative profile target-cpu fixture must FAIL (exit 1, file:line finding)\n'
run_gate_capture "$NEGATIVE_PROFILE_TARGET_CPU"
assert_exit "negative profile target-cpu fixture" "1" "$GATE_EXIT" "$GATE_OUTPUT"
assert_output_contains "negative profile target-cpu file:line" \
  "fixtures/removed-feature-residue/negative_profile.txt:" "$GATE_OUTPUT"
assert_output_contains "negative profile target-cpu token" \
  "REMOVED-FEATURE: target-cpu=native: exact substring 'target-cpu=native'" "$GATE_OUTPUT"
assert_output_contains "negative profile target-cpu summary" "summary: active=1 allowlisted=0 files_scanned=1" "$GATE_OUTPUT"
echo "  ok: exit 1 with file:line finding"
echo "  ok: summary reports active=1 allowlisted=0 files_scanned=1"
echo "  ok: exact target-cpu finding present"

printf '[4/5] negative profile pgo fixture must FAIL (exit 1, file:line finding)\n'
run_gate_capture "$NEGATIVE_PROFILE_PGO"
assert_exit "negative profile pgo fixture" "1" "$GATE_EXIT" "$GATE_OUTPUT"
assert_output_contains "negative profile pgo file:line" \
  "fixtures/removed-feature-residue/negative_profile_pgo.txt:" "$GATE_OUTPUT"
assert_output_contains "negative profile pgo assignment token" \
  "REMOVED-FEATURE: pgo: PGO active context 'pgo = '" "$GATE_OUTPUT"
assert_output_contains "negative profile cargo pgo token" \
  "REMOVED-FEATURE: pgo: PGO active context 'cargo pgo'" "$GATE_OUTPUT"
assert_output_contains "negative profile rustc pgo token" \
  "REMOVED-FEATURE: pgo: PGO active context 'RUSTC_PGO'" "$GATE_OUTPUT"
assert_output_contains "negative profile pgo-data token" \
  "REMOVED-FEATURE: pgo: PGO active context 'pgo-data'" "$GATE_OUTPUT"
assert_output_contains "negative profile pgo summary" "summary: active=4 allowlisted=0 files_scanned=1" "$GATE_OUTPUT"
echo "  ok: exit 1 with file:line finding"
echo "  ok: summary reports active=4 allowlisted=0 files_scanned=1"
echo "  ok: exact PGO context findings present"

printf '[5/5] real repository scan must PASS (exit 0, no active residue)\n'
run_gate_capture
assert_exit "real repository scan" "0" "$GATE_EXIT" "$GATE_OUTPUT"
assert_output_contains "real repository summary" "summary: active=0 allowlisted=6" "$GATE_OUTPUT"
assert_output_contains "real repository coverage" "files_scanned=" "$GATE_OUTPUT"
assert_output_omits "real repository active line" " REMOVED-FEATURE:" "$GATE_OUTPUT"
echo "  ok: exit 0"
echo "  ok: summary reports active=0 allowlisted=6"
echo "  ok: files_scanned coverage is present"
echo "  ok: no REMOVED-FEATURE line in output"

echo "self-test PASSED"
exit 0
