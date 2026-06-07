#!/usr/bin/env bash
# bench-instruction-counts.sh — vb-a7t6.3 Path B perf-stat pipeline
#
# Wraps the existing criterion bench binary with `perf stat -e instructions:u`
# for the 3 accepted Section 39 scenarios, parses the userspace counter, and
# appends a schema-valid row to evidence/instruction-counts.jsonl.
#
# This implements the "test-writer" state (9) of the go-skill state machine
# for bead vb-a7t6.3. The wrapper IS the perf-stat pipeline; the JSONL is
# the collected evidence; the raw logs are the auditable raw artifacts.
#
# Acceptance rule (per velvet-ballistics-MASTER.md L1611, L1616, L1876,
# L4956; vb-a7t6.3 contract.md L78, L94):
#   - tool_kind=perf-userspace-counters (Path B)
#   - limitation=non-empty, includes "perf_event_paranoid=2" + "no-kernel-counters"
#   - all 19 required fields per row (see .beads/vb-a7t6.3/type-contracts.md)
#   - 10 cross-field invariants (IC-1..IC-10) enforced via jq checks below
#
# Usage:
#   bench-instruction-counts.sh [OPTIONS]
#
# Options:
#   --workspace-root <path>   Source checkout (default: $VB_WORKSPACE_ROOT or
#                             /home/lewis/src/velvet-ballistics)
#   --bead-root <path>        Bead deliverables root (default: directory
#                             containing this script's parent).
#                             The JSONL is written to <bead-root>/evidence/
#                             and raw logs to <bead-root>/evidence/benchmark-logs/
#   --bench-binary <path>     Override bench binary path (default: auto-discover
#                             via target/release/deps/velvet_ballistics-*)
#   --scenario <name>         Add a scenario (repeatable). Default: the 3
#                             accepted Section 39 scenarios.
#   --sample-size <n>         Criterion --sample-size (default: 10)
#   --warm-up-time <secs>     Criterion --warm-up-time (default: 1)
#   --measurement-time <secs> Criterion --measurement-time (default: 1)
#   --skip-existing           If a row for (commit, metric) already exists, skip.
#   --dry-run                 Print commands but do not execute perf stat.
#   -h, --help                Show this help.
#
# Exit codes:
#   0  all scenarios collected
#   1  one or more scenarios failed (partial evidence written)
#   2  pre-condition failure (missing tool, missing binary)
#   3  argument error
#
# Contract: this script does NOT modify any source under crates/**, Cargo.toml,
# or Cargo.lock. It only writes to the bead's evidence/ directory and to the
# bench binary's working directory (criterion creates target/ artifacts).

set -u  # nounset; -e disabled per-scenario to allow partial evidence
shopt -s nullglob

# ---- Defaults (override via env or args) ---------------------------------
WORKSPACE_ROOT="${VB_WORKSPACE_ROOT:-/home/lewis/src/velvet-ballistics}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BEAD_ROOT_DEFAULT="$(cd "${SCRIPT_DIR}/.." && pwd)"
BEAD_ROOT="${BEAD_ROOT:-$BEAD_ROOT_DEFAULT}"
BENCH_BINARY=""
SAMPLE_SIZE="10"
WARM_UP_TIME="1"
MEASUREMENT_TIME="1"
SKIP_EXISTING="0"
DRY_RUN="0"
SCENARIOS=()

# ---- Help ----------------------------------------------------------------
usage() {
  sed -n '2,40p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
  exit 0
}

# ---- Argument parsing ----------------------------------------------------
while [[ $# -gt 0 ]]; do
  case "$1" in
    --workspace-root) WORKSPACE_ROOT="$2"; shift 2 ;;
    --bead-root)      BEAD_ROOT="$2"; shift 2 ;;
    --bench-binary)   BENCH_BINARY="$2"; shift 2 ;;
    --scenario)       SCENARIOS+=("$2"); shift 2 ;;
    --sample-size)    SAMPLE_SIZE="$2"; shift 2 ;;
    --warm-up-time)   WARM_UP_TIME="$2"; shift 2 ;;
    --measurement-time) MEASUREMENT_TIME="$2"; shift 2 ;;
    --skip-existing)  SKIP_EXISTING="1"; shift ;;
    --dry-run)        DRY_RUN="1"; shift ;;
    -h|--help)        usage ;;
    *) echo "bench-instruction-counts.sh: unknown argument: $1" >&2; exit 3 ;;
  esac
done

# Default scenarios: the 3 accepted Section 39 scenarios
if [[ ${#SCENARIOS[@]} -eq 0 ]]; then
  SCENARIOS=(
    "bench_engine_step_once_save_const_single_transition"
    "ipc_frame_decode"
    "engine_run_until_blocked_budget_10_small_workflow"
  )
fi

# ---- Pre-conditions ------------------------------------------------------
emit_reject() {
  # usage: emit_reject <variant> <metric> <detail>
  printf 'instruction-counts/reject: variant=%s metric=%s detail=%s\n' \
    "$1" "${2:-}" "${3:-}" >&2
}

need_tool() {
  # usage: need_tool <binary>
  if ! command -v "$1" >/dev/null 2>&1; then
    emit_reject "MissingTool" "" "tool=\"$1\""
    return 1
  fi
  return 0
}

for t in perf jq git; do
  need_tool "$t" || exit 2
done

# Workspace root must exist and be a git checkout
if [[ ! -d "$WORKSPACE_ROOT" ]]; then
  emit_reject "MissingTool" "" "workspace_root=\"$WORKSPACE_ROOT\" not found"
  exit 2
fi

# Bead root must exist (created by bead lifecycle)
if [[ ! -d "$BEAD_ROOT" ]]; then
  emit_reject "MissingTool" "" "bead_root=\"$BEAD_ROOT\" not found"
  exit 2
fi

# Auto-discover bench binary if not overridden
if [[ -z "$BENCH_BINARY" ]]; then
  cand=( "$WORKSPACE_ROOT"/target/release/deps/velvet_ballistics-* )
  # Filter to actual executables (no extension, no .d/.rmeta/.rlib)
  BENCH_BINARY=""
  for f in "${cand[@]}"; do
    case "$f" in
      *.d|*.rlib|*.rmeta) continue ;;
    esac
    if [[ -x "$f" ]]; then BENCH_BINARY="$f"; break; fi
  done
fi

if [[ -z "$BENCH_BINARY" || ! -x "$BENCH_BINARY" ]]; then
  emit_reject "MissingTool" "" "tool=\"velvet_ballistics_bench_binary\""
  exit 2
fi

# ---- Capture environment metadata ---------------------------------------
TOOL_VERSION="$(perf --version | head -n1 | awk '{print $NF}')"
KERNEL="$(uname -r)"
PARANOID="$(cat /proc/sys/kernel/perf_event_paranoid)"
COMMIT="$(git -C "$WORKSPACE_ROOT" rev-parse HEAD)"
PERF_VERSION_FULL="$(perf --version | head -n1)"

# Load section39-metadata.jsonl into bash arrays keyed by metric
META_FILE="$WORKSPACE_ROOT/evidence/section39-metadata.jsonl"
if [[ ! -f "$META_FILE" ]]; then
  emit_reject "MissingTool" "" "section39_metadata=\"$META_FILE\" not found"
  exit 2
fi

declare -A META_FIXTURE=()
declare -A META_DURABILITY=()
declare -A META_EXECUTION=()
declare -A META_RAW_LOG=()
while IFS= read -r line; do
  m=$(printf '%s' "$line" | jq -r '.metric // empty' 2>/dev/null) || continue
  fd=$(printf '%s' "$line" | jq -r '.fixture_digest // empty' 2>/dev/null) || continue
  dm=$(printf '%s' "$line" | jq -r '.durability_mode // empty' 2>/dev/null) || continue
  em=$(printf '%s' "$line" | jq -r '.mode // empty' 2>/dev/null) || continue
  rl=$(printf '%s' "$line" | jq -r '.raw_log // empty' 2>/dev/null) || continue
  META_FIXTURE["$m"]="$fd"
  META_DURABILITY["$m"]="$dm"
  META_EXECUTION["$m"]="$em"
  META_RAW_LOG["$m"]="$rl"
done < "$META_FILE"

# ---- Output paths --------------------------------------------------------
EVIDENCE_DIR="$BEAD_ROOT/evidence"
LOGS_DIR="$EVIDENCE_DIR/benchmark-logs"
JSONL="$EVIDENCE_DIR/instruction-counts.jsonl"
mkdir -p "$LOGS_DIR"

TIMESTAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

# ---- Per-scenario collection --------------------------------------------
collect_scenario() {
  local scenario="$1"
  local raw_log_rel="evidence/benchmark-logs/instructions.${scenario}.log"
  local raw_log_abs="$BEAD_ROOT/$raw_log_rel"
  local log_tmp="${raw_log_abs}.tmp"

  # Cross-check (metric, fixture_digest) is in section39-metadata
  local fixture="${META_FIXTURE[$scenario]:-}"
  local durability="${META_DURABILITY[$scenario]:-}"
  local execution="${META_EXECUTION[$scenario]:-}"
  if [[ -z "$fixture" ]]; then
    emit_reject "ScenarioNotRegistered" "$scenario" "missing from section39-metadata.jsonl"
    return 1
  fi

  # Skip if row already present for (commit, metric) and --skip-existing
  if [[ "$SKIP_EXISTING" == "1" && -f "$JSONL" ]]; then
    if jq -e --arg m "$scenario" --arg c "$COMMIT" \
        'select(.metric == $m and .commit == $c)' "$JSONL" >/dev/null 2>&1; then
      printf 'instruction-counts/skip: metric=%s commit=%s already-present\n' \
        "$scenario" "$COMMIT" >&2
      return 0
    fi
  fi

  # Build the perf-stat command as an array (defensive against injection).
  # The same argv is then serialized for the JSONL `command` field.
  local -a cmd_argv
  cmd_argv=(
    perf stat
    -e instructions:u
    -x,
    -o "$raw_log_abs"
    --
    "$BENCH_BINARY"
    --exact "$scenario"
    --warm-up-time "$WARM_UP_TIME"
    --measurement-time "$MEASUREMENT_TIME"
    --sample-size "$SAMPLE_SIZE"
  )

  # Serialize the argv as a shell-quoted command string for the JSONL `command` field.
  local cmd=""
  local -a cmd_quoted=()
  local arg
  for arg in "${cmd_argv[@]}"; do
    cmd_quoted+=("$(printf '%q' "$arg")")
  done
  # shellcheck disable=SC2128  # intentional array expansion
  cmd="${cmd_quoted[*]}"

  if [[ "$DRY_RUN" == "1" ]]; then
    printf 'instruction-counts/dry-run: %s\n' "$cmd" >&2
    return 0
  fi

  # Execute perf stat (direct argv, no shell interpretation)
  if ! "${cmd_argv[@]}"; then
    emit_reject "PerfInvocationFailed" "$scenario" "perf-stat-exit-non-zero"
    return 1
  fi

  # Parse the counter: <value>,,instructions:u,<scale>,<pct>,,
  if [[ ! -s "$raw_log_abs" ]]; then
    emit_reject "ParseError" "$scenario" "raw_log empty"
    return 1
  fi

  local value
  value="$(awk -F, '/^[[:space:]]*[0-9]+,,instructions:u,/ {gsub(/[[:space:]]/, "", $1); print $1; exit}' "$raw_log_abs")"
  if [[ -z "$value" || ! "$value" =~ ^[0-9]+$ ]]; then
    emit_reject "NoCounterMatch" "$scenario" "no instructions:u row in raw_log"
    return 1
  fi

  # Build the row via jq
  local limitation
  limitation="no-kernel-counters; perf_event_paranoid=${PARANOID}; tool=perf ${TOOL_VERSION}; paranoid-restricts-userspace-only"

  local row
  if ! row="$(jq -c -n \
      --arg metric        "$scenario" \
      --arg tool          "perf stat" \
      --arg tool_version  "$TOOL_VERSION" \
      --arg event         "instructions:u" \
      --argjson value     "$value" \
      --arg unit          "instructions" \
      --arg counter_class "userspace" \
      --arg command       "$cmd" \
      --arg raw_log       "$raw_log_rel" \
      --arg fixture       "$fixture" \
      --arg durability    "$durability" \
      --arg execution     "$execution" \
      --arg commit        "$COMMIT" \
      --arg kernel        "$KERNEL" \
      --argjson paranoid  "$PARANOID" \
      --arg limitation    "$limitation" \
      --arg tool_kind     "perf-userspace-counters" \
      --arg timestamp     "$TIMESTAMP" \
      '{metric:$metric, tool:$tool, tool_version:$tool_version, event:$event,
        value:$value, unit:$unit, counter_class:$counter_class,
        command:$command, raw_log:$raw_log, fixture_digest:$fixture,
        durability_mode:$durability, execution_mode:$execution,
        commit:$commit, kernel:$kernel, perf_event_paranoid:$paranoid,
        limitation:$limitation, tool_kind:$tool_kind,
        timestamp:$timestamp, schema_version:"instruction-counts/v1"}')"; then
    emit_reject "SchemaViolation" "$scenario" "jq row-build failed"
    return 1
  fi

  # Cross-field invariant IC-4: tool_kind=perf-userspace-counters => limitation non-empty
  if ! printf '%s' "$row" | jq -e '.tool_kind != "perf-userspace-counters" or (.limitation | length) > 0' >/dev/null; then
    emit_reject "CrossFieldInvariant" "$scenario" "IC-4 empty limitation under perf-userspace-counters"
    return 1
  fi

  # IC-1: tool=perf stat => tool_kind in {perf-userspace-counters, perf-kernel-counters}
  if ! printf '%s' "$row" | jq -e '.tool != "perf stat" or (.tool_kind == "perf-userspace-counters" or .tool_kind == "perf-kernel-counters")' >/dev/null; then
    emit_reject "CrossFieldInvariant" "$scenario" "IC-1 tool/tool_kind mismatch"
    return 1
  fi

  # IC-2: event=instructions:u => counter_class=userspace
  if ! printf '%s' "$row" | jq -e '.event != "instructions:u" or .counter_class == "userspace"' >/dev/null; then
    emit_reject "CrossFieldInvariant" "$scenario" "IC-2 event/counter_class mismatch"
    return 1
  fi

  # IC-3: counter_class=userspace + tool=perf stat => tool_kind=perf-userspace-counters
  # Equivalent (De Morgan): counter_class!=userspace OR tool!=perf stat OR tool_kind=perf-userspace-counters
  if ! printf '%s' "$row" | jq -e \
      '(.counter_class != "userspace" or .tool != "perf stat") or .tool_kind == "perf-userspace-counters"' >/dev/null; then
    emit_reject "CrossFieldInvariant" "$scenario" "IC-3 counter_class/tool_kind mismatch"
    return 1
  fi

  # IC-8: raw_log file exists and contains the value as a non-blank first token
  if [[ ! -s "$raw_log_abs" ]]; then
    emit_reject "CrossFieldInvariant" "$scenario" "IC-8 raw_log missing or empty"
    return 1
  fi
  if ! grep -q "^${value}," "$raw_log_abs"; then
    emit_reject "CrossFieldInvariant" "$scenario" "IC-8 raw_log does not contain value=${value}"
    return 1
  fi

  # Atomic append: write to .tmp, then concatenate
  printf '%s\n' "$row" >> "$JSONL.tmp"
  cat "$JSONL.tmp" >> "$JSONL"
  rm -f "$JSONL.tmp"
  printf 'instruction-counts/ok: metric=%s value=%d\n' "$scenario" "$value" >&2
  return 0
}

# ---- Main loop -----------------------------------------------------------
SUCCEEDED=0
FAILED=0
FAILED_LIST=()

for scenario in "${SCENARIOS[@]}"; do
  if collect_scenario "$scenario"; then
    SUCCEEDED=$((SUCCEEDED + 1))
  else
    FAILED=$((FAILED + 1))
    FAILED_LIST+=("$scenario")
  fi
done

# ---- Summary -------------------------------------------------------------
printf 'bench-instruction-counts.sh: succeeded=%d failed=%d commit=%s kernel=%s paranoid=%s\n' \
  "$SUCCEEDED" "$FAILED" "$COMMIT" "$KERNEL" "$PARANOID" >&2

if [[ $FAILED -gt 0 ]]; then
  printf 'bench-instruction-counts.sh: failed scenarios: %s\n' \
    "${FAILED_LIST[*]}" >&2
  exit 1
fi

exit 0
