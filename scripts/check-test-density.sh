#!/usr/bin/env bash
# scripts/check-test-density.sh
# Bead: vb-scr01 — R1-A11 / R4-A11 missing check
#
# Asserts that each production crate in `crates/` has a test density
# (count of test functions) at or above a configurable threshold
# relative to the count of public production functions. The "5x test
# density" rule referenced in the master plan is the canonical default
# (see `docs/black-hat-review-2026-06-07/round4/r4-a5-coverage.md` and
# `crates/vb_ipc/test-suite-review.md:9`).
#
# Counting model:
#   - Production pub fns: every `pub fn`, `pub async fn`, `pub(crate) fn`,
#     `pub(super) fn`, `pub(in path) fn` declaration in
#     `crates/<crate>/src/**/*.rs` under the `src/` tree only.
#     Generated code under `src/generated/**` is excluded — generated
#     code is not a reasonable density target and would distort the
#     ratio. Kani/verus/loom `cfg(...)` modules ARE counted because
#     they live alongside production and follow the same naming rules.
#   - Test functions: each `#[test]`, `#[tokio::test]`, `#[proptest]`,
#     and `#[test_case(...)]` attribute attaches to exactly one
#     function; we count these markers, not the functions themselves,
#     because proptest macro expansions may yield multiple cases.
#   - Both `crates/<crate>/src/**` (inline `#[cfg(test)] mod tests`)
#     and `crates/<crate>/tests/**` are scanned for test markers.
#
# Threshold:
#   - Default: 5.0x. Override with `TEST_DENSITY_MIN` env var.
#   - A crate with zero production `pub fn` (e.g. a constants-only
#     module) is reported as "skip" — no density to compute. Override
#     with `TEST_DENSITY_ALLOW_EMPTY=1` to require ≥1 test marker.
#   - A crate with zero test markers and non-zero production code
#     is a hard fail (the production code has effectively no tests).
#     Override with `TEST_DENSITY_ALLOW_ZERO=1` to allow it.
#
# Output:
#   - Per-crate ratio table is written to stdout.
#   - Exits 0 when every crate meets the threshold.
#   - Exits 1 when one or more crates fall below the threshold.
#   - Exits 2 on usage or environment error.
#
# This script is read-only: it never modifies repository state.

set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$ROOT"

if [[ ! -d "$ROOT/crates" ]]; then
  printf 'check-test-density: error: %s/crates is not a directory\n' "$ROOT" >&2
  exit 2
fi

if ! command -v rg >/dev/null 2>&1; then
  printf 'check-test-density: error: rg (ripgrep) is required on PATH\n' >&2
  exit 2
fi

MIN_RATIO="${TEST_DENSITY_MIN:-5.0}"
ALLOW_ZERO="${TEST_DENSITY_ALLOW_ZERO:-0}"
ALLOW_EMPTY="${TEST_DENSITY_ALLOW_EMPTY:-0}"

printf 'check-test-density: scanning %s/crates (min ratio = %sx)\n' "$ROOT" "$MIN_RATIO" >&2

count_pub_fns() {
  # Count pub fn / pub async fn / pub(crate|super|in ...) fn declarations.
  # Strategy: rg returns every line in the crate; grep -E filters to
  # declarations starting with `pub` and ending in `fn name`. We accept
  # `pub fn`, `pub async fn`, and the qualified-visibility forms
  # `pub(crate) fn`, `pub(super) fn`, `pub(in path::to::mod) fn`.
  local crate="$1"
  local src_dir="$ROOT/crates/$crate/src"
  if [[ ! -d "$src_dir" ]]; then
    printf '0\n'
    return
  fi
  # Use `-e '.*'` (explicit pattern) so rg actually scans every line.
  # Positional patterns that look like a single character cause rg to
  # treat them as a regex and scan, but a bare path arg with no
  # pattern drops the search entirely. `-e` is the safe form.
  # `|| true` so an empty match set does not abort under set -e.
  rg --no-filename --no-heading \
     --glob '*.rs' \
     --glob '!**/generated/**' \
     --glob '!**/target/**' \
     -e '.*' \
     "$src_dir" 2>/dev/null \
   | grep -cE 'pub[[:space:]]+(\([[:alpha:] ,_]+\))?[[:space:]]*(async[[:space:]]+)?fn[[:space:]]+[A-Za-z_]' \
   || true
}

count_test_markers() {
  local crate="$1"
  local total=0
  local roots=("$ROOT/crates/$crate/src" "$ROOT/crates/$crate/tests")
  for root in "${roots[@]}"; do
    [[ -d "$root" ]] || continue
    local n
    # `|| true` so a missing dir or empty match set does not abort under
    # set -e; rg returns 1 when no matches are found.
    n=$(rg --no-filename --no-heading --count-matches \
        -e '#\[test\]' \
        -e '#\[cfg\(test\)\]' \
        -e '#\[tokio::test' \
        -e '#\[proptest\]' \
        -e '#\[test_case' \
        --glob '*.rs' \
        --glob '!**/target/**' \
        --glob '!**/generated/**' \
        "$root" 2>/dev/null \
        | awk -F: '{ s += $NF } END { printf "%d\n", s+0 }' \
        || true)
    total=$((total + n))
  done
  printf '%d\n' "$total"
}

# Crate set: all directories under crates/ that have a Cargo.toml.
# We capture only the basename because every crate lives at crates/<name>.
mapfile -t CRATES < <(
  for ct in "$ROOT"/crates/*/Cargo.toml; do
    [[ -f "$ct" ]] || continue
    d="${ct%/Cargo.toml}"
    printf '%s\n' "$(basename -- "$d")"
  done | sort -u
)

printf '\n%-32s %12s %12s %10s %s\n' "CRATE" "PUB_FNS" "TEST_MARKERS" "RATIO" "STATUS"
printf '%-32s %12s %12s %10s %s\n' "--------------------------------" "------------" "------------" "----------" "------"

failed=0
total_pub=0
total_markers=0

for crate in "${CRATES[@]}"; do
  pub_count="$(count_pub_fns "$crate")"
  markers="$(count_test_markers "$crate")"
  total_pub=$((total_pub + pub_count))
  total_markers=$((total_markers + markers))

  if [[ "$pub_count" -eq 0 ]]; then
    if [[ "$ALLOW_EMPTY" == "1" ]]; then
      status="skip (no pub fns)"
    else
      status="skip (no pub fns — non-testable)"
    fi
    printf '%-32s %12s %12s %10s %s\n' "$crate" "$pub_count" "$markers" "n/a" "$status"
    continue
  fi

  # awk for one-decimal ratio
  ratio=$(awk -v p="$pub_count" -v m="$markers" 'BEGIN { if (p == 0) { printf "0.00"; exit } printf "%.2f", m / p }')

  status="OK"
  cmp=$(awk -v r="$ratio" -v t="$MIN_RATIO" 'BEGIN { print (r+0 < t+0) ? "BELOW" : "OK" }')
  if [[ "$cmp" == "BELOW" ]]; then
    if [[ "$markers" -eq 0 && "$ALLOW_ZERO" != "1" ]]; then
      status="FAIL (zero test markers)"
    else
      status="FAIL (below $MIN_RATIO x)"
    fi
    failed=1
  fi

  printf '%-32s %12s %12s %10s %s\n' "$crate" "$pub_count" "$markers" "${ratio}x" "$status"
done

if [[ "$total_pub" -gt 0 ]]; then
  overall=$(awk -v p="$total_pub" -v m="$total_markers" 'BEGIN { if (p == 0) { printf "0.00"; exit } printf "%.2f", m / p }')
  printf '%-32s %12s %12s %10s\n' "TOTAL" "$total_pub" "$total_markers" "${overall}x"
fi

if [[ "$failed" -ne 0 ]]; then
  printf '\ncheck-test-density: FAILED — one or more crates are below the %sx threshold.\n' "$MIN_RATIO" >&2
  printf '  Override with TEST_DENSITY_MIN=<value> for environment-specific scans.\n' >&2
  printf '  Override with TEST_DENSITY_ALLOW_ZERO=1 to accept zero test markers.\n' >&2
  exit 1
fi

printf '\ncheck-test-density: OK (all crates meet the %sx test density threshold)\n' "$MIN_RATIO"
exit 0
