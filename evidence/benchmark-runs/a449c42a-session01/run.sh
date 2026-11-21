#!/usr/bin/env bash
# Session runner for commit a449c42a. Usage: run.sh (runs from evidence dir).
# Per metric: interleaved pinned timing runs A1 B1 B2 A2 (criterion mean + per-sample file kept),
# then perf instructions:u and heaptrack allocation counters on the candidate binary.
set -eu
here="$(cd "$(dirname "$0")" && pwd)"
A=/home/lewis/src/isoloated/vb-bench-target-base/release/deps/velvet_ballistics-a06673bbb7accbb9
B=/home/lewis/src/isoloated/vb-fix-target/release/deps/velvet_ballistics-a06673bbb7accbb9
SCRROOT=/home/lewis/src/isoloated/vb-fix-logs/criterion-scratch
ARGS=(--bench --noplot --sample-size 100 --warm-up-time 3 --measurement-time 5)
for m in bench_engine_step_once_save_const_single_transition engine_run_until_blocked_budget_10_small_workflow ipc_frame_decode; do
  out="$here/runs/$m"; mkdir -p "$out"
  for run in A1:$A B1:$B B2:$B A2:$A; do
    label="${run%%:*}"; bin="${run#*:}"
    dir="$SCRROOT/$label-$m"; rm -rf "$dir"
    CARGO_TARGET_DIR="$dir" taskset -c 14 "$bin" "$m" "${ARGS[@]}" > "$out/$label.txt" 2>&1
    find "$dir" -path '*/new/sample.json' -print -quit | xargs -I{} cp {} "$out/$label-sample.json"
    find "$dir" -path '*/new/estimates.json' -print -quit | xargs -I{} cp {} "$out/$label-estimates.json"
  done
  taskset -c 14 perf stat -x, -e instructions:u -o "$out/perf-instructions.csv" -- "$B" "$m" "${ARGS[@]}" > "$out/perf-run.txt" 2>&1
  mkdir -p "$out/heaptrack"
  taskset -c 14 heaptrack --record-only --output "$out/heaptrack/$m.data" -- "$B" "$m" "${ARGS[@]}" > "$out/heaptrack/run.txt" 2>&1
  heaptrack_print --print-peaks=0 --print-allocators=0 --print-temporary=0 "$(ls "$out"/heaptrack/"$m".data* | head -1)" > "$out/heaptrack/summary.txt" 2>&1
done
