#!/usr/bin/env bash
# Fail-closed instruction-count benchmark runner.
#
# This script records hardware instruction counts for selected Criterion bench
# scenarios with `perf stat`. It is governance plumbing only; its output is not
# benchmark evidence unless a reviewer records workload, hardware, variance, and
# acceptance thresholds separately.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$SCRIPT_DIR/.."
BENCH_PACKAGE="velvet-ballistics-workspace-tests"
TARGET_DIR="$ROOT_DIR/target/bench-instruction-counts"
EVIDENCE_DIR="$ROOT_DIR/target/bench-instruction-counts/evidence"

usage() {
  printf 'usage: bash scripts/bench-instruction-counts.sh [bench-name ...]\n' >&2
}

if [ "${1:-}" = "--help" ]; then
  usage
  exit 0
fi

if ! command -v perf >/dev/null 2>&1; then
  printf 'Missing required instruction counter: perf\n' >&2
  exit 127
fi

mkdir -p "$EVIDENCE_DIR"

if [ "$#" -eq 0 ]; then
  benches=(ir_traversal action_dispatch timer_wheel_tick)
else
  benches=("$@")
fi

for bench in "${benches[@]}"; do
  if [ -z "$bench" ]; then
    printf 'Empty benchmark name is not allowed.\n' >&2
    exit 2
  fi

  log_file="$EVIDENCE_DIR/$bench.perf.log"
  printf '[bench-instruction-counts] running %s\n' "$bench" >&2
  if ! CARGO_TARGET_DIR="$TARGET_DIR" rustup run nightly-2026-04-28 cargo bench --quiet -p "$BENCH_PACKAGE" --bench "$bench" --all-features --no-run; then
    printf '[bench-instruction-counts] compile failed for %s\n' "$bench" >&2
    exit 1
  fi
  if ! CARGO_TARGET_DIR="$TARGET_DIR" perf stat -x, -e instructions -o "$log_file" -- rustup run nightly-2026-04-28 cargo bench --quiet -p "$BENCH_PACKAGE" --bench "$bench" --all-features -- --bench; then
    printf '[bench-instruction-counts] perf stat failed for %s\n' "$bench" >&2
    exit 1
  fi
  if [ ! -s "$log_file" ]; then
    printf 'Instruction-count log is empty: %s\n' "$log_file" >&2
    exit 1
  fi
done
