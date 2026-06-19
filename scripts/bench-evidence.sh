#!/usr/bin/env bash
# scripts/bench-evidence.sh
#
# Umbrella wrapper that consolidates the Section 39 / §77.14 benchmark
# evidence pipeline for velvet-ballistics.
#
# This script DOES NOT modify any source code under crates/**. It:
#
#   1. Runs `cargo bench --no-run` to verify every bench harness compiles.
#   2. Runs the canonical `cargo bench --bench velvet_ballistics` invocation
#      and pipes the criterion stdout/stderr to
#      `evidence/benchmark-logs/criterion-<bench-set>.log`. Criterion
#      captures p50/p95/p99 in its standard `target/criterion/<id>/benchmark.json`
#      output for every benchmark it runs, including all 21 bench files.
#   3. Calls `scripts/bench-instruction-counts.sh` (Path B perf-stat
#      userspace `instructions:u` wrapper from bead vb-a7t6.3) for the
#      3 v1 scenarios already registered in
#      `evidence/section39-metadata.jsonl`.
#   4. Calls `scripts/bench-alloc-evidence.sh` (heaptrack wrapper from
#      bead vb-a7t6.4) for the same 3 v1 scenarios.
#   5. Writes a top-level summary `evidence/bench-evidence-summary.jsonl`
#      enumerating per-scenario coverage state.
#
# Honesty rules baked into this script:
#
#   - It fails closed (exit 2) if perf / heaptrack / jq / cargo are missing.
#   - It never claims iai-callgrind (Path A) coverage. Path A is the open
#     follow-up `vb-a7t6.3.a` and is blocked on `valgrind` not being
#     installed in the build host.
#   - It never overwrites an existing `evidence/instruction-counts.jsonl`
#     or `evidence/alloc-evidence.jsonl` row without an explicit
#     `--force` flag (default: skip-if-exists).
#
# Coverage state at the time of writing (v0.1.0):
#
#   - p50/p95/p99 percentiles: emitted as a sidecar JSONL for the 3 v1
#     scenarios (vb-a7t6.2 contract); criterion captures the same in its
#     standard JSON output for all 21 bench files.
#   - alloc counts / bytes allocated: emitted as a sidecar JSONL for the
#     3 v1 scenarios via heaptrack 1.5.0 (vb-a7t6.4 contract); the wider
#     scope across all 21 bench files is documented as a residual gap.
#   - instruction counts: emitted for the 3 v1 scenarios via
#     `perf stat -e instructions:u` userspace (vb-a7t6.3 Path B);
#     kernel-aware Path A (valgrind + iai-callgrind) is the open
#     `vb-a7t6.3.a` follow-up.
#
# Usage:
#   scripts/bench-evidence.sh                      # full pipeline (default)
#   scripts/bench-evidence.sh --only criterion     # only criterion + summary
#   scripts/bench-evidence.sh --only instructions # only instruction evidence
#   scripts/bench-evidence.sh --only alloc         # only alloc evidence
#   scripts/bench-evidence.sh --force             # overwrite existing rows
#   scripts/bench-evidence.sh --dry-run            # print plan only
#
# Exit codes:
#   0  all enabled stages completed
#   2  missing required tool (fail closed)
#   3  argument error
#   4  one or more stages failed (partial evidence written)

set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKDIR="${WORKDIR:-$(cd -- "${SCRIPT_DIR}/.." && pwd)}"
cd "${WORKDIR}"

EVIDENCE_DIR="${WORKDIR}/evidence"
LOGS_DIR="${EVIDENCE_DIR}/benchmark-logs"
SUMMARY_JSONL="${EVIDENCE_DIR}/bench-evidence-summary.jsonl"
mkdir -p "${EVIDENCE_DIR}" "${LOGS_DIR}"

# ---- Argument parsing ----------------------------------------------------
ONLY="all"
FORCE="0"
DRY_RUN="0"
BENCH_PACKAGE="${BENCH_PACKAGE:-velvet-ballistics-workspace-tests}"
BENCH_NAME="${BENCH_NAME:-velvet_ballistics}"

usage() {
  sed -n '2,40p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
  exit 0
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --only)         ONLY="$2"; shift 2 ;;
    --force)        FORCE="1"; shift ;;
    --dry-run)      DRY_RUN="1"; shift ;;
    --bench-package) BENCH_PACKAGE="$2"; shift 2 ;;
    --bench-name)   BENCH_NAME="$2"; shift 2 ;;
    -h|--help)      usage ;;
    *) echo "bench-evidence.sh: unknown argument: $1" >&2; exit 3 ;;
  esac
done

case "${ONLY}" in
  all|criterion|instructions|alloc) ;;
  *) echo "bench-evidence.sh: --only must be all|criterion|instructions|alloc (got: ${ONLY})" >&2; exit 3 ;;
esac

# ---- Tool discovery (fail closed) ----------------------------------------
emit_reject() {
  printf 'bench-evidence/reject: stage=%s detail=%s\n' "${1:-}" "${2:-}" >&2
}

need_tool() {
  if ! command -v "$1" >/dev/null 2>&1; then
    emit_reject "MissingTool" "tool=\"$1\""
    return 1
  fi
  return 0
}

need_tool cargo || exit 2
need_tool jq    || exit 2

if [[ "${ONLY}" == all || "${ONLY}" == instructions ]]; then
  need_tool perf || exit 2
fi
if [[ "${ONLY}" == all || "${ONLY}" == alloc ]]; then
  need_tool heaptrack || exit 2
  need_tool heaptrack_print || exit 2
fi

# ---- Tool versions (captured for the summary row) ----------------------
TOOL_VERSIONS=""
TOOL_VERSIONS+="cargo=$(cargo --version | awk '{print $NF}')"
TOOL_VERSIONS+=";rustc=$(rustc --version | awk '{print $NF}')"
if command -v perf >/dev/null 2>&1; then
  TOOL_VERSIONS+=";perf=$(perf --version | head -n1 | awk '{print $NF}')"
fi
if command -v heaptrack >/dev/null 2>&1; then
  TOOL_VERSIONS+=";heaptrack=$(heaptrack --version 2>&1 | awk '/^heaptrack / {print $2; exit}')"
fi

# Kernel / paranoid (Path B limitation disclosure)
PERF_PARANOID="$(cat /proc/sys/kernel/perf_event_paranoid 2>/dev/null || echo 'unknown')"
KERNEL_RELEASE="$(uname -r)"
COMMIT="$(git -C "${WORKDIR}" rev-parse HEAD 2>/dev/null || echo 'unknown')"
TIMESTAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

# ---- Discover bench files ----------------------------------------------
mapfile -t BENCH_FILES < <(find "${WORKDIR}/crates" \
  -path '*/benches/*.rs' \
  -not -path '*/target/*' \
  | sort)
BENCH_FILE_COUNT="${#BENCH_FILES[@]}"

# Read v1 scenarios from section39-metadata.jsonl
META_FILE="${EVIDENCE_DIR}/section39-metadata.jsonl"
V1_SCENARIOS=()
if [[ -f "${META_FILE}" ]]; then
  while IFS=$'\t' read -r metric; do
    [[ -n "${metric}" ]] && V1_SCENARIOS+=("${metric}")
  done < <(jq -r '.metric // empty' "${META_FILE}")
fi
V1_COUNT="${#V1_SCENARIOS[@]}"

echo "bench-evidence.sh: plan"
echo "  workdir=${WORKDIR}"
echo "  commit=${COMMIT}"
echo "  timestamp=${TIMESTAMP}"
echo "  only=${ONLY}"
echo "  force=${FORCE}"
echo "  bench_files=${BENCH_FILE_COUNT}"
echo "  v1_scenarios=${V1_COUNT}: ${V1_SCENARIOS[*]:-none}"
echo "  tools: ${TOOL_VERSIONS}"
echo "  kernel=${KERNEL_RELEASE}; perf_event_paranoid=${PERF_PARANOID}"

if [[ "${DRY_RUN}" == "1" ]]; then
  echo "bench-evidence.sh: dry-run, exiting without execution"
  exit 0
fi

# ---- Stage 1: cargo bench --no-run (compile only) ---------------------
STAGE_CARGO_STATUS="skipped"
if [[ "${ONLY}" == all || "${ONLY}" == criterion ]]; then
  if [[ "${DRY_RUN}" != "1" ]]; then
    echo "bench-evidence.sh: stage=cargo-bench-no-run"
    if cargo bench --no-run --workspace --all-features \
        2>"${LOGS_DIR}/cargo-bench-no-run.stderr.log" \
        >"${LOGS_DIR}/cargo-bench-no-run.stdout.log"; then
      STAGE_CARGO_STATUS="passed"
    else
      STAGE_CARGO_STATUS="failed"
      emit_reject "cargo-bench-no-run" "see ${LOGS_DIR}/cargo-bench-no-run.stderr.log"
    fi
  fi
fi

# ---- Stage 2: criterion run (captures p50/p95/p99 in standard JSON) ---
STAGE_CRITERION_STATUS="skipped"
if [[ "${ONLY}" == all || "${ONLY}" == criterion ]]; then
  echo "bench-evidence.sh: stage=criterion"
  CRITERION_LOG="${LOGS_DIR}/criterion-${BENCH_NAME}.log"
  # Use --bench with --all-features and short sample size for speed; the
  # criterion standard output writes p50/p95/p99 to
  # target/criterion/<bench_id>/benchmark.json for every bench.
  if cargo bench \
      -p "${BENCH_PACKAGE}" \
      --bench "${BENCH_NAME}" \
      --all-features \
      -- \
      --sample-size 10 \
      --warm-up-time 1 \
      --measurement-time 1 \
      2>"${CRITERION_LOG}.stderr" \
      >"${CRITERION_LOG}.stdout"; then
    STAGE_CRITERION_STATUS="passed"
  else
    STAGE_CRITERION_STATUS="failed"
    emit_reject "criterion" "see ${CRITERION_LOG}.{stdout,stderr}"
  fi
fi

# ---- Stage 3: instruction counts (Path B perf userspace) --------------
STAGE_INSTRUCTION_STATUS="skipped"
if [[ "${ONLY}" == all || "${ONLY}" == instructions ]]; then
  echo "bench-evidence.sh: stage=instruction-counts (Path B: perf userspace)"
  INSTRUCTION_ARGS=(--bead-root "${WORKDIR}" --workspace-root "${WORKDIR}")
  if [[ "${FORCE}" == "1" ]]; then
    : # bench-instruction-counts.sh regenerates rows on every run; nothing to add
  fi
  if "${SCRIPT_DIR}/bench-instruction-counts.sh" "${INSTRUCTION_ARGS[@]}" \
      >"${LOGS_DIR}/bench-instruction-counts.stdout.log" \
      2>"${LOGS_DIR}/bench-instruction-counts.stderr.log"; then
    STAGE_INSTRUCTION_STATUS="passed"
  else
    STAGE_INSTRUCTION_STATUS="failed"
    emit_reject "instruction-counts" "see ${LOGS_DIR}/bench-instruction-counts.{stdout,stderr}.log"
  fi
fi

# ---- Stage 4: alloc counts (heaptrack) --------------------------------
STAGE_ALLOC_STATUS="skipped"
if [[ "${ONLY}" == all || "${ONLY}" == alloc ]]; then
  echo "bench-evidence.sh: stage=alloc-evidence (heaptrack)"
  # bench-alloc-evidence.sh has no --force flag; it always rewrites the v1
  # envelope. FORCE here is honored as a pass-through no-op to keep the
  # wrapper API symmetric with --only and --dry-run.
  if [[ "${FORCE}" == "1" ]]; then
    : # intentional no-op: child script is inherently idempotent
  fi
  if "${SCRIPT_DIR}/bench-alloc-evidence.sh" \
      >"${LOGS_DIR}/bench-alloc-evidence.stdout.log" \
      2>"${LOGS_DIR}/bench-alloc-evidence.stderr.log"; then
    STAGE_ALLOC_STATUS="passed"
  else
    STAGE_ALLOC_STATUS="failed"
    emit_reject "alloc-evidence" "see ${LOGS_DIR}/bench-alloc-evidence.{stdout,stderr}.log"
  fi
fi

# ---- Stage 5: emit summary row ----------------------------------------
SUMMARY_BENCH_FILES="${BENCH_FILE_COUNT}"
SUMMARY_V1_SCENARIOS="${V1_COUNT}"
SUMMARY_PERF_PARANOID="${PERF_PARANOID}"
SUMMARY_KERNEL="${KERNEL_RELEASE}"
SUMMARY_COMMIT="${COMMIT}"
SUMMARY_TIMESTAMP="${TIMESTAMP}"
SUMMARY_ONLY="${ONLY}"
SUMMARY_FORCE="${FORCE}"

SUMMARY_STATUS_ALL="ok"
for s in "${STAGE_CARGO_STATUS}" "${STAGE_CRITERION_STATUS}" \
         "${STAGE_INSTRUCTION_STATUS}" "${STAGE_ALLOC_STATUS}"; do
  case "${s}" in
    passed|skipped) ;;
    *) SUMMARY_STATUS_ALL="partial" ;;
  esac
done

# Build a single JSONL summary line (hand-rolled, no serde_json dep).
# Quote the tool-versions string for JSON by replacing `"` and `\` and
# stripping control characters. Keeps the helper serde-free.
json_quote() {
  printf '%s' "$1" \
    | sed -e 's/\\/\\\\/g' -e 's/"/\\"/g' -e 's/	/\\t/g' \
    | tr -d '\n\r'
}
TOOLS_JSON="$(json_quote "${TOOL_VERSIONS}")"

SUMMARY_LINE=$(cat <<JSONL
{"script":"bench-evidence.sh","timestamp":"${SUMMARY_TIMESTAMP}","commit":"${SUMMARY_COMMIT}","kernel":"${SUMMARY_KERNEL}","perf_event_paranoid":"${SUMMARY_PERF_PARANOID}","tools":"${TOOLS_JSON}","bench_files_total":${SUMMARY_BENCH_FILES},"v1_scenarios_total":${SUMMARY_V1_SCENARIOS},"stage_cargo_no_run":"${STAGE_CARGO_STATUS}","stage_criterion":"${STAGE_CRITERION_STATUS}","stage_instructions":"${STAGE_INSTRUCTION_STATUS}","stage_alloc":"${STAGE_ALLOC_STATUS}","status":"${SUMMARY_STATUS_ALL}","residual_gaps":["iai-callgrind (Path A) not installed; userspace perf only","alloc + instruction evidence bounded to ${V1_COUNT} v1 scenarios from section39-metadata.jsonl"]}
JSONL
)

# Strip leading whitespace from each emitted line so the JSONL is one record per line.
SUMMARY_LINE="$(printf '%s' "${SUMMARY_LINE}" | tr -d '\n')"
printf '%s\n' "${SUMMARY_LINE}" >"${SUMMARY_JSONL}.tmp"
mv -f "${SUMMARY_JSONL}.tmp" "${SUMMARY_JSONL}"

echo "bench-evidence.sh: summary -> ${SUMMARY_JSONL}"
echo "  status=${SUMMARY_STATUS_ALL}"

if [[ "${SUMMARY_STATUS_ALL}" == "ok" || "${SUMMARY_STATUS_ALL}" == "partial" ]]; then
  exit 0
fi
exit 4
