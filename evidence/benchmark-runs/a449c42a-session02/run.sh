#!/usr/bin/env bash
# Protocol (fixed before the run, not tuned to results):
#  - 6 interleaved rounds per side; order alternates per round (A B / B A / ...), so drift affects both sides equally.
#  - Each run pinned with taskset to a CPU whose SMT siblings are all idle at that moment (mpstat snapshot logged).
#  - Metric = Criterion mean point estimate per run; baseline = mean over 6 A runs, result = mean over 6 B runs.
#  - perf stat instructions:u on A and B once each; heaptrack allocation counters on candidate once.
#  - Binaries are byte-identical (sha256 4a2675dc...); any A/B delta is harness noise.
set -eu
here="$(cd "$(dirname "$0")" && pwd)"
A=/home/lewis/src/isoloated/vb-bench-target-base/release/deps/velvet_ballistics-a06673bbb7accbb9
B=/home/lewis/src/isoloated/vb-fix-target/release/deps/velvet_ballistics-a06673bbb7accbb9
SCRROOT=/home/lewis/src/isoloated/vb-fix-logs/criterion-scratch
ARGS=(--bench --noplot --sample-size 100 --warm-up-time 3 --measurement-time 5)

pick_core() {
  for attempt in $(seq 1 30); do
    snap="$(mpstat -P ALL 1 1)"
    idle_list="$(printf '%s\n' "$snap" | awk '/Average:/ && $2 ~ /^[0-9]+$/ {print $2, $NF}')"
    for c in $(seq 0 31); do
      sibs="$(cat /sys/devices/system/cpu/cpu$c/topology/thread_siblings_list | tr ',' ' ')"
      ok=1
      for s in $sibs; do
        idle="$(printf '%s\n' "$idle_list" | awk -v s="$s" '$1==s {print $2}')"
        awk -v i="$idle" 'BEGIN{exit !(i>=99)}' || { ok=0; break; }
      done
      if [ "$ok" = 1 ]; then echo "$c"; printf '%s\n' "$snap" >&2; return 0; fi
    done
    sleep 5
  done
  echo "no idle core pair after 30 attempts" >&2; return 1
}

for m in bench_engine_step_once_save_const_single_transition engine_run_until_blocked_budget_10_small_workflow ipc_frame_decode; do
  out="$here/runs/$m"; mkdir -p "$out"
  for round in 1 2 3 4 5 6; do
    if [ $((round % 2)) -eq 1 ]; then order="A B"; else order="B A"; fi
    for side in $order; do
      bin=$([ "$side" = A ] && echo "$A" || echo "$B")
      label="$side$round"
      core="$(pick_core 2>"$out/$label-mpstat.txt")"
      dir="$SCRROOT/$label-$m"; rm -rf "$dir"
      CARGO_TARGET_DIR="$dir" taskset -c "$core" "$bin" "$m" "${ARGS[@]}" > "$out/$label.txt" 2>&1
      echo "cpu=$core" >> "$out/$label.txt"
      find "$dir" -path '*/new/sample.json' -print -quit | xargs -I{} cp {} "$out/$label-sample.json"
      find "$dir" -path '*/new/estimates.json' -print -quit | xargs -I{} cp {} "$out/$label-estimates.json"
    done
  done
  taskset -c "$(pick_core 2>/dev/null)" perf stat -x, -e instructions:u -o "$out/perf-instructions-A.csv" -- "$A" "$m" "${ARGS[@]}" > /dev/null 2>&1
  taskset -c "$(pick_core 2>/dev/null)" perf stat -x, -e instructions:u -o "$out/perf-instructions-B.csv" -- "$B" "$m" "${ARGS[@]}" > /dev/null 2>&1
  mkdir -p "$out/heaptrack"
  taskset -c "$(pick_core 2>/dev/null)" heaptrack --record-only --output "$out/heaptrack/$m.data" -- "$B" "$m" "${ARGS[@]}" > "$out/heaptrack/run.txt" 2>&1
  heaptrack_print -f "$out/heaptrack/$m.data.zst" -H "$out/heaptrack/histogram.txt" > "$out/heaptrack/summary.txt" 2>&1
done
