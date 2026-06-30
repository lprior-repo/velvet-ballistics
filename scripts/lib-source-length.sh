#!/usr/bin/env bash
# Source-length categorization library for the velvet-ballistics source-length gate.
#
# Categories and limits (lines):
#   production         300 (warn at 240)  first-party src/**/*.rs (default)
#   test_in_src       1500 (warn at 1200) src/**/tests.rs, src/**/*_tests.rs,
#                                         src/**/tests/**
#   test_top_level    3000 (warn at 2400) crates/*/tests/**, crates/*/benches/**,
#                                         benches/**
#   kani               800 (warn at  640) verification/kani/**, contracts/kani/**,
#                                         src/**/*kani*.rs
#   verus              800 (warn at  640) verification/verus/**, contracts/verus/**,
#                                         src/**/*verus*.rs
#   flux               800 (warn at  640) verification/flux/**, contracts/flux/**
#   verification       600 (warn at  480) src/verification/**
#   generated          -1  (excluded)     src/generated/** (per spec)
#   perf               -1  (excluded)     src/perf/** (per spec)
#
# A negative limit marks an excluded category: files in that category are
# never raised as failures or warnings, but they are still emitted as part
# of the summary so drift is observable.
#
# Functions:
#   sl_categorize <path>          -> echoes category name to stdout
#   sl_limit <category>           -> echoes hard-fail limit to stdout
#   sl_warn_at <category>         -> echoes warn-at threshold to stdout
#   sl_is_excluded <category>     -> exits 0 if category is excluded
#   sl_is_test_like <path>        -> exits 0 if path is a test file (informational)
#   sl_bead_id_valid <id>         -> exits 0 if bead id matches vb-<name> rule
#   sl_parse_ledger_row <row>     -> echoes parsed columns; non-zero on bad row
#
# Note on globs: bash case `*` matches `/`, so a single `*` cannot anchor
# a path segment. We use `[!/.]*` or recursively enumerate depth levels to
# enforce single-segment matching where the design depends on it.

set -euo pipefail

# bash glob `**` only matches across `/` when extglob is enabled.
if [[ "${BASH_VERSINFO[0]}" -ge 3 ]]; then
  shopt -s extglob 2>/dev/null || true
fi

# Categories in evaluation order (most specific first).
sl_categories() {
  printf '%s\n' \
    generated \
    perf \
    kani \
    verus \
    flux \
    verification \
    test_in_src \
    test_top_level \
    production
}

# Categorize a tracked, non-excluded .rs path.
#
# Pure path matching. Returns the deepest matching category. A category that
# resolves to "excluded" still has a name; the caller should consult
# sl_is_excluded to decide whether to count the file.
sl_categorize() {
  local file="$1"

  # Generated and perf are first-party code paths. They are excluded by
  # the velvet governance contract because their drift is reported
  # separately under the perf-only tracking.
  case "$file" in
    */src/generated/*|*/src/generated.rs)  printf 'generated\n'; return 0 ;;
    */src/perf/*|*/src/perf.rs)            printf 'perf\n';      return 0 ;;
  esac

  # Kani / verus / flux artifacts under verification/, contracts/, or as
  # embedded single-file harnesses inside first-party src trees. Pattern
  # `[a-zA-Z0-9_-]*` keeps `*` non-slash. Order matters: kani/verus/flux
  # under src/verification/ must match BEFORE the generic src/verification
  # fallback below.
  case "$file" in
    verification/kani/*.rs|contracts/kani/*.rs)
      printf 'kani\n'; return 0 ;;
    verification/verus/*.rs|contracts/verus/*.rs)
      printf 'verus\n'; return 0 ;;
    verification/flux/*.rs|contracts/flux/*.rs)
      printf 'flux\n'; return 0 ;;
    */src/verification/kani/*.rs)
      printf 'kani\n'; return 0 ;;
    */src/verification/verus/*.rs)
      printf 'verus\n'; return 0 ;;
    */src/verification/flux/*.rs)
      printf 'flux\n'; return 0 ;;
    */src/verification.rs|*/src/verification/*.rs)
      printf 'verification\n'; return 0 ;;
    # Single-file harnesses that live directly under src/ but follow the
    # `kani_<topic>.rs` or `verus_<topic>.rs` naming convention used by
    # the legacy verifier harness projects.
    */src/kani_[a-zA-Z0-9_]*.rs)
      printf 'kani\n'; return 0 ;;
    */src/kani_*/*.rs)
      printf 'kani\n'; return 0 ;;
    */src/verus_[a-zA-Z0-9_]*.rs)
      printf 'verus\n'; return 0 ;;
    */src/verus_*/*.rs)
      printf 'verus\n'; return 0 ;;
  esac

  # Test files nested inside a crate's src/. Both the leading `crates/<name>/src/`
  # and `*/src/` (xtask-style) roots are accepted.
  case "$file" in
    crates/[^/]*/src/tests.rs|crates/[^/]*/src/*_tests.rs|crates/[^/]*/src/tests_*.rs|crates/[^/]*/src/*tests*.rs)
      printf 'test_in_src\n'; return 0 ;;
    */src/tests.rs|*/src/*_tests.rs|*/src/tests_*.rs|*/src/*tests*.rs)
      printf 'test_in_src\n'; return 0 ;;
    crates/[^/]*/src/*/tests.rs|crates/[^/]*/src/*/*tests*.rs|crates/[^/]*/src/*/_tests.rs|crates/[^/]*/src/*/tests_*.rs|crates/[^/]*/src/*/tests/*.rs)
      printf 'test_in_src\n'; return 0 ;;
    */src/*/tests.rs|*/src/*/*tests*.rs|*/src/*/_tests.rs|*/src/*/tests_*.rs|*/src/*/tests/*.rs)
      printf 'test_in_src\n'; return 0 ;;
    crates/[^/]*/src/*/*/tests.rs|crates/[^/]*/src/*/*/*tests*.rs|crates/[^/]*/src/*/*/_tests.rs|crates/[^/]*/src/*/*/tests_*.rs|crates/[^/]*/src/*/*/tests/*.rs)
      printf 'test_in_src\n'; return 0 ;;
    */src/*/*/tests.rs|*/src/*/*/*tests*.rs|*/src/*/*/_tests.rs|*/src/*/*/tests_*.rs|*/src/*/*/tests/*.rs)
      printf 'test_in_src\n'; return 0 ;;
    crates/[^/]*/src/*/*/*/tests.rs|crates/[^/]*/src/*/*/*/*tests*.rs|crates/[^/]*/src/*/*/*/_tests.rs|crates/[^/]*/src/*/*/*/tests_*.rs|crates/[^/]*/src/*/*/*/tests/*.rs)
      printf 'test_in_src\n'; return 0 ;;
    */src/*/*/*/tests.rs|*/src/*/*/*/*tests*.rs|*/src/*/*/*/_tests.rs|*/src/*/*/*/tests_*.rs|*/src/*/*/*/tests/*.rs)
      printf 'test_in_src\n'; return 0 ;;
  esac

  # Top-level integration tests and benches.
  case "$file" in
    crates/[^/]*/tests.rs|crates/[^/]*/tests/*.rs|crates/[^/]*/tests_*.rs|crates/[^/]*/benches/*.rs|benches/*.rs)
      printf 'test_top_level\n'; return 0 ;;
  esac

  # Everything else first-party under src/.
  case "$file" in
    crates/[^/]*/src/*.rs|crates/[^/]*/src/*/*.rs|crates/[^/]*/src/*/*/*.rs|crates/[^/]*/src/*/*/*/*.rs)
      printf 'production\n'; return 0 ;;
    */src/*.rs)
      printf 'production\n'; return 0 ;;
  esac

  # Unknown — treat as production so violations still surface.
  printf 'production\n'
}

sl_limit() {
  local category="$1"
  case "$category" in
    production)         printf '300\n' ;;
    test_in_src)        printf '1500\n' ;;
    test_top_level)     printf '3000\n' ;;
    kani)               printf '800\n' ;;
    verus)              printf '800\n' ;;
    flux)               printf '800\n' ;;
    verification)       printf '600\n' ;;
    generated|perf)     printf -- '-1\n' ;;
    *)                  printf -- '-1\n' ;;
  esac
}

sl_warn_at() {
  local category="$1"
  case "$category" in
    production)         printf '240\n' ;;
    test_in_src)        printf '1200\n' ;;
    test_top_level)     printf '2400\n' ;;
    kani)               printf '640\n' ;;
    verus)              printf '640\n' ;;
    flux)               printf '640\n' ;;
    verification)       printf '480\n' ;;
    generated|perf|*)   printf -- '-1\n' ;;
  esac
}

sl_is_excluded() {
  local category="$1"
  local lim
  lim=$(sl_limit "$category")
  [[ "$lim" -lt 0 ]]
}

# Test-like paths used by callers that need to know whether a file
# is part of a test suite even when its category limit differs.
sl_is_test_like() {
  local file="$1"
  case "$file" in
    tests.rs|tests/*.rs|tests_*.rs|*_tests.rs|_tests/*.rs|benches/*.rs)
      return 0 ;;
    *)
      return 1 ;;
  esac
}

# Strict bead ID validation. Accepts vb-<name> with optional .<part>.
sl_bead_id_valid() {
  [[ "$1" =~ ^vb-[a-z0-9]+(\.[a-z0-9]+)*$ ]]
}
